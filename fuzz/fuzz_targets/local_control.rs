//! Any local-control request gets one bounded JSON reply in the protocol
//! envelope, and never schedules a shutdown.
#![no_main]
use libfuzzer_sys::fuzz_target;
use myrig_host_core::{
    host::Host,
    local_control::{execute, is_event_wait_request},
    paired_devices::RegistryConfig,
    shutdown::{Core, Power, PowerError},
};
use myrig_protocol::local_control::RESPONSE_LIMIT;
use serde_json::Value;
use std::sync::Arc;

const CONFIG: &[u8] = br#"{"schema_version":3,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","port":47991,"test_mode":false,"paired_devices":[{"device_id":"9fbf7536-c952-45a2-ac90-2c15076715fe","name":"Device","token_sha256":"72cd6e8422c407fb6d098690f1130b7ded7ec2f7f5e1d30bd9d521f015363793"}]}"#;

struct NoPower;
impl Power for NoPower {
    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        panic!("local control must not schedule power")
    }
    fn cancel_shutdown(&self) -> Result<(), PowerError> {
        panic!("local control must not cancel power")
    }
}

fuzz_target!(|data: &[u8]| {
    let _ = is_event_wait_request(data);
    let host = Host::new(
        RegistryConfig::from_json(CONFIG).expect("CONFIG is a fixed, valid schema-3 document"),
        Core::new(Box::new(NoPower)),
        "Fuzz PC".into(),
        Arc::new(|_| Ok(())),
    );
    let reply = execute(&host, data, |_| Ok(()));
    assert!(reply.len() <= RESPONSE_LIMIT);
    let reply: Value =
        serde_json::from_slice(&reply).expect("execute replies with serde_json text only");
    assert_eq!(reply["version"], 1);
    assert!(reply["ok"].is_boolean());
});
