using System.ComponentModel;
using System.Runtime.Versioning;
using MyRigConfig.Core.LocalControl;

namespace MyRigConfig.Core.Host;

[SupportedOSPlatform("windows10.0")]
public static class HostDiagnosticsService
{
    public static async Task<HostObservation> ReadAsync(CancellationToken cancellationToken)
        => await ReadAsync(
            token => Task.Run(() => HostServiceNative.ReadState(token), token),
            token => new HostControlClientService().StatusAsync(token),
            new HostPortProbeService(), cancellationToken).ConfigureAwait(false);

    internal static async Task<HostObservation> ReadAsync(
        Func<CancellationToken, Task<(ServiceState State, int? ProcessId)>> readService,
        Func<CancellationToken, Task<LocalControlReply>> readControl,
        IPortProbe probe, CancellationToken cancellationToken)
    {
        var (state, processId) = await readService(cancellationToken).ConfigureAwait(false);
        LocalControlStatusReply? status = null;
        if (state == ServiceState.Running)
        {
            try { status = await readControl(cancellationToken).ConfigureAwait(false) as LocalControlStatusReply; }
            catch (HostControlException) when (!cancellationToken.IsCancellationRequested) { }
        }
        var conflict = status is { } parsed
            ? await Task.Run(() => FindConflict(probe, parsed.Port, processId), cancellationToken).ConfigureAwait(false)
            : null;
        return new(state, status is null ? null : new HostStatus(status.Port, status.TestMode, status.ListenerState), conflict);
    }

    /// <summary>Another process listening on <paramref name="port"/>, or null if it's free or
    /// ours.</summary>
    internal static PortConflict? FindConflict(IPortProbe probe, int port, int? serviceProcessId)
    {
        int? listener;
        // ? Only a diagnostic: if the table can't be read we just show no conflict, rather than a failed status.
        try { listener = probe.FindListener(port, serviceProcessId); }
        catch (Win32Exception) { return null; }
        return listener is { } owner
            ? new PortConflict(port, probe.ProcessName(owner))
            : null;
    }
}
