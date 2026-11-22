use serde::Deserialize;

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
    },
}
