using MyRigConfig.Core.LocalControl;

namespace MyRigConfig.Core.Host;

public sealed partial record PortCheck
{
    /// <summary>Checks <paramref name="input"/> (NaN when empty) against the allowed range and current listeners.</summary>
    internal static PortCheck Of(double input, int currentPort, IPortProbe probe)
    {
        if (double.IsNaN(input) || input != Math.Floor(input) || !LocalControlProtocolService.IsSettablePort((int)input))
            return new(PortCheckResult.OutOfRange, 0);
        var port = (int)input;
        if (port == currentPort) return new(PortCheckResult.Unchanged, port);
        return probe.FindListener(port, excludedProcessId: null) is { } owner
            ? new(PortCheckResult.InUse, port, probe.ProcessName(owner))
            : new(PortCheckResult.Available, port);
    }
}
