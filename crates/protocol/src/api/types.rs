use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PowerState {
    Checking,
    Offline,
    Starting,
    Online,
    Stopping,
}

/// A problem that disables Stop until the user re-pairs or updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PcIssue {
    Authentication,
    Compatibility,
    Identity,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PcSnapshot {
    pub id: String,
    pub name: String,
    pub state: PowerState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issue: Option<PcIssue>,
}

/// How many test-mode stops this session has seen, and the name of the last PC.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct TestStops {
    pub count: u64,
    pub name: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct SnapshotFrame {
    pub revision: u64,
    pub refreshed: bool,
    pub pcs: Vec<PcSnapshot>,
    pub test_stops: TestStops,
    pub closed: bool,
}

/// A saved PC without its credentials.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PcSettings {
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub mac: String,
}

/// Whether the PC also forgot this device when its settings were deleted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct DeleteResult {
    pub unpaired: bool,
}

/// A finished pairing exchange waiting for the user's name and MAC address.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PairingDraftResult {
    pub draft_id: String,
    pub name: String,
    pub mac: Option<String>,
    pub mac_error: Option<String>,
}
