using System.Text;
using MyRigConfig.Core.Host;
using MyRigConfig.Core.LocalControl;
using MyRigConfig.Features.Host;
using MyRigConfig.Shared;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class HostServiceActionTests
{
    [TestMethod]
    public async Task ThePendingActionIsKnownUntilItEndsEvenIfItFails()
    {
        // ! Closing the window waits on this: a restart cut in half would leave the service stopped.
        var finish = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var tracked = HostServiceControlService.Track(finish.Task);
        Assert.IsFalse(HostServiceControlService.Pending.IsCompleted);
        finish.SetException(new HostServiceException(new InvalidOperationException()));
        await Assert.ThrowsExceptionAsync<HostServiceException>(() => tracked);
        await HostServiceControlService.Pending;
        Assert.IsTrue(HostServiceControlService.Pending.IsCompleted);
    }

    [TestMethod]
    [DataRow(false, "complete")]
    [DataRow(false, "cancel")]
    [DataRow(false, "fail")]
    [DataRow(true, "complete")]
    [DataRow(true, "cancel")]
    [DataRow(true, "fail")]
    public async Task CancelledServiceActionPublishesObservedFinalState(bool restart, string completion)
    {
        var state = restart ? ServiceState.Running : ServiceState.Stopped;
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var resume = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var startCalls = 0;
        var restartCalls = 0;
        var observations = new List<CancellationToken>();
        async Task NativeAction(CancellationToken token)
        {
            state = ServiceState.Stopped;
            entered.TrySetResult();
            if (completion == "cancel") await resume.Task.WaitAsync(token);
            else await resume.Task;
            if (completion == "fail") throw new HostServiceException(new InvalidOperationException("Fixture SCM failure."));
            state = ServiceState.Running;
        }
        var client = new HostControlClientService((_, sending, _) =>
        {
            sending();
            return Task.FromResult(TestHost.Bytes(TestHost.ListJson()));
        });
        Task<HostObservation> Observe(CancellationToken token)
        {
            observations.Add(token);
            return Task.FromResult(new HostObservation(state, state == ServiceState.Running
                ? new HostStatus(49000, false, "listening") : null, null));
        }
        var model = TestHost.Model(client, Observe, null,
            token => { startCalls++; return NativeAction(token); },
            token => { restartCalls++; return NativeAction(token); });
        await model.RefreshCommand.ExecuteAsync(null);
        var command = restart ? model.RestartServiceCommand : model.StartServiceCommand;
        var running = command.ExecuteAsync(null);
        await entered.Task;
        command.Cancel();
        resume.TrySetResult();
        await running;
        Assert.AreEqual(completion == "complete" ? ServiceState.Running : ServiceState.Stopped, model.ServiceState);
        Assert.IsFalse(model.IsBusy);
        Assert.IsFalse(model.IsFeedbackOpen);
        Assert.AreEqual(restart ? 0 : 1, startCalls);
        Assert.AreEqual(restart ? 1 : 0, restartCalls);
        Assert.AreEqual(2, observations.Count);
        Assert.IsFalse(observations.Last().IsCancellationRequested);
    }

    [TestMethod]
    public async Task ARetiredPageIsNotRefreshedAfterTheServiceActionEnds()
    {
        var entered = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var resume = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var observations = 0;
        using var page = new CancellationTokenSource();
        var model = TestHost.Model(TestHost.Client(_ => null), _ =>
        {
            observations++;
            return Task.FromResult(new HostObservation(ServiceState.Stopped, null, null));
        }, null, async _ => { entered.SetResult(); await resume.Task; });
        model.PageLifetime = page.Token;
        await model.RefreshCommand.ExecuteAsync(null);
        var running = model.StartServiceCommand.ExecuteAsync(null);
        await entered.Task;
        page.Cancel();
        resume.SetResult();
        await running;
        Assert.AreEqual(1, observations, "only the refresh before the action ran");
        Assert.IsFalse(model.IsBusy);
    }

    [TestMethod]
    [DataRow(false)]
    [DataRow(true)]
    public async Task AFailedStartOrRestartTellsTheUser(bool restart)
    {
        var state = restart ? ServiceState.Running : ServiceState.Stopped;
        var model = TestHost.Model(TestHost.Client(_ => null),
            _ => Task.FromResult(new HostObservation(state, state == ServiceState.Running ? TestHost.Status() : null, null)),
            null,
            _ => throw new HostServiceException(new InvalidOperationException("Fixture SCM failure.")),
            _ => throw new HostServiceException(new InvalidOperationException("Fixture SCM failure.")));
        await model.RefreshCommand.ExecuteAsync(null);
        await (restart ? model.RestartServiceCommand : model.StartServiceCommand).ExecuteAsync(null);
        Assert.IsTrue(model.IsFeedbackOpen);
        Assert.AreEqual(restart ? HostMessages.RestartFailed : HostMessages.StartFailed, model.FeedbackMessage);
        Assert.AreEqual(MessageTone.Error, model.FeedbackSeverity);
        Assert.IsFalse(model.IsBusy);
    }
}
