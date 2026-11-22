using System.Text;
using MyRigConfig.Core.Host;
using MyRigConfig.Core.LocalControl;
using MyRigConfig.Features.Host;
using MyRigConfig.Shared;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class HostSettingsTests
{
    private const string Host = "03c85317-9d93-4fb3-a04b-91a0e16668c6";

    /// <summary>A service that lists no device and answers settings changes with <paramref name="reply"/>.</summary>
    private static (HostViewModel Model, HostStatusStore Status) Model(Func<string, string?> reply)
    {
        var status = new HostStatusStore();
        var client = new HostControlClient((request, sending, _) =>
        {
            sending();
            var text = Encoding.UTF8.GetString(request);
            var answer = reply(text)
                ?? $"{{\"version\":1,\"ok\":true,\"host_id\":\"{Host}\",\"paired_devices\":[]}}";
            return Task.FromResult(Encoding.UTF8.GetBytes(answer));
        });
        var model = new HostViewModel(client,
            _ => Task.FromResult(new HostObservation(ServiceState.Running, status.Current, null)),
            TimeProvider.System);
        return (model, status);
    }

    private sealed class HostStatusStore
    {
        public HostStatus Current { get; set; } = new(47991, false, "listening");
    }

    [TestMethod]
    public void NoticeShowsTheMostImportantCondition()
    {
        var conflict = new PortConflict(47991, "OtherApp");
        Assert.AreEqual(HostMessages.MissingTitle,
            HostHelpers.SelectNotice(ServiceState.NotInstalled, conflict, true, true, true, true)?.Title);
        Assert.AreEqual(HostMessages.PortInUse(47991, "OtherApp"),
            HostHelpers.SelectNotice(ServiceState.Stopped, conflict, true, false, false, true)?.Title);
        Assert.AreEqual(HostMessages.ListenerUnavailable,
            HostHelpers.SelectNotice(ServiceState.Running, null, true, true, true, true)?.Title);
        // - Just a title: the port setting is right below it.
        Assert.AreEqual("", HostHelpers.SelectNotice(ServiceState.Running, null, true, false, false, false)?.Message);
        Assert.AreEqual(HostMessages.StoppedTitle,
            HostHelpers.SelectNotice(ServiceState.Stopped, null, false, true, true, true)?.Title);
        Assert.AreEqual(HostMessages.SignOutTitle,
            HostHelpers.SelectNotice(ServiceState.Running, null, false, true, true, true)?.Title);
        Assert.AreEqual(HostMessages.ControlTitle,
            HostHelpers.SelectNotice(ServiceState.Running, null, false, false, true, true)?.Title);
        Assert.AreEqual(MessageTone.Informational,
            HostHelpers.SelectNotice(ServiceState.Running, null, false, false, false, true)?.Severity);
        Assert.IsNull(HostHelpers.SelectNotice(ServiceState.Running, null, false, false, false, false));
    }

    [TestMethod]
    public async Task ConfirmedTestModeShowsItsNotice()
    {
        var (model, status) = Model(text => text.Contains("\"set_test_mode\"", StringComparison.Ordinal)
            ? $"{{\"version\":1,\"ok\":true,\"host_id\":\"{Host}\",\"test_mode\":true}}"
            : null);
        await model.RefreshCommand.ExecuteAsync(null);
        status.Current = status.Current with { TestMode = true };
        await model.SetTestModeAsync(true, CancellationToken.None);
        Assert.IsTrue(model.TestModeOn);
        Assert.AreEqual(HostMessages.TestModeTitle, model.NoticeTitle);
        Assert.IsFalse(model.IsFeedbackOpen);
    }

    [TestMethod]
    public async Task RejectedTestModeKeepsTheServicesValue()
    {
        var (model, _) = Model(text => text.Contains("\"set_test_mode\"", StringComparison.Ordinal)
            ? "{\"version\":1,\"ok\":false,\"error\":\"unavailable\"}"
            : null);
        await model.RefreshCommand.ExecuteAsync(null);
        await model.SetTestModeAsync(true, CancellationToken.None);
        Assert.IsFalse(model.TestModeOn);
        Assert.AreEqual(HostMessages.TestModeChangeFailed, model.FeedbackMessage);
    }

    [TestMethod]
    public async Task SavedPortIsShownAndUnconfirmedPortSaysSo()
    {
        // - The port dialog tells the user that devices need the new port.
        Assert.AreEqual("Port changed to 49537.", HostMessages.PortChanged(49537));
        foreach (var (savedPort, expected) in new[] { (49537, HostMessages.PortChanged(49537)), (47991, HostMessages.PortNotChanged) })
        {
            var (model, status) = Model(text => text.Contains("\"set_port\"", StringComparison.Ordinal)
                ? $"{{\"version\":1,\"ok\":true,\"host_id\":\"{Host}\",\"port\":{savedPort}}}"
                : null);
            await model.RefreshCommand.ExecuteAsync(null);
            status.Current = status.Current with { Port = savedPort };
            await model.ApplyPortAsync(49537, CancellationToken.None);
            Assert.AreEqual(expected, model.FeedbackMessage);
            Assert.AreEqual(savedPort, model.CurrentPort);
        }
    }
}
