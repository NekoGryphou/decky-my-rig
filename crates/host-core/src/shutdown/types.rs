use super::errors::PowerError;
use std::{sync::Mutex, time::Instant};
use uuid::Uuid;

/// Implementations should return quickly after asking for the shutdown.
pub trait Power: Send + Sync {
    /// Asks the OS for a graceful shutdown after `SHUTDOWN_GRACE`.
    fn schedule_shutdown(&self) -> Result<(), PowerError>;
    /// Withdraws a scheduled shutdown the host no longer tracks, so the OS
    /// can't power off after the host has reported that the PC stayed up.
    fn cancel_shutdown(&self) -> Result<(), PowerError>;
}

pub(super) struct Pending {
    pub(super) id: Uuid,
    pub(super) accepted_at: Instant,
    /// The power call is still running; same-id repeats can't be told it worked yet.
    pub(super) scheduling: bool,
}

pub struct Core {
    pub(super) power: Box<dyn Power>,
    pub(super) pending: Mutex<Option<Pending>>,
}

/// Drops the pending marker for `id` unless disarmed. Used for failed and
/// panicking power calls.
pub(super) struct ClearOnDrop<'a> {
    pub(super) pending: &'a Mutex<Option<Pending>>,
    pub(super) id: Uuid,
    pub(super) armed: bool,
}

impl Drop for ClearOnDrop<'_> {
    fn drop(&mut self) {
        if !self.armed {
            return;
        }
        let mut pending = self
            .pending
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if pending.as_ref().is_some_and(|p| p.id == self.id) {
            *pending = None;
        }
    }
}
