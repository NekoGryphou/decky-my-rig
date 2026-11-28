use super::super::*;
use crate::host::Host;
use crate::{
    paired_devices::RegistryConfig,
    shutdown::{Core, Power, PowerError},
};
use serde_json::json;

pub(super) const HOST: &str = "03c85317-9d93-4fb3-a04b-91a0e16668c6";

pub(super) const DEVICE: &str = "11111111-1111-4111-8111-111111111111";

pub(super) struct NoPower;

impl Power for NoPower {
    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        panic!("local control must not call power");
    }
    fn cancel_shutdown(&self) -> Result<(), PowerError> {
        panic!("local control must not call power")
    }
}

pub(super) fn host() -> Host {
    let config = json!({"schema_version":3,"host_id":HOST,"port":49000,"test_mode":true,"paired_devices":[
        {"device_id":DEVICE,"name":"Device","token_sha256":"00".repeat(32)},
        {"device_id":"22222222-2222-4222-8222-222222222222","name":"Other device","token_sha256":"01".repeat(32)}]});
    Host::new(
        RegistryConfig::from_json(&serde_json::to_vec(&config).unwrap()).unwrap(),
        Core::new(Box::new(NoPower)),
        "Fixture PC".into(),
        std::sync::Arc::new(|_| Ok(())),
    )
}

pub(super) fn remove() -> Vec<u8> {
    json!({"version":1,"operation":"remove","host_id":HOST,"device_id":DEVICE})
        .to_string()
        .into_bytes()
}

pub(super) fn reply(host: &Host, bytes: &[u8]) -> serde_json::Value {
    serde_json::from_slice(&execute(host, bytes, |_| panic!("must not persist"))).unwrap()
}

pub(super) fn fixture(name: &str) -> &'static str {
    match name {
        "list-request" => {
            include_str!("../../../../../tests/fixtures/local-control/list-request.json")
        }
        "list-reply" => {
            include_str!("../../../../../tests/fixtures/local-control/list-reply.json")
        }
        "remove-request" => {
            include_str!("../../../../../tests/fixtures/local-control/remove-request.json")
        }
        "remove-reply" => {
            include_str!("../../../../../tests/fixtures/local-control/remove-reply.json")
        }
        "open-pairing-request" => {
            include_str!("../../../../../tests/fixtures/local-control/open-pairing-request.json")
        }
        "open-pairing-reply" => {
            include_str!("../../../../../tests/fixtures/local-control/open-pairing-reply.json")
        }
        "invalid-request-duplicate" => {
            include_str!(
                "../../../../../tests/fixtures/local-control/invalid-request-duplicate.json"
            )
        }
        "error-reply" => {
            include_str!("../../../../../tests/fixtures/local-control/error-reply.json")
        }
        "set-port-request" => {
            include_str!("../../../../../tests/fixtures/local-control/set-port-request.json")
        }
        "set-port-reply" => {
            include_str!("../../../../../tests/fixtures/local-control/set-port-reply.json")
        }
        _ => panic!("unknown local-control fixture"),
    }
}

pub(super) fn fixture_json(name: &str) -> serde_json::Value {
    serde_json::from_str(fixture(name)).unwrap()
}

pub(super) fn rename(name: &str) -> Vec<u8> {
    json!({"version":1,"operation":"rename","host_id":HOST,"device_id":DEVICE,"name":name})
        .to_string()
        .into_bytes()
}
