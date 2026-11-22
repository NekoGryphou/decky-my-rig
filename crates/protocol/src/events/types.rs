use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// A paired device sent Stop while test mode was on.
    TestShutdown,
    /// A device on the network asked this PC to show a pairing code.
    PairingRequest,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HostEvent {
    pub sequence: u64,
    pub kind: EventKind,
    pub device_name: String,
    pub time_unix: u64,
}
