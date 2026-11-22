using System.ComponentModel;
using System.Runtime.Versioning;
using System.ServiceProcess;

namespace MyRigConfig.Core.Host;

/// <summary>Normal SCM start and stop of the installed service. The installer gives My Rig Managers
/// start and stop rights.</summary>
[SupportedOSPlatform("windows")]
public static class HostServiceControl
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
        RunAsync(service => Start(service), cancellationToken);

    public static Task RestartAsync(CancellationToken cancellationToken) =>
        RunAsync(service =>
        {
            if (service.Status != ServiceControllerStatus.Stopped)
            {
                service.Stop();
                service.WaitForStatus(ServiceControllerStatus.Stopped, HostConstants.ServiceStateDeadline);
            }
            Start(service);
        }, cancellationToken);

    private static void Start(ServiceController service)
    {
        if (service.Status == ServiceControllerStatus.Running) return;
        service.Start();
        service.WaitForStatus(ServiceControllerStatus.Running, HostConstants.ServiceStateDeadline);
    }

    private static Task RunAsync(Action<ServiceController> action, CancellationToken cancellationToken) =>
        Track(Task.Run(() =>
        {
            cancellationToken.ThrowIfCancellationRequested();
            try
            {
                using var service = new ServiceController(HostConstants.ServiceName, ".");
                action(service);
            }
            catch (Exception exception) when (exception is InvalidOperationException or Win32Exception
                or System.ServiceProcess.TimeoutException)
            {
                throw new HostServiceException(exception);
            }
        }, cancellationToken));
}
