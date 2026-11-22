use myrig_protocol::events::HostEvent;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

pub(super) struct State {
    pub(super) next: u64,
    pub(super) closed: bool,
    pub(super) events: VecDeque<HostEvent>,
}

pub(super) struct Inner {
    pub(super) state: Mutex<State>,
    /// Wakes every waiter on a new event or on close.
    pub(super) changed: tokio::sync::Notify,
}

#[derive(Clone)]
pub struct HostEvents(pub(super) Arc<Inner>);
