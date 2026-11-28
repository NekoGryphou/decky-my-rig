use super::{
    consts::PENDING_TIMEOUT,
    errors::ShutdownError,
    types::{ClearOnDrop, Core, Pending, Power},
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
        let expired = pending
            .as_ref()
            .is_some_and(|p| now.saturating_duration_since(p.accepted_at) >= PENDING_TIMEOUT);
        if expired {
            // ! The request is no longer tracked, so its OS timer must go too.
            *pending = None;
            drop(pending);
            // * A failed cancel is reported by the power implementation's event log entry; the HTTP
            // * answer is the same either way.
            let _ = self.power.cancel_shutdown();
            return Ok(false);
        }
        Ok(pending.is_some())
    }

    pub fn shutdown(&self, id: Uuid, now: Instant) -> Result<(), ShutdownError> {
        let mut pending = self.pending.lock().map_err(|_| ShutdownError::Internal)?;
        Self::expire(&mut pending, now);
        if let Some(active) = pending.as_ref() {
            return if active.id == id && !active.scheduling {
                Ok(())
            } else {
                Err(ShutdownError::InProgress)
            };
        }
        // ! The OS call runs outside the lock so /status never waits on it. The marker keeps
        // ! concurrent requests from scheduling a second one.
        *pending = Some(Pending {
            id,
            accepted_at: now,
            scheduling: true,
        });
        drop(pending);
        // ! A panicking power call must not leave the marker set, or every later request would see
        // ! InProgress until the deadline.
        let mut clear = ClearOnDrop {
            pending: &self.pending,
            id,
            armed: true,
        };
        let scheduled = self.power.schedule_shutdown();
        let mut pending = self.pending.lock().map_err(|_| ShutdownError::Internal)?;
        match scheduled {
            Ok(()) => {
                if let Some(active) = pending.as_mut() {
                    active.scheduling = false;
                }
                clear.armed = false;
                Ok(())
            }
            Err(error) => Err(ShutdownError::Failed(error)),
        }
    }
}
