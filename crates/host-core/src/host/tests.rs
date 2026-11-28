use super::Host;
use crate::{
    paired_devices::RegistryConfig,
    shutdown::{Core, Power, PowerError},
};
use serde_json::json;
use std::sync::Arc;

struct NoPower;

impl Power for NoPower {
    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        panic!("storage tests must not call power");
    }
    fn cancel_shutdown(&self) -> Result<(), PowerError> {
        panic!("storage tests must not call power");
    }
}

fn host() -> Host {
    let config = json!({"schema_version":3,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6",
        "port":49000,"test_mode":false,"paired_devices":[]});
    Host::new(
        RegistryConfig::from_json(&serde_json::to_vec(&config).unwrap()).unwrap(),
        Core::new(Box::new(NoPower)),
        "Fixture PC".into(),
        Arc::new(|_| Ok(())),
    )
}

#[test]
fn a_storage_failure_recovers_once_the_saved_copy_matches_memory_again() {
    // ! A failed read is not proof that the data is gone, so once the file reads back exactly as
    // ! memory holds it, the host may take requests again.
    let host = host();
    let saved = host.config_bytes().unwrap();
    host.require_storage_recovery();
    assert!(!host.storage_healthy());
    assert!(!host.restore_storage(b"changed elsewhere"));
    assert!(!host.storage_healthy());
    assert!(host.restore_storage(&saved));
    assert!(host.storage_healthy());
}
