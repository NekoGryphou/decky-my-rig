using MyRigConfig.Core.Host;
using MyRigConfig.Core.Shared;
using CommunityToolkit.Mvvm.Input;
using MyRigConfig.Features.Host;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class HostPresentationTests
{
    private static readonly AsyncRelayCommand<Guid> NoCommand = new(_ => Task.CompletedTask);

    [TestMethod]
    public void EqualDeviceNamesHaveDistinctStableRemoveIdentifiers()
    {
        var first = new ManagedDeviceSummary(new PairedDeviceSummary(TestHost.DeviceId, "Steam Deck"), NoCommand, NoCommand);
        var second = new ManagedDeviceSummary(new PairedDeviceSummary(Guid.Parse("22222222-2222-4222-8222-222222222222"), "Steam Deck"), NoCommand, NoCommand);
        Assert.AreEqual($"RemoveDeviceButton-{TestHost.DeviceIdText}", first.RemoveAutomationId);
        Assert.AreNotEqual(first.RemoveAutomationId, second.RemoveAutomationId);
        var renamed = new ManagedDeviceSummary(new PairedDeviceSummary(first.DeviceId, "Living room device"), NoCommand, NoCommand);
        Assert.AreEqual(first.RemoveAutomationId, renamed.RemoveAutomationId);
        Assert.AreEqual("Remove Living room device", renamed.RemoveLabel);
    }

    [TestMethod]
    public void RowEditAvailabilityNotifiesOnlyWhenItChanges()
    {
        var row = new ManagedDeviceSummary(new(TestHost.DeviceId, "Bedroom device"), NoCommand, NoCommand);
        var changes = 0;
        row.PropertyChanged += (_, args) => { Assert.AreEqual(nameof(row.CanEdit), args.PropertyName); changes++; };
        Assert.IsFalse(row.CanEdit);
        row.CanEdit = true;
        row.CanEdit = true;
        row.CanEdit = false;
        Assert.AreEqual(2, changes);
    }

    [TestMethod]
    public void PortAccessibilityTracksTheValueAndConflict()
    {
        var model = new HostViewModel();
        var announcements = new List<string>();
        model.PropertyChanged += (_, args) =>
        {
            if (args.PropertyName == nameof(HostViewModel.PortAccessibleName))
                announcements.Add(model.PortAccessibleName);
        };
        model.CurrentPort = 48000;
        model.PortConflict = new PortConflict(48000, "OtherApp");
        model.PortConflict = null;
        model.CurrentPort = 49000;
        CollectionAssert.AreEqual(new[] { "Port 48000", "Port 48000 is used by OtherApp", "Port 48000", "Port 49000" }, announcements);
    }
}
