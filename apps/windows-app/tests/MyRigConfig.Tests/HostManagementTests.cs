using MyRigConfig.Core.LocalControl;
using MyRigConfig.Core.Management;
using MyRigConfig.Core.Shared;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class HostManagementTests
{
    [TestMethod]
    public void ConsentCannotCrossHostsEvenWhenDeviceIdAndNameMatch()
    {
        var state = new HostManagementService();
        var first = Guid.NewGuid();
        var second = Guid.NewGuid();
        var device = Guid.NewGuid();
        state.Publish(new LocalControlListReply(first, [new(device, "Device")]));
        Assert.IsTrue(state.MatchesHost(first));
        Assert.IsTrue(state.MatchesDevice(first, device, "Device"));
        state.Publish(new LocalControlListReply(second, [new(device, "Device")]));
        Assert.AreEqual(second, state.RequireHost());
        Assert.IsFalse(state.MatchesHost(first));
        Assert.IsFalse(state.MatchesDevice(first, device, "Device"));
        state.Unavailable();
        Assert.IsFalse(state.MatchesHost(first));
    }

    [TestMethod]
    public void RemovalConsentRequiresExactDeviceIdentityAndName()
    {
        var state = new HostManagementService();
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
        var state = new HostManagementService();
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
        var state = new HostManagementService();
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
    public void PairingCodeExpiresFromRequestStartNotReply()
    {
        var clock = new FakeClock();
        var started = clock.GetTimestamp();
        clock.Advance(TimeSpan.FromSeconds(30));
        using var lease = new PairingCodeLeaseService("12345678", started, clock);
        Assert.AreEqual(TimeSpan.FromSeconds(90), lease.Remaining);
        Assert.AreEqual("12345678", lease.Code);
        clock.Advance(TimeSpan.FromSeconds(89));
        Assert.AreEqual("12345678", lease.Code);
        clock.Advance(TimeSpan.FromSeconds(1));
        Assert.IsNull(lease.Code);
        Assert.AreEqual(TimeSpan.Zero, lease.Remaining);
        Assert.IsFalse(lease.ToString().Contains("12345678"));
    }

    [TestMethod]
    public void DisposalAndAlreadyExpiredReplyDoNotExposeCode()
    {
        var clock = new FakeClock();
        using var lease = new PairingCodeLeaseService("87654321", clock.GetTimestamp(), clock);
        lease.Dispose();
        Assert.IsNull(lease.Code);
        clock.Advance(TimeSpan.FromSeconds(121));
        using var expired = new PairingCodeLeaseService("87654321", 0, clock);
        Assert.IsNull(expired.Code);
    }

    [TestMethod]
    public void PairingLeaseRejectsNonEightDigitCodes()
    {
        foreach (var code in new[] { "1234567", "123456789", "1234567a", "１２３４５６７８" })
            Assert.ThrowsException<ArgumentException>(() => new PairingCodeLeaseService(code, 0, TimeProvider.System));
    }
}
