use crate::events::HostEvent;
use serde::{Deserialize, de::DeserializeOwned};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ErrorReply {
    pub(super) version: u32,
    pub(super) ok: bool,
    pub(super) error: String,
}

/// One reply to `wait_events`: the host it came from and its new events.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WaitReply {
    pub(super) version: u32,
    pub(super) ok: bool,
    pub host_id: String,
    /// Changes whenever the service process starts, so a restart is visible even
    /// when the new sequences pass an old cursor.
    pub epoch: String,
    pub baseline_sequence: u64,
    /// The service is stopping and answers at once; waiting again right away
    /// would only spin until the pipe goes away.
    pub closed: bool,
    pub events: Vec<HostEvent>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
/// The code is wiped when the reply is dropped.
pub struct OpenPairingReply {
    pub(super) version: u32,
    pub(super) ok: bool,
    pub host_id: String,
    pub code: String,
    pub expires_in_seconds: u64,
}

/// A management request. Changes name the host they were confirmed against.
pub enum ControlRequest<'a> {
    Status,
    WaitEvents {
        after: u64,
        epoch: Option<&'a str>,
    },
    List,
    OpenPairing {
        host_id: &'a str,
        name: &'a str,
    },
    SetPort {
        host_id: &'a str,
        port: u16,
    },
    SetTestMode {
        host_id: &'a str,
        enabled: bool,
    },
    Remove {
        host_id: &'a str,
        device_id: &'a str,
    },
    Rename {
        host_id: &'a str,
        device_id: &'a str,
        name: &'a str,
    },
}

/// Why a reply wasn't the expected success.
#[derive(Debug, PartialEq, Eq)]
pub enum ReplyError {
    /// The caller isn't allowed to manage the host.
    Unauthorized,
    /// The service refused with one of its known error codes.
    Host(&'static str),
    /// The reply was malformed or from another protocol version.
    Invalid,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StatusReply {
    pub(super) version: u32,
    pub(super) ok: bool,
    pub host_id: String,
    pub port: u16,
    pub test_mode: bool,
    pub service_version: String,
    pub listener_state: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListReply {
    pub(super) version: u32,
    pub(super) ok: bool,
    pub host_id: String,
    pub paired_devices: Vec<DeviceEntry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeviceEntry {
    pub device_id: String,
    pub name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortReply {
    pub(super) version: u32,
    pub(super) ok: bool,
    pub host_id: String,
    pub port: u16,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestModeReply {
    pub(super) version: u32,
    pub(super) ok: bool,
    pub host_id: String,
    pub test_mode: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoveReply {
    pub(super) version: u32,
    pub(super) ok: bool,
    pub host_id: String,
    pub removed: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenameReply {
    pub(super) version: u32,
    pub(super) ok: bool,
    pub host_id: String,
    pub renamed: bool,
}

/// A success reply: its envelope and any field rules the types don't cover.
pub trait Reply: DeserializeOwned {
    fn envelope(&self) -> (u32, bool, &str);
    fn fields_valid(&self) -> bool {
        true
    }
}
