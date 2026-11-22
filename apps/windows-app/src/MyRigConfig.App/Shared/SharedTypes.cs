namespace MyRigConfig.Shared;

/// <summary>How important a message is. The view maps it to an InfoBar severity.</summary>
public enum MessageTone { Informational, Success, Warning, Error }

/// <summary>Why a pairing code couldn't be opened.</summary>
public enum PairingFailure { Unreachable, HostChanged, InvalidName, CodeFailed }

/// <summary>What happened when we asked the service for a pairing code. ApprovalMayExist is true
/// when the service may still hold an approval that has to be closed later.</summary>
internal sealed record PairingOpenResult(
    MyRigConfig.Core.Management.PairingCodeLease? Lease, PairingFailure? Failure, bool ApprovalMayExist);
