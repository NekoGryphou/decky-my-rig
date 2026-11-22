namespace MyRigConfig.Core.LocalControl;

internal sealed record LocalControlPairingReply(Guid HostId, string Code) : LocalControlHostReply(HostId)
{
    // ! The generated text would include the code.
    public override string ToString() => "Local pairing approval";
}
