using System.Runtime.Versioning;
using MyRigConfig.Core.LocalControl;

namespace MyRigConfig.Core.Host;

public sealed partial record PortCheck
{
    /// <summary>Checks <paramref name="input"/> (NaN when empty) against the allowed range and the
    /// current listeners.</summary>
    [SupportedOSPlatform("windows10.0")]
    public static PortCheck Of(double input, int currentPort) =>
        Of(input, currentPort, HostNetwork.FindListener, HostNetwork.ProcessName);

    internal static PortCheck Of(double input, int currentPort, Func<int, int?> findListener,
        Func<int, string?> processName)
    {
        if (double.IsNaN(input) || input != Math.Floor(input) || !LocalControlProtocol.IsSettablePort((int)input))
            return new(PortCheckResult.OutOfRange, 0);
        var port = (int)input;
        if (port == currentPort) return new(PortCheckResult.Unchanged, port);
        return findListener(port) is { } owner
            ? new(PortCheckResult.InUse, port, processName(owner))
            : new(PortCheckResult.Available, port);
    }
}
