using MyRigConfig.Core.Host;

namespace MyRigConfig.Core.LocalControl;

internal static class ControlServerIdentityConstants
{
    internal const string ServiceName = HostConstants.ServiceName;
    internal const uint ServiceManagerConnect = 1;
    internal const uint ServiceQueryStatus = 4;
}
