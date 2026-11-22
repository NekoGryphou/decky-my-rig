use super::errors::PersistError;
use crate::{paired_devices::RegistryConfig, shutdown::Core};
use myrig_pairing_crypto::session::{ApprovedSession, Challenge};
use std::{
    sync::{Arc, Mutex, RwLock, atomic::AtomicBool},
    time::Instant,
};

pub struct Host {
    pub(crate) computer_name: String,
    pub(crate) host_id: uuid::Uuid,
    pub(crate) config: RwLock<RegistryConfig>,
    pub(crate) registry_mutation: Mutex<()>,
    pub(crate) core: Core,
    pub(crate) events: crate::events::HostEvents,
    /// Every pairing change happens under this lock. Crypto and saves run
    /// outside it, and `busy` marks the claimed session.
    pub(crate) pairing: Mutex<PairingState>,
    pub(crate) pairing_finished: tokio::sync::Notify,
    pub(crate) storage_healthy: AtomicBool,
    pub(crate) listener_available: AtomicBool,
    pub(crate) port_change: tokio::sync::watch::Sender<u16>,
    pub(crate) pair_requested_at: Mutex<Option<Instant>>,
    /// Saves changes a network request makes on its own behalf.
    pub(crate) persist: PairingPersistence,
}

/// Only for trusted persistence. A failed save has to say whether the disk
/// changed.
pub type PairingPersistence = Arc<dyn Fn(&[u8]) -> Result<(), PersistError> + Send + Sync>;

pub(crate) struct RegistryPairing {
    pub(crate) device_id: uuid::Uuid,
    pub(crate) name: String,
    pub(crate) digest: [u8; 32],
    pub(crate) persist: PairingPersistence,
}
pub(crate) struct SecurePairing {
    pub(crate) crypto: ApprovedSession,
    pub(crate) leaf_digest: [u8; 32],
    pub(crate) nonce: [u8; 32],
    pub(crate) token: zeroize::Zeroizing<String>,
    pub(crate) registry: RegistryPairing,
}
pub struct SecureChallenge {
    pub nonce: [u8; 32],
    pub challenge: Challenge,
}
#[derive(Default)]
pub(crate) struct PairingState {
    /// Goes up on every close, which invalidates approvals sent before it.
    pub(crate) generation: u64,
    /// The opened, unclaimed session, if any.
    pub(crate) session: Option<SecurePairing>,
    /// Closed locally; a session being prepared mustn't be published.
    pub(crate) closed: bool,
    /// The service is stopping: no session can be opened ever again.
    pub(crate) stopped: bool,
    /// A session is being prepared, or a request or save has claimed it.
    pub(crate) busy: bool,
    /// Failed guesses since the last successful pairing, across all codes.
    pub(crate) failed_starts: u32,
}
pub(crate) struct PairingReservation(pub(crate) Arc<Host>);

/// Only built inside the host, never from a network or local JSON caller.
pub struct PairingApproval(pub(crate) u64);
