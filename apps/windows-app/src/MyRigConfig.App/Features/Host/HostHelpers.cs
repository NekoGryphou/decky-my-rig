using MyRigConfig.Core.Host;
using MyRigConfig.Shared;

namespace MyRigConfig.Features.Host;

internal static class HostHelpers
{
    /// <summary>The one notice the page shows: the most important current problem, or
    /// none.</summary>
    internal static HostNotice? SelectNotice(ServiceState state, PortConflict? conflict, bool listenerUnavailable,
        bool controlUnauthorized, bool controlUnavailable, bool testModeOn) => state switch
    {
        ServiceState.NotInstalled => new(HostMessages.MissingTitle, HostMessages.MissingMessage, MessageTone.Error),
        _ when conflict is not null => new(HostMessages.PortInUse(conflict.Port, conflict.ProcessName), "", MessageTone.Warning),
        _ when listenerUnavailable => new(HostMessages.ListenerUnavailable, "", MessageTone.Error),
        ServiceState.Stopped => new(HostMessages.StoppedTitle, "", MessageTone.Warning),
        ServiceState.Unavailable => new(HostMessages.UnavailableTitle, "", MessageTone.Error),
        ServiceState.Running when controlUnauthorized => new(HostMessages.SignOutTitle, "", MessageTone.Warning),
        ServiceState.Running when controlUnavailable => new(HostMessages.ControlTitle, "", MessageTone.Error),
        _ when testModeOn => new(HostMessages.TestModeTitle, HostMessages.TestModeMessage, MessageTone.Informational),
        _ => null,
    };
}
