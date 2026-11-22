use myrig_host_core::{file_storage::FileStorage, host::Host};
use std::sync::{Arc, Mutex};

pub struct RegistryControl {
    pub(super) host: Arc<Host>,
    pub(super) store: FileStorage,
    pub(super) baseline: Mutex<Vec<u8>>,
}
