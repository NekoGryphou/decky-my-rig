using System.Text;
using MyRigConfig.Core.Host;
using MyRigConfig.Core.LocalControl;
using MyRigConfig.Features.Host;

namespace MyRigConfig.Tests;

[TestClass]
public sealed class HostServiceActionTests
{
    [TestMethod]
    public async Task ThePendingActionIsKnownUntilItEndsEvenIfItFails()
    {
        // ! Closing the window waits on this: a restart cut in half would leave the service stopped.
        var finish = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var tracked = HostServiceControl.Track(finish.Task);
        Assert.IsFalse(HostServiceControl.Pending.IsCompleted);
        finish.SetException(new HostServiceException(new InvalidOperationException()));
        await Assert.ThrowsExceptionAsync<HostServiceException>(() => tracked);
        await HostServiceControl.Pending;
        Assert.IsTrue(HostServiceControl.Pending.IsCompleted);
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
        var client = new HostControlClient((_, sending, _) =>
        {
            sending();
            return Task.FromResult(Encoding.UTF8.GetBytes("{\"version\":1,\"ok\":true,\"host_id\":\"03c85317-9d93-4fb3-a04b-91a0e16668c6\",\"paired_devices\":[]}"));
        });
        Task<HostObservation> Observe(CancellationToken token)
        {
            observations.Add(token);
            return Task.FromResult(new HostObservation(state, state == ServiceState.Running
                ? new HostStatus(49000, false, "listening") : null, null));
        }
        var model = new HostViewModel(client, Observe, TimeProvider.System,
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
}
