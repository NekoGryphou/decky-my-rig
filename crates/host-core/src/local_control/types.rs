use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Request {
    Status {
        version: u32,
    },
    List {
        version: u32,
    },
    Remove {
        version: u32,
        host_id: String,
        device_id: String,
    },
    Rename {
        version: u32,
        host_id: String,
        device_id: String,
        name: String,
    },
    OpenPairing {
        version: u32,
        host_id: String,
        name: String,
    },
    ClosePairing {
        version: u32,
        host_id: String,
    },
    SetPort {
        version: u32,
        host_id: String,
        port: u16,
    },
    SetTestMode {
        version: u32,
        host_id: String,
        enabled: bool,
    },
    WaitEvents {
        version: u32,
        after: u64,
        /// The service process the caller last heard from; a different one restarts
        /// the sequence, so the events are read from the start.
        #[serde(default)]
        epoch: Option<String>,
    },
}

/// The reply to a successful `open_pairing`; `code` is secret.
#[derive(Serialize)]
pub(super) struct OpenedPairing<'a> {
    pub(super) version: u32,
    pub(super) ok: bool,
    pub(super) host_id: String,
    pub(super) code: &'a str,
    pub(super) expires_in_seconds: u64,
}
