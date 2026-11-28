use super::super::{errors::PairRequestError, types::Host, utils::session_pending};
use super::utils::request_interval;
use myrig_protocol::{events::EventKind, validation::valid_name};
use std::time::Instant;

impl Host {
    pub fn request_pairing(&self, device_name: &str, now: Instant) -> Result<(), PairRequestError> {
        if !valid_name(device_name) {
            return Err(PairRequestError::Invalid);
        }
        if !self.storage_healthy() {
            return Err(PairRequestError::Unavailable);
        }
        let mut last = self
            .pair_requested_at
            .lock()
            .map_err(|_| PairRequestError::Unavailable)?;
        {
            let mut state = self
                .pairing
                .lock()
                .map_err(|_| PairRequestError::Unavailable)?;
            // ! Failed guesses slow down the next request, so cycling through codes can't brute-
            // ! force a PC nobody's watching.
            let interval = request_interval(state.failed_starts);
            if last.is_some_and(|at| now.saturating_duration_since(at) < interval) {
                return Err(PairRequestError::Busy);
            }
            // ! A code nobody has tried yet can be replaced (its notification was dismissed). A
            // ! code someone is still guessing at stays until it expires, so asking again never
            // ! buys more guesses. A code whose guesses are all spent can be replaced.
            let tried = session_pending(&mut state, now)
                && state.session.as_ref().is_some_and(|session| {
                    !session.crypto.untouched() && !session.crypto.exhausted()
                });
            if state.busy || tried {
                return Err(PairRequestError::Busy);
            }
        }
        *last = Some(now);
        self.events
            .record(EventKind::PairingRequest, device_name.to_owned());
        Ok(())
    }
}
