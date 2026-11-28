use super::types::{HostEvents, Inner, State};
use myrig_protocol::events::{EVENT_CAPACITY, EventKind, HostEvent};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

impl Default for HostEvents {
    fn default() -> Self {
        Self::new()
    }
}

impl HostEvents {
    pub fn new() -> Self {
        Self(Arc::new(Inner {
            state: Mutex::new(State {
                next: 1,
                closed: false,
                events: VecDeque::with_capacity(EVENT_CAPACITY),
            }),
            changed: tokio::sync::Notify::new(),
        }))
    }

    pub fn record(&self, kind: EventKind, device_name: String) {
        let mut state = self
            .0
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let sequence = state.next;
        state.next = state.next.saturating_add(1);
        if !state.closed {
            let time_unix = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            if state.events.len() == EVENT_CAPACITY {
                state.events.pop_front();
            }
            state.events.push_back(HostEvent {
                sequence,
                kind,
                device_name,
                time_unix,
            });
            self.0.changed.notify_waiters();
        }
    }

    pub fn latest_sequence(&self) -> u64 {
        let state = self
            .0
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        state.events.back().map_or(0, |event| event.sequence)
    }

    pub fn is_closed(&self) -> bool {
        self.state().closed
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.0
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
    }

    /// The events we still have that are newer than `after`, without waiting.
    pub fn events_after(&self, after: u64) -> Vec<HostEvent> {
        self.state()
            .events
            .iter()
            .filter(|event| event.sequence > after)
            .cloned()
            .collect()
    }

    /// Waits up to `timeout` for an event newer than `after`, or for close,
    /// then returns the newer events. Waiting doesn't tie up a thread.
    pub async fn wait_after(&self, after: u64, timeout: Duration) -> Vec<HostEvent> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let changed = self.0.changed.notified();
            tokio::pin!(changed);
            // ! Register before checking, so an event that lands in between still wakes us.
            changed.as_mut().enable();
            {
                let state = self.state();
                if state.closed
                    || state
                        .events
                        .back()
                        .is_some_and(|event| event.sequence > after)
                {
                    break;
                }
            }
            if tokio::time::timeout_at(deadline, changed).await.is_err() {
                break;
            }
        }
        self.events_after(after)
    }

    pub fn close(&self) {
        let mut state = self
            .0
            .state
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        state.closed = true;
        self.0.changed.notify_waiters();
    }
}
