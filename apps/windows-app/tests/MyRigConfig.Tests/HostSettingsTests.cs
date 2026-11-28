using Microsoft.VisualStudio.TestTools.UnitTesting;
using System.Text;
using MyRigConfig.Core.Host;
using MyRigConfig.Core.LocalControl;
using MyRigConfig.Features.Host;
using MyRigConfig.Shared;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class HostSettingsTests
{
    private const string Host = TestHost.IdText;

    /// <summary>A service that lists no device and answers settings changes with <paramref name="reply"/>.</summary>
    private static (HostViewModel Model, HostStatusStore Status) Model(Func<string, string?> reply)
    {
        var status = new HostStatusStore();
        var model = TestHost.Model(TestHost.Client(reply),
            _ => Task.FromResult(new HostObservation(ServiceState.Running, status.Current, null)));
        return (model, status);
    }

    private sealed class HostStatusStore
    {
        public HostStatus Current { get; set; } = TestHost.Status();
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
        foreach (var (savedPort, expected) in new[] { (49537, "Port changed to 49537."), (47991, HostMessages.PortNotChanged) })
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

    [TestMethod]
    public async Task APortInUseIsReportedAndNothingIsSent()
    {
        var requests = new List<string>();
        var status = TestHost.Status();
        var model = TestHost.Model(TestHost.Client(text => { requests.Add(text); return null; }),
            _ => Task.FromResult(new HostObservation(ServiceState.Running, status, null)),
            probe: new StubPortProbe(port => port == 49537 ? 42 : null, _ => "OtherApp"));
        await model.RefreshCommand.ExecuteAsync(null);
        await model.ApplyPortAsync(49537, CancellationToken.None);
        Assert.AreEqual("Port 49537 is used by OtherApp.", model.FeedbackMessage);
        Assert.IsFalse(requests.Any(r => r.Contains("\"set_port\"", StringComparison.Ordinal)));
    }

    [TestMethod]
    public async Task AnUnreadablePortTableMeansTheCheckFailedAndNothingIsSent()
    {
        var requests = new List<string>();
        var model = TestHost.Model(TestHost.Client(text => { requests.Add(text); return null; }),
            probe: new StubPortProbe(_ => throw new System.ComponentModel.Win32Exception(5)));
        await model.RefreshCommand.ExecuteAsync(null);
        await model.ApplyPortAsync(49537, CancellationToken.None);
        Assert.AreEqual(HostMessages.PortCheckFailed, model.FeedbackMessage);
        Assert.IsFalse(requests.Any(r => r.Contains("\"set_port\"", StringComparison.Ordinal)));
    }

    private sealed class PortDialogs(int? answer) : IHostDialogs
    {
        internal int Asked;
        public Task ShowPairingAsync(CancellationToken cancellationToken) => Task.CompletedTask;
        public Task<bool> ConfirmRemovalAsync(string name) => Task.FromResult(false);
        public Task<string?> AskNameAsync(string current) => Task.FromResult<string?>(null);
        public Task<int?> AskPortAsync(int current) { Asked++; return Task.FromResult(answer); }
    }

    [TestMethod]
    public async Task ChangePortCommandAsksThenSavesTheChosenPort()
    {
        var requests = new List<string>();
        var (model, status) = Model(text =>
        {
            requests.Add(text);
            return text.Contains("\"set_port\"", StringComparison.Ordinal)
                ? $"{{\"version\":1,\"ok\":true,\"host_id\":\"{Host}\",\"port\":49537}}" : null;
        });
        var dialogs = new PortDialogs(49537);
        model.Dialogs = dialogs;
        await model.RefreshCommand.ExecuteAsync(null);
        status.Current = status.Current with { Port = 49537 };
        await model.ChangePortCommand.ExecuteAsync(null);
        Assert.AreEqual(1, dialogs.Asked);
        Assert.AreEqual(1, requests.Count(r => r.Contains("\"port\":49537", StringComparison.Ordinal)));
        Assert.AreEqual("Port changed to 49537.", model.FeedbackMessage);
        Assert.AreEqual(49537, model.CurrentPort);
        Assert.IsFalse(model.IsDialogOpen);
        Assert.IsFalse(model.IsInteracting);
    }

    [TestMethod]
    public async Task ACancelledPortDialogSendsNothing()
    {
        var requests = new List<string>();
        var (model, _) = Model(text => { requests.Add(text); return null; });
        model.Dialogs = new PortDialogs(null);
        await model.RefreshCommand.ExecuteAsync(null);
        var before = requests.Count;
        await model.ChangePortCommand.ExecuteAsync(null);
        Assert.AreEqual(before, requests.Count);
        Assert.IsFalse(model.IsFeedbackOpen);
        Assert.IsFalse(model.IsDialogOpen);
    }

    [TestMethod]
    public async Task ARejectedTestModeChangeNotifiesSoTheSwitchShowsTheServicesValue()
    {
        var (model, _) = Model(text => text.Contains("\"set_test_mode\"", StringComparison.Ordinal)
            ? "{\"version\":1,\"ok\":false,\"error\":\"unavailable\"}" : null);
        await model.RefreshCommand.ExecuteAsync(null);
        var notified = new List<string?>();
        model.PropertyChanged += (_, args) => notified.Add(args.PropertyName);
        await model.ChangeTestModeCommand.ExecuteAsync(true);
        Assert.IsFalse(model.TestModeOn);
        Assert.IsTrue(notified.Contains(nameof(HostViewModel.TestModeOn)), "the page re-reads the switch from this notification");
        Assert.AreEqual(HostMessages.TestModeChangeFailed, model.FeedbackMessage);
    }

    [TestMethod]
    public async Task ACancelledTestModeChangeAlsoNotifiesAndIsNotAFailure()
    {
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var client = new HostControlClientService(async (request, sending, token) =>
        {
            sending();
            if (Encoding.UTF8.GetString(request).Contains("\"set_test_mode\"", StringComparison.Ordinal))
            {
                entered.SetResult();
                await Task.Delay(Timeout.InfiniteTimeSpan, token);
            }
            return TestHost.Bytes(TestHost.ListJson());
        });
        var model = TestHost.Model(client);
        await model.RefreshCommand.ExecuteAsync(null);
        var notified = new List<string?>();
        model.PropertyChanged += (_, args) => notified.Add(args.PropertyName);
        var changing = model.ChangeTestModeCommand.ExecuteAsync(true);
        await entered.Task;
        model.ChangeTestModeCommand.Cancel();
        await changing;
        Assert.IsFalse(model.TestModeOn);
        Assert.IsTrue(notified.Contains(nameof(HostViewModel.TestModeOn)));
        Assert.IsFalse(model.IsFeedbackOpen);
        Assert.IsFalse(model.IsInteracting);
    }
}
