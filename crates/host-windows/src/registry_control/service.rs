//! Connects local messages to protected storage, with recovery.
use myrig_host_core::{
    file_storage::{FileStorage, StorageError},
    host::{Host, PairingApproval, PersistError},
    local_control,
    paired_devices::RegistryConfig,
    shutdown::{Core, Power},
};
use std::sync::{Arc, Mutex};

use super::types::RegistryControl;

impl RegistryControl {
    /// Built from one startup read of this protected store: power mode, port,
    /// host identity and the save baseline all come from it.
    pub(crate) fn from_loaded(
        store: FileStorage,
        bytes: Vec<u8>,
        config: RegistryConfig,
        power: Box<dyn Power>,
    ) -> Result<Arc<Self>, &'static str> {
        let computer_name = crate::shared::native::computer_name()?;
        // ? The host saves through this control, so it holds a weak link to avoid
        //   a cycle.
        Ok(Arc::new_cyclic(|control: &std::sync::Weak<Self>| {
            let control = control.clone();
            Self {
                host: Arc::new(Host::new(
                    config,
                    Core::new(power),
                    computer_name,
                    Arc::new(move |candidate| {
                        control
                            .upgrade()
                            .ok_or(PersistError::Unchanged)?
                            .persist(candidate)
                    }),
                )),
                store,
                baseline: Mutex::new(bytes),
            }
        }))
    }
    pub fn host(&self) -> Arc<Host> {
        self.host.clone()
    }

    /// Only call this after local approval. It uses the same fixed identity the
    /// TLS listener serves, and the code never goes over the LAN.
    pub async fn open_secure_pairing(
        self: &Arc<Self>,
        name: &str,
        identity: &myrig_host_core::tls::FixedIdentity,
        approval: PairingApproval,
    ) -> Result<zeroize::Zeroizing<String>, &'static str> {
        let (code, entropy, device_id) = myrig_host_core::pairing_entropy::generate()?;
        let code = zeroize::Zeroizing::new(code);
        let mut entropy = zeroize::Zeroizing::new(entropy);
        let nonce = myrig_host_core::pairing_entropy::nonce()?;
        let control = Arc::downgrade(self);
        self.host
            .open_secure_pairing(
                approval,
                name,
                &code,
                nonce,
                identity.leaf_der(),
                std::mem::take(&mut *entropy),
                device_id,
                Arc::new(move |candidate| {
                    control
                        .upgrade()
                        .ok_or(PersistError::Unchanged)?
                        .persist(candidate)
                }),
            )
            .await
            .map_err(|_| "Pairing could not be opened.")?;
        Ok(code)
    }

    /// Only captures the fixed startup identity. The pipe runs its callback on
    /// its own OS thread, and preparation runs on the service runtime. Stopping
    /// or disconnecting waits for that callback instead of canceling
    /// preparation.
    pub async fn serve_secure(
        self: Arc<Self>,
        pipe: crate::control_pipe::ControlPipe,
        stop: tokio::sync::watch::Receiver<bool>,
        identity: myrig_host_core::tls::FixedIdentity,
    ) -> Result<(), &'static str> {
        let runtime = tokio::runtime::Handle::current();
        let host = self.host.clone();
        let capture_host = self.host.clone();
        let serve = move |request: &[u8], approval| {
            let persist = |candidate: &[u8]| self.persist(candidate);
            let open = |name: String, approval| {
                let (control, identity) = (&self, &identity);
                async move { control.open_secure_pairing(&name, identity, approval).await }
            };
            runtime.block_on(local_control::execute_secure(
                &self.host, request, approval, persist, open,
            ))
        };
        let result = pipe
            .serve(serve, move || capture_host.pairing_approval(), stop)
            .await;
        if result.is_err() {
            host.require_storage_recovery();
        }
        result
    }

    fn persist(&self, candidate: &[u8]) -> Result<(), PersistError> {
        let mut baseline = self
            .baseline
            .lock()
            .map_err(|_| PersistError::RecoveryRequired)?;
        match self.store.replace(&baseline, candidate) {
            Ok(()) => {
                *baseline = candidate.to_vec();
                Ok(())
            }
            Err(StorageError::Conflict) => Err(PersistError::Conflict),
            Err(StorageError::Io) => match self.store.read() {
                Ok(current) if current == *baseline => Err(PersistError::Unchanged),
                Ok(_) => Err(PersistError::Conflict),
                Err(_) => Err(PersistError::RecoveryRequired),
            },
        }
    }
}
