using System.Runtime.Versioning;

namespace MyRigConfig.Core.Host;

/// <summary>Normal SCM start and stop of the installed service. The installer gives My Rig Managers
/// start and stop rights.</summary>
[SupportedOSPlatform("windows10.0")]
public static class HostServiceControlService
{
    private static Task pending = Task.CompletedTask;

    /// <summary>Completes once the start or restart in progress has ended, however it ended.
    /// Closing the window waits on it.</summary>
    public static Task Pending => Volatile.Read(ref pending);

    internal static Task Track(Task action)
    {
        Volatile.Write(ref pending, action.ContinueWith(static _ => { }, CancellationToken.None,
            TaskContinuationOptions.ExecuteSynchronously, TaskScheduler.Default));
        return action;
    }

    public static Task StartAsync(CancellationToken cancellationToken) =>
        RunAsync(HostServiceNative.Start, cancellationToken);

    public static Task RestartAsync(CancellationToken cancellationToken) =>
        RunAsync(HostServiceNative.Restart, cancellationToken);

    private static Task RunAsync(Action action, CancellationToken cancellationToken) =>
        Track(Task.Run(() =>
        {
            cancellationToken.ThrowIfCancellationRequested();
            action();
        }, cancellationToken));
}
