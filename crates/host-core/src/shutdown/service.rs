use super::{
    consts::PENDING_TIMEOUT,
    errors::ShutdownError,
    types::{Core, Pending, Power},
};
use std::{sync::Mutex, time::Instant};
use uuid::Uuid;

impl Core {
    pub fn new(power: Box<dyn Power>) -> Self {
        Self {
            power,
            pending: Mutex::new(None),
        }
    }

    fn expire(pending: &mut Option<Pending>, now: Instant) {
        if pending
            .as_ref()
            .is_some_and(|p| now.saturating_duration_since(p.accepted_at) >= PENDING_TIMEOUT)
        {
            *pending = None;
        }
    }

    pub fn shutting_down(&self, now: Instant) -> Result<bool, ShutdownError> {
        let mut pending = self.pending.lock().map_err(|_| ShutdownError::Internal)?;
        Self::expire(&mut pending, now);
        Ok(pending.is_some())
    }

    pub fn shutdown(&self, id: Uuid, now: Instant) -> Result<(), ShutdownError> {
        let mut pending = self.pending.lock().map_err(|_| ShutdownError::Internal)?;
        Self::expire(&mut pending, now);
        if let Some(active) = pending.as_ref() {
            return if active.id == id {
                Ok(())
            } else {
                Err(ShutdownError::InProgress)
            };
        }
        self.power
            .schedule_shutdown()
            .map_err(|_| ShutdownError::Failed)?;
        *pending = Some(Pending {
            id,
            accepted_at: now,
        });
        Ok(())
    }
}
