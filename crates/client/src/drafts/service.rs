use super::{
    messages::{
        BUSY, CANCELLED, CODE_REQUIRED, NOT_READY, PC_CHANGED, PC_MISSING, SAVE_FAILED, UNAVAILABLE,
    },
    types::{Draft, DraftState, Drafts, Release, Reservation},
};
use crate::{
    pairing::{CREDENTIAL_BUDGET, pair_tracked, request_code, revoke},
    shared::{
        PublicError, normalize_host, normalize_mac, normalize_name, normalize_port, normalize_uuid,
    },
    store::{Authentication, PcChanges, PcConfig, PcInput, Store},
};
use myrig_protocol::{
    api::{PairingDraftResult, PcSettings},
    validation::valid_pairing_code,
};
use std::time::Duration;
use std::{
    net::Ipv4Addr,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{sync::oneshot, task::JoinSet};

impl Drafts {
    pub fn new(store: Arc<Store>, device: String) -> Self {
        Self {
            store,
            device,
            state: Mutex::new(DraftState::default()),
            holding: AtomicBool::new(false),
            settling: Mutex::new(JoinSet::new()),
        }
    }

    /// Reserves and pairs in one call (the plugin's "begin pairing").
    pub async fn begin(
        &self,
        draft_id: &str,
        host: &str,
        port: &str,
        code: &str,
        original: Option<&str>,
        detect_mac: impl Fn(Ipv4Addr) -> Result<String, PublicError>,
    ) -> Result<PairingDraftResult, PublicError> {
        let reservation = self.reserve(draft_id, host, port, original)?;
        self.exchange(reservation, code, detect_mac).await
    }

    /// Starts a draft for the PC at `host`:`port`. Pass `original` to re-pair a
    /// saved PC. Only one pairing can run at a time.
    pub(super) fn reserve(
        &self,
        draft_id: &str,
        host: &str,
        port: &str,
        original: Option<&str>,
    ) -> Result<Reservation, PublicError> {
        let id = normalize_uuid(draft_id)?;
        let (host, port) = (normalize_host(host)?, normalize_port(port)?);
        let original = match original {
            Some(original) => {
                let original = normalize_uuid(original)?;
                Some(self.store.get(original).ok_or(PublicError(PC_MISSING))?)
            }
            None => None,
        };
        let mut state = self.locked();
        if state.closed {
            return Err(PublicError(UNAVAILABLE));
        }
        if state.active.is_some() {
            return Err(PublicError(BUSY));
        }
        // ! A finished pairing holds a credential the PC already saved. Replacing its draft would
        // ! drop that credential without a revoke, so the user cancels or saves it first.
        if state
            .draft
            .as_ref()
            .is_some_and(|draft| draft.paired.is_some() && !draft.kept)
        {
            return Err(PublicError(BUSY));
        }
        state.next += 1;
        let generation = state.next;
        state.draft = Some(Draft {
            generation,
            id,
            host,
            port,
            original,
            started: false,
            paired: None,
            kept: false,
        });
        state.active = Some(generation);
        Ok(Reservation(generation))
    }

    /// Pairs using the code the PC shows. When a re-pair gets the same
    /// certificate, the new credential is saved right away, because the PC
    /// already revoked the old one. That way cancelling the edit afterwards
    /// can't lock this device out. With a different certificate, the old
    /// credential stays until the user hits Save.
    pub(super) async fn exchange(
        &self,
        reservation: Reservation,
        code: &str,
        detect_mac: impl Fn(Ipv4Addr) -> Result<String, PublicError>,
    ) -> Result<PairingDraftResult, PublicError> {
        let (host, port, original) = {
            let mut state = self.locked();
            let draft = state
                .draft_mut(reservation.0)
                .ok_or(PublicError(CANCELLED))?;
            if draft.started {
                return Err(PublicError(BUSY));
            }
            draft.started = true;
            (draft.host, draft.port, draft.original.clone())
        };
        let mut release = Release {
            drafts: self,
            generation: reservation.0,
            published: false,
        };
        if !valid_pairing_code(code) {
            return Err(PublicError(CODE_REQUIRED));
        }
        if !original.as_ref().is_none_or(|pc| self.is_current(pc)) {
            return Err(PublicError(PC_CHANGED));
        }
        if self.locked().draft_mut(reservation.0).is_none() {
            return Err(PublicError(CANCELLED));
        }
        let previous = original
            .as_ref()
            .map(|pc| (&pc.authentication.tls_pin, pc.authentication.token.as_str()));
        let paired = pair_tracked(host, port, code, previous, &self.holding).await?;
        let (mac, mac_error) = match detect_mac(host) {
            Ok(mac) => (Some(mac), None),
            Err(error) => (None, Some(error.0.to_owned())),
        };
        let mut stored = false;
        let settled = self
            .settle(
                reservation,
                original,
                &paired,
                &mut stored,
                (mac, mac_error),
                &mut release,
            )
            .await;
        // ! The PC has saved this credential already. If this device didn't store it, the PC keeps
        // ! a device that can never use it, so it's revoked.
        match settled {
            Ok(result) => Ok(result),
            Err(error) => {
                if !stored {
                    revoke(host, port, paired.authentication.clone()).await;
                }
                Err(error)
            }
        }
    }

    /// Stores the finished exchange on its draft. `stored` turns true once the
    /// new credential is in the store.
    async fn settle(
        &self,
        reservation: Reservation,
        original: Option<PcConfig>,
        paired: &crate::pairing::Paired,
        stored: &mut bool,
        (mac, mac_error): (Option<String>, Option<String>),
        release: &mut Release<'_>,
    ) -> Result<PairingDraftResult, PublicError> {
        let kept = original
            .as_ref()
            .is_some_and(|pc| pc.authentication.tls_pin == paired.authentication.tls_pin);
        let original = match original {
            Some(original) if kept => {
                let stored_pc = self.keep_credential(original, paired).await?;
                *stored = true;
                Some(stored_pc)
            }
            other => other,
        };
        let current = original.as_ref().is_none_or(|pc| self.is_current(pc));
        let mut state = self.locked();
        let draft = state
            .draft_mut(reservation.0)
            .ok_or(PublicError(CANCELLED))?;
        if !current {
            return Err(PublicError(PC_CHANGED));
        }
        let result = PairingDraftResult {
            draft_id: draft.id.to_string(),
            name: paired.computer_name.clone(),
            mac,
            mac_error,
        };
        draft.original = original;
        draft.paired = Some(paired.clone());
        draft.kept = kept;
        release.published = true;
        Ok(result)
    }

    /// Saves the paired PC with the name and MAC address the user confirmed.
    pub async fn save(
        &self,
        draft_id: &str,
        name: &str,
        mac: &str,
    ) -> Result<PcSettings, PublicError> {
        let id = normalize_uuid(draft_id)?;
        let (name, mac) = (normalize_name(name)?, normalize_mac(mac)?);
        let draft = {
            let mut state = self.locked();
            match &state.draft {
                Some(draft) if draft.id == id => {}
                _ => return Err(PublicError(CANCELLED)),
            }
            if state.active.is_some()
                || state
                    .draft
                    .as_ref()
                    .is_some_and(|draft| draft.paired.is_none())
            {
                return Err(PublicError(NOT_READY));
            }
            let draft = state.draft.take().ok_or(PublicError(CANCELLED))?;
            state.active = Some(draft.generation);
            draft
        };
        let _release = Release {
            drafts: self,
            generation: draft.generation,
            published: true,
        };
        let paired = draft.paired.ok_or(PublicError(NOT_READY))?;
        self.holding.store(!draft.kept, Ordering::SeqCst);
        // ! A kept credential is already in the store. Any other one only exists on the PC until
        // ! this save stores it.
        let unstored = (!draft.kept).then(|| paired.authentication.clone());
        let (host, port) = (draft.host, draft.port);
        let changes = SaveChanges {
            name,
            mac,
            host,
            port,
            authentication: paired.authentication,
        };
        let store = self.store.clone();
        let original = draft.original;
        // ! The store write and the revoke after a failed one run in a task that outlives this
        // ! future: if the agent drops the save while it waits for the store lock, the credential
        // ! is still stored or revoked, and `close` waits for it.
        let (sender, receiver) = oneshot::channel();
        {
            let mut settling = self.settling();
            while settling.try_join_next().is_some() {}
            settling.spawn(async move {
                let _ = sender.send(store_save(store, original, changes, unstored).await);
            });
        }
        receiver.await.unwrap_or(Err(PublicError(SAVE_FAILED)))
    }

    /// Drops the draft. If the PC issued a credential that never got saved,
    /// it's revoked on the PC.
    pub async fn cancel(&self, draft_id: &str) -> Result<(), PublicError> {
        let id = normalize_uuid(draft_id)?;
        let dropped = {
            let mut state = self.locked();
            let mut dropped = None;
            if let Some(draft) = state.draft.as_ref().filter(|draft| draft.id == id) {
                let (generation, started) = (draft.generation, draft.started);
                dropped = state.draft.take();
                if state.active == Some(generation) && !started {
                    state.active = None;
                }
            }
            dropped
        };
        if let Some(draft) = dropped {
            self.forget_holding(draft).await;
        }
        Ok(())
    }

    /// Asks the PC to show a pairing code for this device.
    pub async fn request_code(&self, host: &str, port: &str) -> Result<(), PublicError> {
        let (host, port) = (normalize_host(host)?, normalize_port(port)?);
        request_code(host, port, &self.device).await
    }

    /// Stops new pairings. A running exchange can finish but won't publish
    /// anything, and an unsaved draft's credential is revoked if the runtime
    /// lives long enough.
    pub async fn close(&self) {
        let dropped = {
            let mut state = self.locked();
            state.closed = true;
            let started = state.draft.as_ref().is_some_and(|draft| draft.started);
            let dropped = state.draft.take();
            if !started {
                state.active = None;
            }
            dropped
        };
        // ! The caller's runtime stops right after close, so the revoke is awaited here under a
        // ! deadline rather than spawned and forgotten.
        let mut settling = std::mem::take(&mut *self.settling());
        let _ = tokio::time::timeout(CREDENTIAL_BUDGET, async {
            while settling.join_next().await.is_some() {}
        })
        .await;
        if let Some(draft) = dropped {
            let _ = tokio::time::timeout(CLOSE_REVOKE_DEADLINE, self.forget_holding(draft)).await;
        }
    }

    /// Whether a pairing holds a credential the PC issued that isn't stored or
    /// revoked yet.
    pub fn holds_credential(&self) -> bool {
        self.holding.load(Ordering::SeqCst)
            || self
                .locked()
                .draft
                .as_ref()
                .is_some_and(|draft| draft.paired.is_some() && !draft.kept)
    }

    /// Revokes the draft's credential, counting it as held until that ends: the
    /// draft is out of the state by then.
    async fn forget_holding(&self, draft: Draft) {
        let held = draft.paired.is_some() && !draft.kept;
        if held {
            self.holding.store(true, Ordering::SeqCst);
        }
        forget(draft).await;
        if held {
            self.holding.store(false, Ordering::SeqCst);
        }
    }

    pub(super) async fn keep_credential(
        &self,
        original: PcConfig,
        paired: &crate::pairing::Paired,
    ) -> Result<PcConfig, PublicError> {
        let change = PcChanges {
            authentication: Some(paired.authentication.clone()),
            ..PcChanges::default()
        };
        let failed = |_| PublicError(SAVE_FAILED);
        let (observed, update) = (original.clone(), change.clone());
        let current = self
            .store
            .blocking(move |store| store.update_if_current(&observed, update))
            .await
            .map_err(failed)?;
        if let Some(stored) = current {
            return Ok(stored);
        }
        // ! The PC was edited meanwhile. The record is updated only when its identity still
        // ! matches. Otherwise nothing is stored, and the error keeps the caller from counting the
        // ! credential as stored.
        let observed = original.clone();
        let same_host = self
            .store
            .blocking(move |store| store.update_same_host(&observed, change))
            .await
            .map_err(failed)?;
        same_host.map(|_| original).ok_or(PublicError(PC_CHANGED))
    }

    fn is_current(&self, pc: &PcConfig) -> bool {
        self.store.get(pc.id).is_some_and(|saved| saved == *pc)
    }

    fn settling(&self) -> MutexGuard<'_, JoinSet<()>> {
        self.settling.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn locked(&self) -> MutexGuard<'_, DraftState> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl DraftState {
    fn draft_mut(&mut self, generation: u64) -> Option<&mut Draft> {
        self.draft
            .as_mut()
            .filter(|draft| draft.generation == generation)
    }
}

impl Drop for Release<'_> {
    fn drop(&mut self) {
        let mut state = self.drafts.locked();
        if state.active == Some(self.generation) {
            state.active = None;
            self.drafts.holding.store(false, Ordering::SeqCst);
        }
        if !self.published
            && state
                .draft
                .as_ref()
                .is_some_and(|draft| draft.generation == self.generation)
        {
            state.draft = None;
        }
    }
}

/// Revokes the credential the PC issued for a draft that was never saved.
async fn forget(draft: Draft) {
    if let Some(paired) = draft.paired.filter(|_| !draft.kept) {
        revoke(draft.host, draft.port, paired.authentication).await;
    }
}

/// Bounds the revoke at shutdown so a slow PC can't hold the plugin open.
const CLOSE_REVOKE_DEADLINE: Duration = Duration::from_secs(3);

/// What a save writes besides the credential's host: the confirmed name and MAC.
struct SaveChanges {
    name: String,
    mac: String,
    host: Ipv4Addr,
    port: u16,
    authentication: Authentication,
}

/// Stores a paired PC, or revokes its credential if that fails.
async fn store_save(
    store: Arc<Store>,
    original: Option<PcConfig>,
    changes: SaveChanges,
    unstored: Option<Authentication>,
) -> Result<PcSettings, PublicError> {
    let (host, port) = (changes.host, changes.port);
    let saved = match original {
        None => {
            let input = PcInput {
                name: changes.name,
                host: changes.host.to_string(),
                port: Some(changes.port.to_string()),
                mac: changes.mac,
            };
            let authentication = changes.authentication;
            store
                .blocking(move |store| store.add(input, authentication))
                .await
        }
        Some(original) => {
            let changes = PcChanges {
                name: Some(changes.name),
                host: Some(changes.host.to_string()),
                port: Some(changes.port.to_string()),
                mac: Some(changes.mac),
                authentication: Some(changes.authentication),
            };
            store
                .blocking(move |store| store.update_if_current(&original, changes))
                .await
                .and_then(|saved| saved.ok_or(PublicError(PC_CHANGED)))
        }
    };
    match saved {
        Ok(saved) => Ok(PcSettings::from(&saved)),
        Err(error) => {
            if let Some(authentication) = unstored {
                revoke(host, port, authentication).await;
            }
            Err(match error.0 {
                PC_CHANGED => error,
                _ => PublicError(SAVE_FAILED),
            })
        }
    }
}
