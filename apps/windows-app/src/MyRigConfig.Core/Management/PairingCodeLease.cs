using MyRigConfig.Core.Shared;

namespace MyRigConfig.Core.Management;

// ! Count the code's lifetime from before the request is sent, not from the reply.
internal sealed class PairingCodeLease : IDisposable
{
    private string? code;
    private readonly long started;
    private readonly TimeProvider clock;
    internal PairingCodeLease(string code, long started, TimeProvider clock)
    {
        if (code.Length != 8 || code.Any(c => c is < '0' or > '9'))
            throw new ArgumentException("Invalid pairing code.", nameof(code));
        this.code = code;
        this.started = started;
        this.clock = clock;
    }
    internal string? Code => Remaining > TimeSpan.Zero ? code : null;
    internal TimeSpan Remaining
    {
        get
        {
            var remaining = PairingProtocolConstants.Lifetime - clock.GetElapsedTime(started);
            return code is null || remaining <= TimeSpan.Zero ? TimeSpan.Zero : remaining;
        }
    }
    public void Dispose() => code = null;
    public override string ToString() => "Pairing approval";
}
