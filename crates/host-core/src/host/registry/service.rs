use super::super::{Host, PersistError, RegistryError};

impl Host {
    pub fn set_test_mode_checked(
        &self,
        enabled: bool,
        persist: impl FnOnce(&[u8]) -> Result<(), PersistError>,
    ) -> Result<bool, RegistryError> {
        self.mutate_registry(|registry| registry.set_test_mode(enabled), persist, || {})
    }

    /// Local persistence that can recover. A conflict or an uncertain commit
    /// blocks new logins and changes; only a failure we know left the data
    /// unchanged keeps access. Requests that were already authenticated can
    /// still finish.
    pub fn remove_device_checked(
        &self,
        device_id: uuid::Uuid,
        persist: impl FnOnce(&[u8]) -> Result<(), PersistError>,
    ) -> Result<bool, RegistryError> {
        self.mutate_registry(|registry| registry.revoke(device_id), persist, || {})
    }

    /// A device removes its own pairing, authenticated by its `token`.
    pub fn revoke_own(&self, token: &str) -> Result<bool, RegistryError> {
        let persist = self.persist.clone();
        self.mutate_registry(
            |registry| registry.revoke_token(token),
            |bytes| persist(bytes),
            || {},
        )
    }

    pub fn rename_device_checked(
        &self,
        device_id: uuid::Uuid,
        name: &str,
        persist: impl FnOnce(&[u8]) -> Result<(), PersistError>,
    ) -> Result<bool, RegistryError> {
        self.mutate_registry(|registry| registry.rename(device_id, name), persist, || {})
    }

    /// Saves the port and asks for a listener rebind, in the same order as the
    /// save.
    pub fn set_port_checked(
        &self,
        port: u16,
        persist: impl FnOnce(&[u8]) -> Result<(), PersistError>,
    ) -> Result<bool, RegistryError> {
        self.mutate_registry(
            |registry| registry.set_port(port),
            persist,
            || {
                self.port_change.send_replace(port);
            },
        )
    }

    /// Applies `change` to a copy of the registry, saves it, then publishes it.
    /// Returns Ok(false) without saving when `change` says nothing changed.
    fn mutate_registry(
        &self,
        change: impl FnOnce(&mut crate::paired_devices::RegistryConfig) -> bool,
        persist: impl FnOnce(&[u8]) -> Result<(), PersistError>,
        on_published: impl FnOnce(),
    ) -> Result<bool, RegistryError> {
        let _mutation = self
            .registry_mutation
            .lock()
            .map_err(|_| RegistryError::Unavailable)?;
        if !self.storage_healthy() {
            return Err(RegistryError::RecoveryRequired);
        }
        let mut candidate = {
            let config = self.config.read().map_err(|_| RegistryError::Unavailable)?;
            config.clone()
        };
        if !change(&mut candidate) {
            return Ok(false);
        }
        let bytes = candidate
            .to_json()
            .map_err(|_| RegistryError::Unavailable)?;
        if let Err(error) = persist(&bytes) {
            return Err(match error {
                PersistError::Unchanged => RegistryError::PersistenceFailed,
                PersistError::Conflict => {
                    self.require_storage_recovery();
                    RegistryError::PersistenceConflict
                }
                PersistError::RecoveryRequired => {
                    self.require_storage_recovery();
                    RegistryError::RecoveryRequired
                }
            });
        }
        *self.config.write().map_err(|_| {
            self.require_storage_recovery();
            RegistryError::RecoveryRequired
        })? = candidate;
        on_published();
        Ok(true)
    }
}
