use crate::{
    snapshots::Snapshots,
    store::{PcConfig, Store},
};
use myrig_protocol::api::{PcIssue, PowerState, SnapshotFrame};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};
use uuid::Uuid;

/// Owns the live state of the saved PCs. Each check is compared with the exact
/// saved record it started from, so an edit or delete in the meantime throws
/// the result away.
pub struct Controller<E> {
    pub(super) store: Arc<Store>,
    pub(super) environment: E,
    pub(super) state: Mutex<State>,
    pub(super) snapshots: Snapshots,
}

#[derive(Default)]
pub(super) struct State {
    pub(super) entries: HashMap<Uuid, Entry>,
    pub(super) revision: u64,
    pub(super) test_stops: u64,
    pub(super) test_stop_name: Option<String>,
    pub(super) next_reservation: u64,
    /// The last frame published, so a refresh that changed nothing doesn't send
    /// another.
    pub(super) published: Option<SnapshotFrame>,
}

/// What the controller knows about one saved PC.
pub(super) struct Entry {
    pub(super) config: PcConfig,
    pub(super) state: PowerState,
    /// Which Start or Stop owns the current transition.
    pub(super) reservation: Option<u64>,
    pub(super) failed_probes: u32,
    pub(super) transition_deadline: Option<Instant>,
    pub(super) issue: Option<PcIssue>,
}
