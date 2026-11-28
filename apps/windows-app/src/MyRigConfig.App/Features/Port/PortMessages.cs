using MyRigConfig.Core.LocalControl;

namespace MyRigConfig.Features.Port;

/// <summary>User-facing text for the port dialog.</summary>
internal static class PortMessages
{
    public const string PortLabel = "Port";
    public const string Title = "Change port";
    public const string Save = "Save";
    public static string OutOfRange =>
        $"Use a port from {LocalControlConstants.MinSettablePort} to {LocalControlConstants.MaxPort}.";
    public const string CheckFailed = "Port availability couldn't be checked.";
}
