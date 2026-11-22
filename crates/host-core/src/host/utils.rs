use super::types::PairingState;
use std::time::Instant;

/// Whether a session is open and not expired. An expired one is dropped first.
pub(super) fn session_pending(state: &mut PairingState, now: Instant) -> bool {
    if state
        .session
        .as_ref()
        .is_some_and(|session| now >= session.crypto.expires_at())
    {
        state.session = None;
    }
    state.session.is_some()
}
