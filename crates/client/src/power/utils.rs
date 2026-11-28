use super::types::Entry;
use crate::store::PcConfig;
use myrig_protocol::api::PowerState;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

pub(super) fn new_entry(config: PcConfig) -> Entry {
    Entry {
        config,
        state: PowerState::Checking,
        reservation: None,
        failed_probes: 0,
        transition_deadline: None,
        issue: None,
    }
}

pub(super) fn begin_transition(
    entry: &mut Entry,
    state: PowerState,
    deadline: Instant,
    reservation: u64,
) {
    entry.state = state;
    entry.reservation = Some(reservation);
    entry.failed_probes = 0;
    entry.transition_deadline = Some(deadline);
}

/// Ends any transition in a stable state.
pub(super) fn settle(entry: &mut Entry, state: PowerState) {
    entry.state = state;
    entry.reservation = None;
    entry.transition_deadline = None;
}

pub(super) fn deadline_passed(entry: &Entry, now: Instant) -> bool {
    entry
        .transition_deadline
        .is_some_and(|deadline| now >= deadline)
}

/// Where this process's revisions start: the clock in microseconds. The
/// frontend remembers the last revision it saw, so a restarted agent has to
/// start above anything a previous process published (publishes are far less
/// frequent than one per microsecond). It stays below 2^53, the largest
/// integer JavaScript keeps exactly.
pub(super) fn revision_base() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| u64::try_from(elapsed.as_micros()).unwrap_or(0))
}
