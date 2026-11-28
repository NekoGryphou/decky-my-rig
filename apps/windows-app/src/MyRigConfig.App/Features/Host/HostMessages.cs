using MyRigConfig.Shared;
namespace MyRigConfig.Features.Host;

/// <summary>User-facing text for the host page.</summary>
internal static class HostMessages
{
    public const string PairedDevicesHeader = "Paired devices";
    public const string PairDeviceAction = "Pair a device";
    public const string ServiceHeader = "Service";
    public const string StatusLabel = "Status";
    public const string StartAction = "Start";
    public const string RestartAction = "Restart";
    public const string PortLabel = "Port";
    public const string TestModeLabel = "Test mode";
    public const string On = "On";
    public const string Off = "Off";

    public const string StatusChecking = "Checking…";
    public const string StatusRunning = "Running";
    public const string StatusStopped = "Stopped";
    public const string StatusStarting = "Starting…";
    public const string StatusStopping = "Stopping…";
    public const string StatusNotInstalled = "Missing";
    public const string StatusUnavailable = "Unavailable";
    public const string NoPairedDevice = "No paired device";

    public const string MissingTitle = "The service is missing";
    public const string MissingMessage = "Reinstall My Rig.";
    public const string StoppedTitle = "Service stopped";
    public const string UnavailableTitle = "Service status unavailable";
    public const string ControlTitle = SharedMessages.ServiceUnreachable;
    public const string SignOutTitle = "Ask an administrator to add you to My Rig Managers, then sign out and back in";
    public const string ListenerUnavailable = "Can't open the PC connection port";
    public const string TestModeTitle = "Test mode";
    public const string PairFailed = "The device couldn't be paired.";
    public const string PortNotChanged = "The port couldn't be changed.";
    public const string TestModeChangeFailed = "Test mode couldn't be changed.";

    public const string StartFailed = "The service didn't start.";
    public const string RestartFailed = "The service didn't restart.";
    public const string ListChanged = "The list changed. Try again.";
    public const string RemovalUncertain = "The removal couldn't be confirmed.";
    public const string NoLongerPaired = "Already removed.";

    public const string RemoveAction = "Remove";
    public const string RenameAction = "Rename";
    public const string RenameUncertain = "The new name couldn't be confirmed.";
    public const string MoreOptions = "More options";

    public static string RemoveLabel(string name) => $"Remove {name}";
    public static string PortName(string port) => port.Length == 0 ? PortLabel : $"Port {port}";
    public static string PortInUse(int port, string? program) => SharedMessages.PortInUse(port, program);
    public const string PortCheckFailed = "Port availability couldn't be checked. The port was not changed.";
    public static string PortChanged(int port) => $"Port changed to {port}.";
    public static string Removed(string name) => $"“{name}” removed.";
    public static string Renamed(string name) => $"Renamed to “{name}”.";
    public static string RenameLabel(string name) => $"Rename {name}";
    public static string MoreOptionsLabel(string name) => $"More options for {name}";
    public static string RemoveDialogTitle(string name) => $"Remove “{name}”?";
}
