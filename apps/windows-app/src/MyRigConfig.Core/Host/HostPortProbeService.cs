using System.Runtime.Versioning;

namespace MyRigConfig.Core.Host;

[SupportedOSPlatform("windows10.0")]
internal sealed class HostPortProbeService : IPortProbe
{
    public int? FindListener(int port, int? excludedProcessId) => HostNetworkNative.FindListener(port, excludedProcessId);

    public string? ProcessName(int processId) => HostNetworkNative.ProcessName(processId);
}
