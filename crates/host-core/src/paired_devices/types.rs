use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
pub struct RegistryConfig {
    pub(super) host_id: Uuid,
    pub(super) port: u16,
    pub(super) test_mode: bool,
    pub(super) paired_devices: Vec<PairedDevice>,
}

#[derive(Clone)]
pub(super) struct PairedDevice {
    pub(super) device_id: Uuid,
    pub(super) name: String,
    pub(super) token_sha256: [u8; 32],
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct PublicPairedDevice {
    pub device_id: String,
    pub name: String,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct PublicRegistryConfig {
    pub schema_version: u32,
    pub host_id: String,
    pub port: u16,
    pub test_mode: bool,
    pub paired_devices: Vec<PublicPairedDevice>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredConfig {
    pub(super) schema_version: u32,
    pub(super) host_id: String,
    pub(super) port: u16,
    pub(super) test_mode: bool,
    pub(super) paired_devices: Vec<StoredDevice>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredDevice {
    pub(super) device_id: String,
    pub(super) name: String,
    pub(super) token_sha256: String,
}
