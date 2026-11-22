use super::{
    consts::{MAX_STATUS_PROBES, OFFLINE_AFTER_FAILED_PROBES, TRANSITION_TIMEOUT},
    environment::Environment,
    messages::{
        CREDENTIALS_FROM_PAIRING, INVALID_IDENTIFIER, NOT_STARTABLE, NOT_STOPPABLE, PC_NOT_FOUND,
        START_FAILED, STOP_FAILED,
    },
    types::{Controller, Entry, State},
    utils::{begin_transition, deadline_passed, new_entry, settings_of, settle},
};
use crate::{
    host_client::HostError,
    shared::{PublicError, normalize_host, normalize_uuid},
    snapshots::Snapshots,
    store::{PcChanges, PcConfig, Store},
};
use myrig_protocol::{
    api::{DeleteResult, PcIssue, PcSettings, PcSnapshot, PowerState, SnapshotFrame, TestStops},
    lan::HostState,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};
use tokio::{sync::Semaphore, task::JoinSet};
use uuid::Uuid;

impl<E: Environment> Controller<E> {
    pub fn new(store: Arc<Store>, environment: E) -> Self {
        Self {
            store,
            environment,
            state: Mutex::new(State::default()),
            snapshots: Snapshots::new(),
        }
    }

    pub fn snapshots(&self) -> &Snapshots {
        &self.snapshots
    }

    /// Builds the next frame from the current state and publishes it. A refresh
    /// that changed nothing isn't published again, since every frame wakes up
    /// the frontend.
    pub fn publish(&self, refreshed: bool) -> SnapshotFrame {
        let frame = {
            let mut state = self.reconciled();
            let pcs: Vec<PcSnapshot> = self
                .store
                .configs()
                .iter()
                .filter_map(|pc| state.entries.get(&pc.id))
                .map(|entry| PcSnapshot {
                    id: entry.config.id.to_string(),
                    name: entry.config.name.clone(),
                    state: entry.state,
                    issue: entry.issue,
                })
                .collect();
            let test_stops = TestStops {
                count: state.test_stops,
                name: state.test_stop_name.clone(),
            };
            if let Some(last) = state.published.as_ref().filter(|last| {
                refreshed && last.refreshed && last.pcs == pcs && last.test_stops == test_stops
            }) {
                return last.clone();
            }
            state.revision += 1;
            let frame = SnapshotFrame {
                revision: state.revision,
                refreshed,
                pcs,
                test_stops,
                closed: false,
            };
            state.published = Some(frame.clone());
            frame
        };
        self.snapshots.publish(frame.clone());
        frame
    }

    /// Releases everyone waiting for a snapshot. The controller won't publish
    /// anything after this.
    pub fn close(&self) {
        self.snapshots.close();
    }

    pub fn settings(&self) -> Vec<PcSettings> {
        self.store.configs().iter().map(settings_of).collect()
    }

    pub fn detect_mac(&self, host: &str) -> Result<String, PublicError> {
        self.environment.detect_mac(normalize_host(host)?)
    }

    pub fn update_pc(&self, id: &str, changes: PcChanges) -> Result<PcSettings, PublicError> {
        let id = identifier(id)?;
        if changes.authentication.is_some() {
            return Err(PublicError(CREDENTIALS_FROM_PAIRING));
        }
        let updated = self.store.update(id, changes)?;
        drop(self.reconciled());
        Ok(settings_of(&updated))
    }

    /// Deletes the PC here first, then asks it to forget this device. Deleting
    /// locally first means a failed save changes nothing on either side.
    pub async fn delete_pc(&self, id: &str) -> Result<DeleteResult, PublicError> {
        let id = identifier(id)?;
        let config = self.store.configs().into_iter().find(|pc| pc.id == id);
        self.store.delete(id)?;
        drop(self.reconciled());
        let unpaired = match config {
            Some(config) => self.environment.unpair(&config).await.is_ok(),
            None => false,
        };
        Ok(DeleteResult { unpaired })
    }

    /// Sends one Wake-on-LAN packet to an offline PC. Issues never block it,
    /// since waking doesn't need credentials.
    pub async fn start(&self, id: &str) -> Result<(), PublicError> {
        let id = identifier(id)?;
        let (config, reservation) = self.begin(
            id,
            PowerState::Starting,
            |entry| entry.state == PowerState::Offline,
            NOT_STARTABLE,
        )?;
        self.publish(false);
        if self.environment.wake(&config).await.is_err() {
            if let Some(entry) = self.reconciled().current(id, &config)
                && entry.reservation == Some(reservation)
            {
                settle(entry, PowerState::Offline);
            }
            return Err(PublicError(START_FAILED));
        }
        Ok(())
    }

    /// Asks for a shutdown once and returns true if test mode kept the PC on.
    /// If the result is unclear, polling sorts it out; the request is never
    /// retried.
    pub async fn stop(&self, id: &str) -> Result<bool, PublicError> {
        let id = identifier(id)?;
        let (config, reservation) = self.begin(
            id,
            PowerState::Stopping,
            |entry| entry.state == PowerState::Online && entry.issue.is_none(),
            NOT_STOPPABLE,
        )?;
        self.publish(false);
        let result = match self.environment.shutdown(&config, Uuid::new_v4()).await {
            Ok(result) => result,
            Err(error) => {
                if let Some(issue) = issue_of(error) {
                    self.record_issue(id, &config, issue, true);
                }
                return Err(PublicError(STOP_FAILED));
            }
        };
        if result.test {
            let mut state = self.reconciled();
            state.test_stops += 1;
            state.test_stop_name = Some(config.name.clone());
            if let Some(entry) = state.current(id, &config)
                && entry.reservation == Some(reservation)
            {
                settle(entry, PowerState::Online);
            }
        }
        Ok(result.test)
    }

    /// Checks every PC, at most four at a time. Dropping the future cancels the
    /// checks still running.
    pub async fn refresh(self: &Arc<Self>) {
        let ids: Vec<Uuid> = self.store.configs().iter().map(|pc| pc.id).collect();
        let permits = Arc::new(Semaphore::new(MAX_STATUS_PROBES));
        let mut probes = JoinSet::new();
        for id in ids {
            let (controller, permits) = (self.clone(), permits.clone());
            probes.spawn(async move {
                if let Ok(_permit) = permits.acquire_owned().await {
                    controller.poll(id).await;
                }
            });
        }
        while probes.join_next().await.is_some() {}
    }

    /// Checks one PC and returns true if it answered online.
    pub async fn poll(&self, id: Uuid) -> bool {
        let Some(config) = self
            .reconciled()
            .entries
            .get(&id)
            .map(|entry| entry.config.clone())
        else {
            return false;
        };
        let status = match self.environment.status(&config).await {
            Ok(status) if status.host_id == config.authentication.host_id => status,
            Ok(_) => {
                self.record_issue(id, &config, PcIssue::Identity, false);
                return false;
            }
            Err(error) => {
                match issue_of(error)
                    .or((error == HostError::Invalid).then_some(PcIssue::Compatibility))
                {
                    Some(issue) => self.record_issue(id, &config, issue, false),
                    None => self.record_unanswered(id, &config),
                }
                return false;
            }
        };
        let now = self.environment.now();
        let mut state = self.reconciled();
        if let Some(entry) = state.current(id, &config) {
            entry.issue = None;
            entry.failed_probes = 0;
            match status.state {
                // * A PC that's stopping can still answer online for a moment.
                HostState::Online
                    if entry.state != PowerState::Stopping || deadline_passed(entry, now) =>
                {
                    settle(entry, PowerState::Online);
                }
                HostState::Online => {}
                HostState::ShuttingDown => {
                    if entry.state != PowerState::Stopping || entry.transition_deadline.is_none() {
                        entry.state = PowerState::Stopping;
                        entry.transition_deadline = Some(now + TRANSITION_TIMEOUT);
                    }
                }
            }
        }
        status.state == HostState::Online
    }

    fn begin(
        &self,
        id: Uuid,
        state: PowerState,
        allowed: impl FnOnce(&Entry) -> bool,
        refused: &'static str,
    ) -> Result<(PcConfig, u64), PublicError> {
        let deadline = self.environment.now() + TRANSITION_TIMEOUT;
        let mut guard = self.reconciled();
        guard.next_reservation += 1;
        let reservation = guard.next_reservation;
        let entry = guard
            .entries
            .get_mut(&id)
            .ok_or(PublicError(PC_NOT_FOUND))?;
        if !allowed(entry) {
            return Err(PublicError(refused));
        }
        begin_transition(entry, state, deadline, reservation);
        Ok((entry.config.clone(), reservation))
    }

    fn record_issue(&self, id: Uuid, observed: &PcConfig, issue: PcIssue, rejected_stop: bool) {
        if let Some(entry) = self.reconciled().current(id, observed) {
            entry.issue = Some(issue);
            entry.failed_probes = 0;
            if entry.state != PowerState::Stopping || rejected_stop {
                settle(entry, PowerState::Online);
            }
        }
    }

    fn record_unanswered(&self, id: Uuid, observed: &PcConfig) {
        let now = self.environment.now();
        if let Some(entry) = self.reconciled().current(id, observed) {
            entry.failed_probes += 1;
            if entry.state == PowerState::Starting {
                if deadline_passed(entry, now) {
                    settle(entry, PowerState::Offline);
                }
            } else if entry.failed_probes >= OFFLINE_AFTER_FAILED_PROBES {
                settle(entry, PowerState::Offline);
            }
        }
    }

    /// The state with one entry per saved PC. A new or changed record gets a
    /// fresh "checking" entry.
    fn reconciled(&self) -> MutexGuard<'_, State> {
        let configs = self.store.configs();
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        let mut entries = std::mem::take(&mut state.entries);
        state.entries = configs
            .into_iter()
            .map(|config| {
                let entry = match entries.remove(&config.id) {
                    Some(entry) if entry.config == config => entry,
                    _ => new_entry(config),
                };
                (entry.config.id, entry)
            })
            .collect::<HashMap<_, _>>();
        state
    }
}

impl State {
    /// The entry, if it still holds the record the check started from.
    fn current(&mut self, id: Uuid, observed: &PcConfig) -> Option<&mut Entry> {
        self.entries
            .get_mut(&id)
            .filter(|entry| entry.config == *observed)
    }
}

fn identifier(id: &str) -> Result<Uuid, PublicError> {
    normalize_uuid(id).map_err(|_| PublicError(INVALID_IDENTIFIER))
}

/// Host answers the user has to deal with, rather than something to retry.
fn issue_of(error: HostError) -> Option<PcIssue> {
    match error {
        HostError::PinMismatch => Some(PcIssue::Identity),
        HostError::Unauthorized => Some(PcIssue::Authentication),
        HostError::UnsupportedProtocol => Some(PcIssue::Compatibility),
        _ => None,
    }
}
