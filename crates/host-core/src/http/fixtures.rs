use crate::host::Host;
use crate::{paired_devices::RegistryConfig, shutdown::Core};
use serde_json::json;
use std::sync::Arc;

pub(super) fn empty_registry_host() -> Arc<Host> {
    struct NoPower;
    impl crate::shutdown::Power for NoPower {
        fn schedule_shutdown(&self) -> Result<(), crate::shutdown::PowerError> {
            panic!("pairing cannot power off");
        }
    }
    let value = json!({"schema_version":3,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","port":47991,"test_mode":true,"paired_devices":[]});
    Arc::new(Host::new(
        RegistryConfig::from_json(&serde_json::to_vec(&value).unwrap()).unwrap(),
        Core::new(Box::new(NoPower)),
        "Fixture PC".into(),
        std::sync::Arc::new(|_| Ok(())),
    ))
}
