namespace MyRigConfig.Core.LocalControl;

internal sealed class HostControlException(bool uncertain, bool unauthorized = false) : Exception(
    uncertain ? "Host action could not be confirmed." : "Host control is unavailable.")
{
    internal bool OutcomeUncertain { get; } = uncertain;
    /// <summary>The pipe refused this user: they're not (yet) in My Rig Managers.</summary>
    internal bool Unauthorized { get; } = unauthorized;
}
