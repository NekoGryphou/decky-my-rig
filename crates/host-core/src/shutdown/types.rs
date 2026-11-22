use super::errors::PowerError;
use std::{sync::Mutex, time::Instant};
use uuid::Uuid;

/// Implementations should return quickly after asking for the shutdown.
pub trait Power: Send + Sync {
    /// Asks the OS for a graceful shutdown after `SHUTDOWN_GRACE`.
    fn schedule_shutdown(&self) -> Result<(), PowerError>;
}

pub(super) struct Pending {
    pub(super) id: Uuid,
    pub(super) accepted_at: Instant,
}

pub struct Core {
    pub(super) power: Box<dyn Power>,
    pub(super) pending: Mutex<Option<Pending>>,
}
