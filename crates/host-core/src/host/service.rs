use super::{errors::*, types::*};
use crate::{paired_devices::RegistryConfig, shutdown::Core};
use std::sync::{
    Mutex, RwLock,
    atomic::{AtomicBool, Ordering},
};

impl Host {
    pub fn new(
        config: RegistryConfig,
        core: Core,
        computer_name: String,
        persist: PairingPersistence,
    ) -> Self {
        let (port_change, _) = tokio::sync::watch::channel(config.port());
        Self {
            computer_name,
            host_id: config.host_id(),
            config: RwLock::new(config),
            registry_mutation: Mutex::new(()),
            core,
            events: crate::events::HostEvents::new(),
            pairing: Mutex::default(),
            pairing_finished: tokio::sync::Notify::new(),
            storage_healthy: AtomicBool::new(true),
            listener_available: AtomicBool::new(false),
            port_change,
            pair_requested_at: Mutex::new(None),
            persist,
        }
    }

    pub(crate) fn authenticated_device(&self, token: &str) -> Option<String> {
        self.storage_healthy
            .load(Ordering::Acquire)
            .then(|| self.config.read().ok()?.authenticate_device(token))
            .flatten()
    }

    pub fn events(&self) -> crate::events::HostEvents {
        self.events.clone()
    }

    pub fn test_mode(&self) -> Result<bool, RegistryError> {
        let config = self.config.read().map_err(|_| RegistryError::Unavailable)?;
        Ok(config.test_mode())
    }

    pub fn management_status(&self) -> Result<(u16, bool), RegistryError> {
        if !self.storage_healthy() {
            return Err(RegistryError::RecoveryRequired);
        }
        let config = self.config.read().map_err(|_| RegistryError::Unavailable)?;
        Ok((config.port(), config.test_mode()))
    }

    pub fn listener_state(&self) -> &'static str {
        if self.listener_available.load(Ordering::Acquire) {
            "listening"
        } else {
            "unavailable"
        }
    }

    pub fn set_listener_available(&self) {
        self.listener_available.store(true, Ordering::Release);
    }

    pub fn set_listener_unavailable(&self) {
        self.listener_available.store(false, Ordering::Release);
    }

    pub fn subscribe_port(&self) -> tokio::sync::watch::Receiver<u16> {
        self.port_change.subscribe()
    }

    pub fn host_id(&self) -> uuid::Uuid {
        self.host_id
    }

    pub fn storage_healthy(&self) -> bool {
        self.storage_healthy.load(Ordering::Acquire)
    }

    pub fn require_storage_recovery(&self) {
        self.storage_healthy.store(false, Ordering::Release);
    }

    /// The configuration as it is saved, to compare with what is on disk.
    pub fn config_bytes(&self) -> Result<Vec<u8>, RegistryError> {
        let config = self.config.read().map_err(|_| RegistryError::Unavailable)?;
        config.to_json().map_err(|_| RegistryError::Unavailable)
    }

    /// Takes requests again once the saved copy reads back exactly as memory
    /// holds it. Anything else, including a changed file, keeps them blocked.
    pub fn restore_storage(&self, saved: &[u8]) -> bool {
        let Ok(current) = self.config_bytes() else {
            return false;
        };
        if current != saved {
            return false;
        }
        self.storage_healthy.store(true, Ordering::Release);
        true
    }

    /// The saved port. The listener may still be rebinding to it.
    pub fn stored_port(&self) -> Result<u16, RegistryError> {
        let config = self.config.read().map_err(|_| RegistryError::Unavailable)?;
        Ok(config.port())
    }

    pub fn paired_devices(
        &self,
    ) -> Result<Vec<crate::paired_devices::PublicPairedDevice>, RegistryError> {
        if !self.storage_healthy() {
            return Err(RegistryError::RecoveryRequired);
        }
        let config = self.config.read().map_err(|_| RegistryError::Unavailable)?;
        Ok(config.public_summary().paired_devices)
    }
}
