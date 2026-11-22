using MyRigConfig.Core.LocalControl;
using MyRigConfig.Core.Management;
using MyRigConfig.Core.Shared;
using CommunityToolkit.Mvvm.Input;
using MyRigConfig.Features.Host;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class HostManagementTests
{
    private static readonly AsyncRelayCommand<Guid> NoCommand = new(_ => Task.CompletedTask);

    [TestMethod]
    public void ConsentCannotCrossHostsEvenWhenDeviceIdAndNameMatch()
    {
        var state = new HostManagement();
        var first = Guid.NewGuid();
        var second = Guid.NewGuid();
        var device = Guid.NewGuid();
        state.Publish(new LocalControlListReply(first, [new(device, "Device")]));
        Assert.IsTrue(state.MatchesHost(first));
        Assert.IsTrue(state.MatchesDevice(first, device, "Device"));
        state.Publish(new LocalControlListReply(second, [new(device, "Device")]));
        Assert.IsFalse(state.MatchesHost(first));
        Assert.IsFalse(state.MatchesDevice(first, device, "Device"));
        state.Unavailable();
        Assert.IsFalse(state.MatchesHost(first));
    }

    [TestMethod]
    public void RemovalConsentRequiresExactDeviceIdentityAndName()
    {
        var state = new HostManagement();
        var host = Guid.NewGuid();
        var device = Guid.NewGuid();
        state.Publish(new LocalControlListReply(host, [new(device, "Renamed")]));
        Assert.IsFalse(state.MatchesDevice(host, device, "Device"));
        Assert.IsFalse(state.MatchesDevice(host, Guid.NewGuid(), "Renamed"));
        Assert.IsFalse(state.MatchesDevice(host, device, "renamed"));
        Assert.IsTrue(state.MatchesDevice(host, device, "Renamed"));
        state.Publish(new LocalControlListReply(host, []));
        Assert.IsFalse(state.MatchesDevice(host, device, "Renamed"));
    }

    [TestMethod]
    public void OneOwnedInteractionBlocksRefreshPairAndRemoveUntilJoined()
    {
        var state = new HostManagement();
        Assert.IsTrue(state.TryBegin());
        Assert.IsFalse(state.TryBegin());
        state.Unavailable();
        Assert.IsFalse(state.TryBegin());
        state.End();
        Assert.IsTrue(state.TryBegin());
        state.End();
    }

    [TestMethod]
    public void UnavailableIsNotAnAuthenticatedEmptyList()
    {
        var state = new HostManagement();
        Assert.IsFalse(state.Available);
        Assert.ThrowsException<InvalidOperationException>(() => state.RequireHost());
        var host = Guid.NewGuid();
        state.Publish(new LocalControlListReply(host, []));
        Assert.IsTrue(state.Available);
        Assert.AreEqual(host, state.RequireHost());
        Assert.AreEqual(0, state.Devices.Count);
        state.Unavailable();
        Assert.IsFalse(state.Available);
        Assert.ThrowsException<InvalidOperationException>(() => state.RequireHost());
    }

    [TestMethod]
    public void LatestVerifiedListReplacesRoutingAndRemovalAuthority()
    {
        var state = new HostManagement();
        var first = Guid.NewGuid();
        var second = Guid.NewGuid();
        var device = Guid.NewGuid();
        state.Publish(new LocalControlListReply(first, [new(device, "Device")]));
        Assert.IsTrue(state.MatchesDevice(first, device, "Device"));
        state.Publish(new LocalControlListReply(second, []));
        Assert.AreEqual(second, state.RequireHost());
        Assert.IsFalse(state.MatchesDevice(second, device, "Device"));
        state.Unavailable();
        Assert.IsFalse(state.MatchesDevice(second, device, "Device"));
    }

    [TestMethod]
    public void PairingCodeExpiresFromRequestStartNotReply()
    {
        var clock = new Clock();
        var started = clock.GetTimestamp();
        clock.Advance(30);
        using var lease = new PairingCodeLease("12345678", started, clock);
        Assert.AreEqual(TimeSpan.FromSeconds(90), lease.Remaining);
        Assert.AreEqual("12345678", lease.Code);
        clock.Advance(89);
        Assert.AreEqual("12345678", lease.Code);
        clock.Advance(1);
        Assert.IsNull(lease.Code);
        Assert.AreEqual(TimeSpan.Zero, lease.Remaining);
        Assert.IsFalse(lease.ToString().Contains("12345678"));
    }

    [TestMethod]
    public void DisposalAndAlreadyExpiredReplyDoNotExposeCode()
    {
        var clock = new Clock();
        using var lease = new PairingCodeLease("87654321", clock.GetTimestamp(), clock);
        lease.Dispose();
        Assert.IsNull(lease.Code);
        clock.Advance(121);
        using var expired = new PairingCodeLease("87654321", 0, clock);
        Assert.IsNull(expired.Code);
    }

    [TestMethod]
    public void PairingLeaseRejectsNonEightDigitCodes()
    {
        foreach (var code in new[] { "1234567", "123456789", "1234567a", "１２３４５６７８" })
            Assert.ThrowsException<ArgumentException>(() => new PairingCodeLease(code, 0, TimeProvider.System));
    }

    [TestMethod]
    public void RemoveAvailabilityNotifiesAndNamesTheDevice()
    {
        var row = new ManagedDeviceSummary(new(Guid.NewGuid(), "Bedroom device"), NoCommand, NoCommand);
        var changes = 0;
        row.PropertyChanged += (_, args) => { Assert.AreEqual(nameof(row.CanEdit), args.PropertyName); changes++; };
        Assert.IsFalse(row.CanEdit);
        row.CanEdit = true;
        row.CanEdit = true;
        row.CanEdit = false;
        Assert.AreEqual(2, changes);
        Assert.AreEqual("Remove Bedroom device", row.RemoveLabel);
    }

    private sealed class Clock : TimeProvider
    {
        private long timestamp;
        public override long TimestampFrequency => 1;
        public override long GetTimestamp() => timestamp;
        internal void Advance(long seconds) => timestamp += seconds;
    }
}
