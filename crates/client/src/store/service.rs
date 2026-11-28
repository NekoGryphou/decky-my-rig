use super::{
    consts::MAX_DOCUMENT_BYTES,
    messages::{LOAD_FAILED, LOCK_FAILED, NOT_SAVED, PC_NOT_FOUND, TOO_LARGE},
    types::{Authentication, PcChanges, PcConfig, PcInput, Store},
    utils::{apply, create, decode, encode, open_error, read_error},
};
use crate::{platform::Platform, shared::PublicError};
use std::{
    path::Path,
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
};
use uuid::Uuid;
use zeroize::Zeroizing;

impl Store {
    /// Opens or creates the private settings directory and loads `pcs.json`.
    pub fn open(directory: &Path, platform: Arc<dyn Platform>) -> Result<Self, PublicError> {
        let directory = std::path::absolute(directory).map_err(|_| PublicError(LOAD_FAILED))?;
        let settings = platform
            .open_settings(&directory, true)
            .map_err(open_error)?;
        let pcs = load(settings.as_ref())?;
        Ok(Self {
            directory,
            platform,
            pcs: Mutex::new(pcs),
            saves: AtomicU64::new(0),
        })
    }

    /// The PCs on disk right now, so a CLI write shows up in a running agent.
    /// If the file can't be read or isn't valid, the last good list stands:
    /// a bad file mustn't make every saved PC disappear from view.
    pub fn configs(&self) -> Vec<PcConfig> {
        self.refreshed().clone()
    }

    /// Runs `inspect` on the list read like `configs`, without copying it. The
    /// store's cache is locked meanwhile, so `inspect` must stay short and
    /// must not call the store.
    pub fn with_configs<T>(&self, inspect: impl FnOnce(&[PcConfig]) -> T) -> T {
        inspect(&self.refreshed())
    }

    /// One PC, read like `configs` but without copying the others.
    pub fn get(&self, id: Uuid) -> Option<PcConfig> {
        self.refreshed().iter().find(|pc| pc.id == id).cloned()
    }

    /// Runs one of the saving calls below off the async runtime. They wait for
    /// the directory lock and sync the file, and another process can hold that
    /// lock, so on the runtime they would stop every other request.
    pub async fn blocking<T: Send + 'static>(
        self: &Arc<Self>,
        call: impl FnOnce(&Store) -> Result<T, PublicError> + Send + 'static,
    ) -> Result<T, PublicError> {
        let store = Arc::clone(self);
        tokio::task::spawn_blocking(move || call(&store))
            .await
            .map_err(|_| PublicError(NOT_SAVED))?
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

    /// The disk is read before the cache lock is taken, so a reader inside the
    /// platform never holds up a save. A save that finished meanwhile wins.
    fn refreshed(&self) -> MutexGuard<'_, Vec<PcConfig>> {
        let saves = self.saves.load(Ordering::SeqCst);
        let fresh = self
            .platform
            .open_settings(&self.directory, false)
            .map_err(open_error)
            .and_then(|settings| load(settings.as_ref()));
        let mut cache = self.cache();
        if let Ok(pcs) = fresh
            && self.saves.load(Ordering::SeqCst) == saves
        {
            *cache = pcs;
        }
        cache
    }

    fn set_cache(&self, pcs: Vec<PcConfig>) {
        let mut cache = self.cache();
        *cache = pcs;
        self.saves.fetch_add(1, Ordering::SeqCst);
    }

    fn cache(&self) -> MutexGuard<'_, Vec<PcConfig>> {
        self.pcs.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// One read-modify-write under the directory lock. It starts from the file
    /// on disk, so another process's write is never undone, and memory only
    /// changes after the write succeeds. The cache mutex is never held while
    /// waiting for the lock: readers would stall behind another process.
    fn mutate<T>(
        &self,
        change: impl FnOnce(&mut Vec<PcConfig>) -> Result<T, PublicError>,
    ) -> Result<T, PublicError> {
        let settings = self
            .platform
            .open_settings(&self.directory, false)
            .map_err(open_error)?;
        settings.lock().map_err(|_| PublicError(LOCK_FAILED))?;
        let current = load(settings.as_ref())?;
        self.set_cache(current.clone());
        let mut pcs = current.clone();
        let result = change(&mut pcs)?;
        if pcs != current {
            let raw = encode(&pcs).map_err(|_| PublicError(NOT_SAVED))?;
            if raw.len() > MAX_DOCUMENT_BYTES {
                return Err(PublicError(TOO_LARGE));
            }
            settings.replace(&raw).map_err(|_| PublicError(NOT_SAVED))?;
            self.set_cache(pcs);
        }
        Ok(result)
    }
}

/// The saved PCs, or none if there's no document yet. The error says whether
/// the file couldn't be read, isn't private, is too big or isn't valid.
fn load(settings: &dyn crate::platform::Settings) -> Result<Vec<PcConfig>, PublicError> {
    match settings.read(MAX_DOCUMENT_BYTES).map_err(read_error)? {
        None => Ok(Vec::new()),
        Some(raw) => decode(&Zeroizing::new(raw)).ok_or(PublicError(LOAD_FAILED)),
    }
}
