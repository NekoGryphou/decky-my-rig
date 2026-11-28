#[derive(Debug, PartialEq, Eq)]
pub enum RegistryError {
    PersistenceFailed,
    Unavailable,
    PersistenceConflict,
    RecoveryRequired,
}

/// Only Unchanged guarantees that a failed save left the stored state alone.
#[derive(Debug, PartialEq, Eq)]
pub enum PersistError {
    Unchanged,
    Conflict,
    RecoveryRequired,
}

#[derive(Debug, PartialEq, Eq)]
pub enum OpenPairingError {
    InvalidInput,
    RegistryRejected,
    Busy,
    RecoveryRequired,
    Unavailable,
}

#[derive(Debug, PartialEq, Eq)]
pub enum PairRequestError {
    Invalid,
    /// A code is already showing, or the last request was too recent.
    Busy,
    Unavailable,
}

/// Why a pairing step was refused. Busy is temporary: another request is
/// holding the code for a moment, so the device can ask again.
#[derive(Debug, PartialEq, Eq)]
pub enum PairingRefusal {
    Busy,
    Unavailable,
    /// The registry has no room for another device. Not a secret, and the
    /// code stays usable once the owner removes a device.
    DeviceLimit,
}
