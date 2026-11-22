using MyRigConfig.Shared;
namespace MyRigConfig.Features.Port;

/// <summary>User-facing text for the port dialog.</summary>
internal static class PortMessages
{
    public const string PortLabel = "Port";
    public const string Title = "Change port";
    public const string Save = "Save";
    public const string OutOfRange = "Use a port from 1024 to 65535.";
    public const string CheckFailed = "Port availability couldn't be checked.";
    public const string Consequence = "Paired devices will need the new port.";

    public static string InUse(int port, string? program) => SharedMessages.PortInUse(port, program) + ".";
}
