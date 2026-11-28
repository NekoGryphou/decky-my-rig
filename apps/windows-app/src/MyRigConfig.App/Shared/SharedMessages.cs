using MyRigConfig.Core.LocalControl;

namespace MyRigConfig.Shared;

/// <summary>User-facing text used by more than one feature.</summary>
internal static class SharedMessages
{
    public const string Cancel = "Cancel";
    public static string InvalidDeviceName => $"Use 1 to {LocalControlConstants.MaxNameLength} visible characters.";
    public const string ServiceUnreachable = "Can't reach the service";

    public static string PortInUse(int port, string? program) =>
        $"Port {port} is used by {program ?? "another program"}";

    public static string PortInUseSentence(int port, string? program) => PortInUse(port, program) + ".";
}
