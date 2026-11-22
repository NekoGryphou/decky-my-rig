use crate::platform::Platform;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    net::Ipv4Addr,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use uuid::Uuid;

/// One process's handle on a settings directory. A lock on the directory keeps
/// writers in other processes out, and each write starts from disk.
pub struct Store {
    pub(super) directory: PathBuf,
    pub(super) platform: Arc<dyn Platform>,
    pub(super) pcs: Mutex<Vec<PcConfig>>,
}

/// The paired host's identity and this client's credential for it.
#[derive(Clone, PartialEq, Eq)]
pub struct Authentication {
    pub host_id: Uuid,
    /// Wiped from memory on drop, like every other copy of a secret.
    pub token: zeroize::Zeroizing<String>,
    pub tls_pin: [u8; 32],
}

impl fmt::Debug for Authentication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Authentication")
            .field("host_id", &self.host_id)
            .finish_non_exhaustive()
    }
}

/// A saved PC, normalized.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcConfig {
    pub id: Uuid,
    pub name: String,
    pub host: Ipv4Addr,
    pub port: u16,
    pub mac: String,
    pub authentication: Authentication,
}

/// A new PC as the user entered it. The port has a default if it's left out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PcInput {
    pub name: String,
    pub host: String,
    pub port: Option<String>,
    pub mac: String,
}

/// An edit. Fields left out keep their saved values.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PcChanges {
    pub name: Option<String>,
    pub host: Option<String>,
    pub port: Option<String>,
    pub mac: Option<String>,
    pub authentication: Option<Authentication>,
}

/// `pcs.json`. serde rejects unknown and duplicate keys.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredDocument {
    pub(super) schema_version: u64,
    pub(super) pcs: Vec<StoredPc>,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredPc {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) mac: String,
    pub(super) authentication: StoredAuthentication,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredAuthentication {
    pub(super) host_id: String,
    pub(super) token: String,
    pub(super) tls_pin: String,
}
