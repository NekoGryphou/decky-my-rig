using System.Text;

namespace MyRigConfig.Core.LocalControl;

internal static class LocalControlConstants
{
    internal const int RequestLimit = 1024;
    internal const int ResponseLimit = 8192;
    internal const int FramePrefixBytes = sizeof(uint);
    internal const int JsonMaxDepth = 4;
    // ! Same as the host's MAX_PAIRED_DEVICES (crates/host-core/src/paired_devices/consts.rs).
    internal const int MaxPairedDevices = 16;
    internal const uint PipeWaitMilliseconds = 2000;
    internal const string ListenerListening = "listening";
    internal const string ListenerUnavailableState = "unavailable";
    internal const string UnauthorizedError = "unauthorized";
    internal static readonly TimeSpan ExchangeDeadline = TimeSpan.FromSeconds(10);
    internal const string PipePath = @"\\.\pipe\MyRigHost.Control.v1";
    internal static readonly UTF8Encoding StrictUtf8 = new(false, true);
    internal static readonly HashSet<string> Errors = new(StringComparer.Ordinal)
    {
        "unauthorized", "invalid_request", "wrong_host", "storage_failed", "storage_conflict",
        "recovery_required", "unavailable", "control_failed"
    };

    /// <summary>Code point ranges a name can't contain besides control characters: all Cf format
    /// characters (Unicode 16), the line and paragraph separators, and the blank Hangul fillers.
    /// Must match <c>INVISIBLE</c> in crates/protocol; the shared names.json fixture checks both.</summary>
    internal static readonly (int First, int Last)[] InvisibleRanges =
    [
        (0x00AD, 0x00AD),
        (0x0600, 0x0605),
        (0x061C, 0x061C),
        (0x06DD, 0x06DD),
        (0x070F, 0x070F),
        (0x0890, 0x0891),
        (0x08E2, 0x08E2),
        (0x115F, 0x1160),
        (0x180E, 0x180E),
        (0x200B, 0x200F),
        (0x2028, 0x2029),
        (0x202A, 0x202E),
        (0x2060, 0x206F),
        (0x3164, 0x3164),
        (0xFEFF, 0xFEFF),
        (0xFFA0, 0xFFA0),
        (0xFFF9, 0xFFFB),
        (0x110BD, 0x110BD),
        (0x110CD, 0x110CD),
        (0x13430, 0x1343F),
        (0x1BCA0, 0x1BCA3),
        (0x1D173, 0x1D17A),
        (0xE0001, 0xE0001),
        (0xE0020, 0xE007F),
    ];

    internal const int MaxNameLength = 80;
    internal const int MinSettablePort = 1024;
    internal const int MaxPort = 65535;
}
