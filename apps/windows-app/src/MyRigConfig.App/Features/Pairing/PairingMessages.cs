using MyRigConfig.Core.Shared;
using MyRigConfig.Shared;
namespace MyRigConfig.Features.Pairing;

/// <summary>User-facing text for the pairing dialog.</summary>
internal static class PairingMessages
{
    public const string DeviceNameLabel = "Device name";
    public const string CodeLabel = "Pairing code";
    public const string CodeTimeLabel = "Time left for this code";
    public const string DefaultDeviceName = "Steam Deck";
    public const string TitleName = "Pair a device";
    public const string TitleCode = "Pairing code";
    public const string TitleExpired = "Code expired";
    public const string TitleFailed = "Pairing failed";
    public const string CreateCode = "Create code";
    public const string NewCode = "New code";
    public const string Close = "Close";
    public static string CancelFailed =>
        $"The code could not be cancelled. It will expire within {PairingProtocolConstants.LifetimeSeconds / 60} minutes.";

    public const string HostChanged = "The service changed. Try again.";
    public static string InvalidName => SharedMessages.InvalidDeviceName;
    public const string CodeFailed = "Couldn't create a code.";
    public const string Unreachable = SharedMessages.ServiceUnreachable;

    public static string Failure(PairingFailure failure) => failure switch
    {
        PairingFailure.Unreachable => Unreachable,
        PairingFailure.HostChanged => HostChanged,
        PairingFailure.InvalidName => InvalidName,
        _ => CodeFailed,
    };
    public static string Paired(string name) => $"“{name}” paired.";
    public static string ExpiresIn(TimeSpan remaining) => $"Expires in {remaining:m\\:ss}";
    // ? Spaced-out digits, so screen readers read the code one digit at a time.
    public static string CodeName(string code) =>
        $"{CodeLabel} {string.Join(' ', code.Where(char.IsAsciiDigit))}";
    public static string FormatCode(string code) => $"{code[..4]} {code[4..]}";
}
