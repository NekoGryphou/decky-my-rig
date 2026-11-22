use super::{
    consts::MAX_DOCUMENT_BYTES,
    messages::{LOAD_FAILED, NOT_SAVED, PC_NOT_FOUND, TOO_LARGE},
    types::{Authentication, PcChanges, PcConfig, PcInput, Store},
    utils::{apply, create, decode, encode},
};
use crate::{platform::Platform, shared::PublicError};
use std::{
    path::Path,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};
use uuid::Uuid;

impl Store {
    /// Opens or creates the private settings directory and loads `pcs.json`.
    pub fn open(directory: &Path, platform: Arc<dyn Platform>) -> Result<Self, PublicError> {
        let directory = std::path::absolute(directory).map_err(|_| PublicError(LOAD_FAILED))?;
        let settings = platform
            .open_settings(&directory, true)
            .map_err(|_| PublicError(LOAD_FAILED))?;
        let pcs = load(settings.as_ref()).ok_or(PublicError(LOAD_FAILED))?;
        Ok(Self {
            directory,
            platform,
            pcs: Mutex::new(pcs),
        })
    }

    pub fn configs(&self) -> Vec<PcConfig> {
        self.cache().clone()
    }

    pub fn add(
        &self,
        input: PcInput,
        authentication: Authentication,
    ) -> Result<PcConfig, PublicError> {
        let pc = create(input, authentication)?;
        self.mutate(|pcs| {
            pcs.push(pc.clone());
            Ok(pc)
        })
    }

    pub fn update(&self, id: Uuid, changes: PcChanges) -> Result<PcConfig, PublicError> {
        self.mutate(|pcs| {
            let current = pcs
                .iter_mut()
                .find(|pc| pc.id == id)
                .ok_or(PublicError(PC_NOT_FOUND))?;
            *current = apply(current, changes)?;
            Ok(current.clone())
        })
    }

    /// Commits only if the saved record still equals `observed`. Any edit or
    /// removal since it was read cancels the change, an in-flight pairing for
    /// example.
    pub fn update_if_current(
        &self,
        observed: &PcConfig,
        changes: PcChanges,
    ) -> Result<Option<PcConfig>, PublicError> {
        self.update_matching(observed.id, |current| current == observed, changes)
    }

    /// Commits if the record still points at the same host identity (host ID
    /// and certificate pin), whatever else changed.
    pub fn update_same_host(
        &self,
        observed: &PcConfig,
        changes: PcChanges,
    ) -> Result<Option<PcConfig>, PublicError> {
        let identity = &observed.authentication;
        self.update_matching(
            observed.id,
            |current| {
                current.authentication.host_id == identity.host_id
                    && current.authentication.tls_pin == identity.tls_pin
            },
            changes,
        )
    }

    pub fn delete(&self, id: Uuid) -> Result<(), PublicError> {
        self.mutate(|pcs| {
            let before = pcs.len();
            pcs.retain(|pc| pc.id != id);
            if pcs.len() == before {
                return Err(PublicError(PC_NOT_FOUND));
            }
            Ok(())
        })
    }

    fn update_matching(
        &self,
        id: Uuid,
        matches: impl FnOnce(&PcConfig) -> bool,
        changes: PcChanges,
    ) -> Result<Option<PcConfig>, PublicError> {
        self.mutate(|pcs| match pcs.iter_mut().find(|pc| pc.id == id) {
            Some(current) if matches(current) => {
                *current = apply(current, changes)?;
                Ok(Some(current.clone()))
            }
            _ => Ok(None),
        })
    }

    fn cache(&self) -> MutexGuard<'_, Vec<PcConfig>> {
        self.pcs.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// One read-modify-write under this store's mutex and the directory lock.
    /// It starts from the file on disk, so another process's write is never
    /// undone, and memory only changes after the write succeeds.
    fn mutate<T>(
        &self,
        change: impl FnOnce(&mut Vec<PcConfig>) -> Result<T, PublicError>,
    ) -> Result<T, PublicError> {
        let mut cache = self.cache();
        let settings = self
            .platform
            .open_settings(&self.directory, false)
            .map_err(|_| PublicError(NOT_SAVED))?;
        settings.lock().map_err(|_| PublicError(NOT_SAVED))?;
        *cache = load(settings.as_ref()).ok_or(PublicError(LOAD_FAILED))?;
        let mut pcs = cache.clone();
        let result = change(&mut pcs)?;
        if pcs != *cache {
            let raw = encode(&pcs).map_err(|_| PublicError(NOT_SAVED))?;
            if raw.len() > MAX_DOCUMENT_BYTES {
                return Err(PublicError(TOO_LARGE));
            }
            settings.replace(&raw).map_err(|_| PublicError(NOT_SAVED))?;
            *cache = pcs;
        }
        Ok(result)
    }
}

/// The saved PCs, or none if there's no document yet.
fn load(settings: &dyn crate::platform::Settings) -> Option<Vec<PcConfig>> {
    match settings.read(MAX_DOCUMENT_BYTES).ok()? {
        None => Some(Vec::new()),
        Some(raw) => decode(&raw),
    }
}
