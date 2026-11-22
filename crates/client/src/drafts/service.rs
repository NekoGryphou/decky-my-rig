use super::{
    messages::{
        BUSY, CANCELLED, CODE_REQUIRED, NOT_READY, PC_CHANGED, PC_MISSING, SAVE_FAILED, UNAVAILABLE,
    },
    types::{Draft, DraftState, Drafts, Release, Reservation},
};
use crate::{
    pairing::{pair, request_code, revoke},
    shared::{
        PublicError, normalize_host, normalize_mac, normalize_name, normalize_port, normalize_uuid,
    },
    store::{PcChanges, PcConfig, PcInput, Store},
};
use myrig_protocol::{
    api::{PairingDraftResult, PcSettings},
    validation::valid_pairing_code,
};
use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

impl Drafts {
    pub fn new(store: Arc<Store>, device: String) -> Self {
        Self {
            store,
            device,
            state: Mutex::new(DraftState::default()),
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
    pub fn reserve(
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
                let saved = self
                    .store
                    .configs()
                    .into_iter()
                    .find(|pc| pc.id == original);
                Some(saved.ok_or(PublicError(PC_MISSING))?)
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
    pub async fn exchange(
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
        let paired = pair(host, port, code, previous).await?;
        let kept = original
            .as_ref()
            .is_some_and(|pc| pc.authentication.tls_pin == paired.authentication.tls_pin);
        let original = match original {
            Some(original) if kept => Some(self.keep_credential(original, &paired)?),
            other => other,
        };
        let (mac, mac_error) = match detect_mac(host) {
            Ok(mac) => (Some(mac), None),
            Err(error) => (None, Some(error.0.to_owned())),
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
        draft.paired = Some(paired);
        draft.kept = kept;
        release.published = true;
        Ok(result)
    }

    /// Saves the paired PC with the name and MAC address the user confirmed.
    pub fn save(&self, draft_id: &str, name: &str, mac: &str) -> Result<PcSettings, PublicError> {
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
        let saved = match draft.original {
            None => self.store.add(
                PcInput {
                    name,
                    host: draft.host.to_string(),
                    port: Some(draft.port.to_string()),
                    mac,
                },
                paired.authentication,
            ),
            Some(original) => {
                let changes = PcChanges {
                    name: Some(name),
                    host: Some(draft.host.to_string()),
                    port: Some(draft.port.to_string()),
                    mac: Some(mac),
                    authentication: Some(paired.authentication),
                };
                self.store
                    .update_if_current(&original, changes)
                    .and_then(|saved| saved.ok_or(PublicError(PC_CHANGED)))
            }
        };
        let saved = saved.map_err(|error| match error.0 {
            PC_CHANGED => error,
            _ => PublicError(SAVE_FAILED),
        })?;
        Ok(PcSettings {
            id: saved.id.to_string(),
            name: saved.name,
            host: saved.host.to_string(),
            port: saved.port,
            mac: saved.mac,
        })
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
            forget(draft).await;
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
    pub fn close(&self) {
        let mut state = self.locked();
        state.closed = true;
        let started = state.draft.as_ref().is_some_and(|draft| draft.started);
        let dropped = state.draft.take();
        if !started {
            state.active = None;
        }
        if let (Some(draft), Ok(runtime)) = (dropped, tokio::runtime::Handle::try_current()) {
            runtime.spawn(forget(draft));
        }
    }

    fn keep_credential(
        &self,
        original: PcConfig,
        paired: &crate::pairing::Paired,
    ) -> Result<PcConfig, PublicError> {
        let change = PcChanges {
            authentication: Some(paired.authentication.clone()),
            ..PcChanges::default()
        };
        let failed = |_| PublicError(SAVE_FAILED);
        match self
            .store
            .update_if_current(&original, change.clone())
            .map_err(failed)?
        {
            Some(stored) => Ok(stored),
            // ! The PC was edited meanwhile. It still gets the new credential,
            //   but the draft keeps the old record, and we report the edit so
            //   Save can't overwrite it.
            None => {
                self.store
                    .update_same_host(&original, change)
                    .map_err(failed)?;
                Ok(original)
            }
        }
    }

    fn is_current(&self, pc: &PcConfig) -> bool {
        self.store.configs().iter().any(|saved| saved == pc)
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
