using System.ComponentModel;
using System.Runtime.Versioning;
using System.ServiceProcess;
using MyRigConfig.Core.LocalControl;

namespace MyRigConfig.Core.Host;

[SupportedOSPlatform("windows10.0")]
public static class HostDiagnostics
{
    public static async Task<HostObservation> ReadAsync(CancellationToken cancellationToken)
        => await ReadAsync(
            token => Task.Run(() => ReadService(token), token),
            token => new HostControlClient().StatusAsync(token),
            cancellationToken).ConfigureAwait(false);

    internal static async Task<HostObservation> ReadAsync(
        Func<CancellationToken, Task<(ServiceState State, int? ProcessId)>> readService,
        Func<CancellationToken, Task<LocalControlReply>> readControl,
        CancellationToken cancellationToken)
    {
        var (state, processId) = await readService(cancellationToken).ConfigureAwait(false);
        LocalControlStatusReply? status = null;
        if (state == ServiceState.Running)
        {
            try { status = await readControl(cancellationToken).ConfigureAwait(false) as LocalControlStatusReply; }
            catch (HostControlException) when (!cancellationToken.IsCancellationRequested) { }
        }
        var conflict = status is { } parsed
            ? await Task.Run(() => FindConflict(parsed.Port, processId), cancellationToken).ConfigureAwait(false)
            : null;
        return new(state, status is null ? null : new HostStatus(status.Port, status.TestMode, status.ListenerState), conflict);
    }

    /// <summary>Another process listening on <paramref name="port"/>, or null if it's free or
    /// ours.</summary>
    public static PortConflict? FindConflict(int port, int? serviceProcessId)
    {
        int? listener;
        // ? Only a diagnostic: if the table can't be read we just show no conflict, rather than a
        //   failed status.
        try { listener = HostNetwork.FindListener(port, serviceProcessId); }
        catch (Win32Exception) { return null; }
        return listener is { } owner
            ? new PortConflict(port, HostNetwork.ProcessName(owner))
            : null;
    }

    private static (ServiceState State, int? ProcessId) ReadService(CancellationToken cancellationToken)
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
            return (state, state == ServiceState.Running ? ControlServerIdentity.RunningProcessId() : null);
        }
        catch (InvalidOperationException exception) when
            (exception.InnerException is Win32Exception { NativeErrorCode: HostConstants.ServiceDoesNotExist })
        { return (ServiceState.NotInstalled, null); }
        catch (InvalidOperationException) { return (ServiceState.Unavailable, null); }
        catch (Win32Exception) { return (ServiceState.Unavailable, null); }
    }
}
