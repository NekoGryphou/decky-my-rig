using System.Runtime.InteropServices;

namespace MyRigConfig.Core.LocalControl;

[StructLayout(LayoutKind.Sequential)]
internal struct ControlServiceStatus
{
    public uint ServiceType, CurrentState, ControlsAccepted, Win32ExitCode, ServiceSpecificExitCode,
        CheckPoint, WaitHint, ProcessId, ServiceFlags;
}
