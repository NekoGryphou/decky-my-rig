namespace MyRigConfig.Core.Shared;

internal static class PairingProtocolConstants
{
    internal const int LifetimeSeconds = 120;
    internal static readonly TimeSpan Lifetime = TimeSpan.FromSeconds(LifetimeSeconds);
}
