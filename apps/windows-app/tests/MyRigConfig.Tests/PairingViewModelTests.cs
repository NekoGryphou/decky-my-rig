using Microsoft.VisualStudio.TestTools.UnitTesting;
using MyRigConfig.Shared;
using System.Text;
using System.Text.Json;
using MyRigConfig.Core.Host;
using MyRigConfig.Core.LocalControl;
using MyRigConfig.Features.Host;
using MyRigConfig.Features.Pairing;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class PairingViewModelTests
{
    [TestMethod]
    public async Task ClosingCodeClearsItAndCancelsTheHostApprovalOnce()
    {
        var peer = new Peer();
        var model = peer.Model();
        await model.StartAsync(CancellationToken.None);
        await model.CreateCodeAsync(CancellationToken.None);
        Assert.AreEqual("0012 3456", model.CodeText);
        Assert.AreEqual("Pairing code 0 0 1 2 3 4 5 6", model.CodeAccessibleName);
        await model.CloseAsync();
        await model.CloseAsync();
        Assert.AreEqual("", model.CodeText);
        Assert.IsFalse(peer.Host.IsInteracting);
        Assert.IsFalse(model.CanCreate);
        Assert.AreEqual(1, peer.Requests.Count(r => r.GetProperty("operation").GetString() == "close_pairing"));
        var close = peer.Requests.Single(r => r.GetProperty("operation").GetString() == "close_pairing");
        CollectionAssert.AreEquivalent(new[] { "version", "operation", "host_id" }, close.EnumerateObject().Select(p => p.Name).ToArray());
        Assert.AreEqual(peer.HostId.ToString("D"), close.GetProperty("host_id").GetString());
    }

    [TestMethod]
    public async Task APairedDeviceEndsTheCodeShowsFeedbackAndLeavesNothingToClose()
    {
        var peer = new Peer();
        var model = peer.Model();
        var paired = 0;
        model.Paired += (_, _) => paired++;
        await model.StartAsync(CancellationToken.None);
        await model.CreateCodeAsync(CancellationToken.None);
        peer.Devices.Add((TestHost.DeviceIdText, PairingMessages.DefaultDeviceName));
        for (var tick = 0; tick < PairingConstants.PollEveryTicks; tick++) await model.TickAsync(CancellationToken.None);
        Assert.AreEqual(1, paired);
        Assert.AreEqual("", model.CodeText);
        Assert.AreEqual(PairingMessages.Paired(PairingMessages.DefaultDeviceName), peer.Host.FeedbackMessage);
        Assert.AreEqual(MessageTone.Success, peer.Host.FeedbackSeverity);
        await model.CloseAsync();
        await peer.Host.RetireAsync();
        Assert.IsFalse(peer.Requests.Any(r => r.GetProperty("operation").GetString() == "close_pairing"),
            "the host already used up the code");
    }

    [TestMethod]
    public async Task ADeviceThatWasAlreadyPairedDoesNotCompleteThePairing()
    {
        var peer = new Peer();
        peer.Devices.Add((TestHost.DeviceIdText, PairingMessages.DefaultDeviceName));
        var model = peer.Model();
        var paired = 0;
        model.Paired += (_, _) => paired++;
        await peer.Host.RefreshCommand.ExecuteAsync(null);
        await model.StartAsync(CancellationToken.None);
        await model.CreateCodeAsync(CancellationToken.None);
        for (var tick = 0; tick < PairingConstants.PollEveryTicks * 2; tick++) await model.TickAsync(CancellationToken.None);
        Assert.AreEqual(0, paired);
        Assert.AreEqual(PairingStep.Code, model.Step);
        await model.CloseAsync();
    }

    [TestMethod]
    public async Task FailedCloseClearsTheCodeAndReportsExpiryWithoutRetrying()
    {
        var peer = new Peer { FailClose = true };
        var model = peer.Model();
        await model.StartAsync(CancellationToken.None);
        await model.CreateCodeAsync(CancellationToken.None);
        await model.CloseAsync();
        Assert.AreEqual("", model.CodeText);
        Assert.IsFalse(peer.Host.IsInteracting);
        Assert.IsTrue(peer.Host.IsFeedbackOpen);
        Assert.AreEqual(MessageTone.Warning, peer.Host.FeedbackSeverity);
        Assert.AreEqual(1, peer.Requests.Count(r => r.GetProperty("operation").GetString() == "close_pairing"));
    }

    [TestMethod]
    public async Task FailedPreflightDoesNotCancelAnotherCallersCode()
    {
        var peer = new Peer();
        var model = peer.Model();
        await model.StartAsync(CancellationToken.None);
        peer.FailList = true;
        await model.CreateCodeAsync(CancellationToken.None);
        Assert.AreEqual(PairingStep.Failed, model.Step);
        await model.CloseAsync();
        Assert.IsFalse(peer.Requests.Any(r => r.GetProperty("operation").GetString() is "open_pairing" or "close_pairing"));
    }

    [TestMethod]
    public async Task DefinitiveOpenRejectionDoesNotCancelAnotherCallersPreparation()
    {
        var peer = new Peer { RejectOpen = true };
        var model = peer.Model();
        await model.StartAsync(CancellationToken.None);
        await model.CreateCodeAsync(CancellationToken.None);
        Assert.AreEqual(PairingStep.Failed, model.Step);
        await model.CloseAsync();
        Assert.AreEqual(1, peer.Requests.Count(r => r.GetProperty("operation").GetString() == "open_pairing"));
        Assert.IsFalse(peer.Requests.Any(r => r.GetProperty("operation").GetString() == "close_pairing"));
    }

    [TestMethod]
    public async Task ClosingNameStepDoesNotCancelAnUnrelatedApproval()
    {
        var peer = new Peer();
        var model = peer.Model();
        await model.StartAsync(CancellationToken.None);
        await model.CloseAsync();
        Assert.IsFalse(peer.Requests.Any(r => r.GetProperty("operation").GetString() == "close_pairing"));
        Assert.IsFalse(peer.Host.IsInteracting);
    }

    [TestMethod]
    public async Task ExpiryCancelsTheHostCodeAndAllowsAnotherAttempt()
    {
        var peer = new Peer();
        var model = peer.Model();
        await model.StartAsync(CancellationToken.None);
        await model.CreateCodeAsync(CancellationToken.None);
        peer.Clock.Advance(TimeSpan.FromSeconds(120));
        await model.TickAsync(CancellationToken.None);
        Assert.AreEqual(PairingStep.Expired, model.Step);
        Assert.AreEqual("", model.CodeText);
        Assert.AreEqual(1, peer.Requests.Count(r => r.GetProperty("operation").GetString() == "close_pairing"));
        await model.CreateCodeAsync(CancellationToken.None);
        Assert.AreEqual(PairingStep.Code, model.Step);
        await model.CloseAsync();
    }

    [TestMethod]
    public async Task NewCodeRecoversAfterInitialPreparationFailed()
    {
        var peer = new Peer { FailList = true };
        var model = peer.Model();
        await model.StartAsync(CancellationToken.None);
        Assert.AreEqual(PairingStep.Failed, model.Step);
        peer.FailList = false;
        await model.CreateCodeAsync(CancellationToken.None);
        Assert.AreEqual(PairingStep.Code, model.Step);
        await model.CloseAsync();
    }

    [TestMethod]
    public async Task HostChangeRequiresFreshConfirmationForTheNextCode()
    {
        var peer = new Peer();
        var model = peer.Model();
        await model.StartAsync(CancellationToken.None);
        peer.HostId = TestHost.DeviceId;
        await model.CreateCodeAsync(CancellationToken.None);
        Assert.AreEqual(PairingStep.Failed, model.Step);
        await model.CreateCodeAsync(CancellationToken.None);
        Assert.AreEqual(PairingStep.Code, model.Step);
        Assert.AreEqual(peer.HostId.ToString("D"), peer.Requests.Single(r => r.GetProperty("operation").GetString() == "open_pairing").GetProperty("host_id").GetString());
        await model.CloseAsync();
    }

    [TestMethod]
    public async Task ClosingDuringPreparationDrainsCancellationBeforeClosingTheHost()
    {
        var peer = new Peer();
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        peer.Open = async token =>
        {
            entered.SetResult();
            await Task.Delay(Timeout.InfiniteTimeSpan, token);
        };
        var model = peer.Model();
        await model.StartAsync(CancellationToken.None);
        var creating = model.CreateCodeAsync(CancellationToken.None);
        await entered.Task;
        await model.CloseAsync();
        await Assert.ThrowsExceptionAsync<OperationCanceledException>(() => creating);
        Assert.AreEqual("", model.CodeText);
        Assert.IsFalse(peer.Host.IsInteracting);
        Assert.AreEqual("close_pairing", peer.Requests.Last().GetProperty("operation").GetString());
    }

    [TestMethod]
    public async Task PairingActionWaitsForQuietRefreshThenConfirmsTheHost()
    {
        var peer = new Peer();
        await peer.Host.RefreshCommand.ExecuteAsync(null);
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        peer.Observe = async token => { entered.SetResult(); await release.Task.WaitAsync(token); };
        var refresh = peer.Host.RefreshQuietlyAsync(CancellationToken.None);
        await entered.Task;
        var interaction = peer.Host.BeginInteractionAsync(CancellationToken.None);
        try { Assert.IsFalse(interaction.IsCompleted, "A click must wait for the refresh, not disappear."); }
        finally { release.TrySetResult(); await refresh; }
        Assert.AreEqual(peer.HostId, await interaction);
        peer.Host.EndInteraction();
        Assert.IsFalse(peer.Host.IsInteracting);
    }

    [TestMethod]
    public async Task CancelledQueuedActionLeavesAdmissionForTheNextAction()
    {
        var peer = new Peer();
        await peer.Host.RefreshCommand.ExecuteAsync(null);
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        peer.Observe = async token => { entered.SetResult(); await release.Task.WaitAsync(token); };
        var refresh = peer.Host.RefreshQuietlyAsync(CancellationToken.None);
        await entered.Task;
        using var cancelled = new CancellationTokenSource();
        var waiting = peer.Host.BeginInteractionAsync(cancelled.Token);
        cancelled.Cancel();
        try { await Assert.ThrowsExceptionAsync<OperationCanceledException>(() => waiting); }
        finally { release.TrySetResult(); await refresh; }
        Assert.AreEqual(peer.HostId, await peer.Host.BeginInteractionAsync(CancellationToken.None));
        peer.Host.EndInteraction();
    }

    [TestMethod]
    [DataRow("refresh")]
    [DataRow("start")]
    [DataRow("restart")]
    public async Task CancelledQueuedCommandCompletesWithoutEscapingCancellation(string name)
    {
        var peer = new Peer();
        await peer.Host.RefreshCommand.ExecuteAsync(null);
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        peer.Observe = async token => { entered.SetResult(); await release.Task.WaitAsync(token); };
        var quiet = peer.Host.RefreshQuietlyAsync(CancellationToken.None);
        await entered.Task;
        var command = name switch
        {
            "refresh" => peer.Host.RefreshCommand,
            "start" => peer.Host.StartServiceCommand,
            _ => peer.Host.RestartServiceCommand,
        };
        var queued = command.ExecuteAsync(null);
        try
        {
            Assert.IsFalse(queued.IsCompleted);
            command.Cancel();
            await queued;
            Assert.IsFalse(peer.Host.IsBusy);
        }
        finally { release.TrySetResult(); await quiet; }
        Assert.AreEqual(peer.HostId, await peer.Host.BeginInteractionAsync(CancellationToken.None));
        peer.Host.EndInteraction();
    }

    [TestMethod]
    public async Task QueuedTestModeInteractionDisablesFurtherChangesBeforeAdmission()
    {
        var peer = new Peer();
        await peer.Host.RefreshCommand.ExecuteAsync(null);
        peer.Host.TestModeOn = false;
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var release = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        peer.Observe = async token => { entered.SetResult(); await release.Task.WaitAsync(token); };
        var quiet = peer.Host.RefreshQuietlyAsync(CancellationToken.None);
        await entered.Task;
        using var cancelled = new CancellationTokenSource();
        var changing = peer.Host.SetTestModeAsync(true, cancelled.Token);
        try
        {
            Assert.IsTrue(peer.Host.IsInteracting);
            Assert.IsFalse(peer.Host.CanManage, "The pending selection must disable another toggle.");
        }
        finally
        {
            cancelled.Cancel();
            await Assert.ThrowsExceptionAsync<OperationCanceledException>(() => changing);
            release.TrySetResult();
            await quiet;
        }
        Assert.IsFalse(peer.Host.IsInteracting);
    }

    private sealed class Peer
    {
        internal Guid HostId { get; set; } = TestHost.Id;
        internal bool FailList { get; set; }
        internal bool FailClose { get; set; }
        internal bool RejectOpen { get; set; }
        internal Func<CancellationToken, Task>? Open { get; set; }
        internal Func<CancellationToken, Task>? Observe { get; set; }
        internal List<(string Id, string Name)> Devices { get; } = [];
        internal List<JsonElement> Requests { get; } = [];
        internal FakeClock Clock { get; } = new();
        internal HostViewModel Host { get; }

        internal Peer()
        {
            var client = new HostControlClientService(async (request, sending, token) =>
            {
                sending();
                using var document = JsonDocument.Parse(request);
                var root = document.RootElement;
                Requests.Add(root.Clone());
                var operation = root.GetProperty("operation").GetString();
                var envelope = $"\"version\":1,\"ok\":true,\"host_id\":\"{HostId:D}\"";
                if (operation == "close_pairing" && FailClose) throw new IOException("Fixture pipe unavailable.");
                if (operation == "list" && FailList) throw new IOException("Fixture pipe unavailable.");
                if (operation == "open_pairing" && Open is { } open) await open(token);
                if (operation == "open_pairing" && RejectOpen)
                    return Encoding.UTF8.GetBytes("{\"version\":1,\"ok\":false,\"error\":\"unavailable\"}");
                var reply = operation switch
                {
                    "list" => $"{{{envelope},\"paired_devices\":[{string.Join(',', Devices.Select(d => $"{{\"device_id\":\"{d.Id}\",\"name\":\"{d.Name}\"}}"))}]}}",
                    "open_pairing" => $"{{{envelope},\"code\":\"00123456\",\"expires_in_seconds\":120}}",
                    "close_pairing" => $"{{{envelope}}}",
                    _ => throw new InvalidOperationException("Unexpected fixture request."),
                };
                return Encoding.UTF8.GetBytes(reply);
            });
            Host = TestHost.Model(client, async token =>
            {
                if (Observe is { } observe) await observe(token);
                return new HostObservation(ServiceState.Running, TestHost.Status(49000, testMode: true), null);
            }, Clock);
        }

        internal PairingViewModel Model() => new(Host);
    }
}
