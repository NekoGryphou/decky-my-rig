use super::types::Entry;
use crate::store::PcConfig;
use myrig_protocol::api::{PcSettings, PowerState};
use std::time::Instant;

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

pub(super) fn settings_of(pc: &PcConfig) -> PcSettings {
    PcSettings {
        id: pc.id.to_string(),
        name: pc.name.clone(),
        host: pc.host.to_string(),
        port: pc.port,
        mac: pc.mac.clone(),
    }
}
