using System.ComponentModel;
using System.Runtime.Versioning;
using System.ServiceProcess;
using MyRigConfig.Core.LocalControl;

namespace MyRigConfig.Core.Host;

/// <summary>The Windows service manager calls for the installed host service.</summary>
[SupportedOSPlatform("windows10.0")]
internal static class HostServiceNative
{
    internal static (ServiceState State, int? ProcessId) ReadState(CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();
        try
        {
            using var service = new ServiceController(HostConstants.ServiceName, ".");
            var state = service.Status switch
            {
                ServiceControllerStatus.Running => ServiceState.Running,
                ServiceControllerStatus.Stopped => ServiceState.Stopped,
                ServiceControllerStatus.StartPending or ServiceControllerStatus.ContinuePending => ServiceState.Starting,
                ServiceControllerStatus.StopPending or ServiceControllerStatus.PausePending => ServiceState.Stopping,
                _ => ServiceState.Unavailable,
            };
            return (state, state == ServiceState.Running ? ControlServerIdentityNative.RunningProcessId() : null);
        }
        catch (InvalidOperationException exception) when
            (exception.InnerException is Win32Exception { NativeErrorCode: HostConstants.ServiceDoesNotExist })
        { return (ServiceState.NotInstalled, null); }
        catch (InvalidOperationException) { return (ServiceState.Unavailable, null); }
        catch (Win32Exception) { return (ServiceState.Unavailable, null); }
    }

    internal static void Start() => Run(StartCore);

    internal static void Restart() => Run(service =>
    {
        if (service.Status != ServiceControllerStatus.Stopped)
        {
            service.Stop();
            service.WaitForStatus(ServiceControllerStatus.Stopped, HostConstants.ServiceStateDeadline);
        }
        StartCore(service);
    });

    private static void StartCore(ServiceController service)
    {
        if (service.Status == ServiceControllerStatus.Running) return;
        service.Start();
        service.WaitForStatus(ServiceControllerStatus.Running, HostConstants.ServiceStateDeadline);
    }

    private static void Run(Action<ServiceController> action)
    {
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
    }
}
