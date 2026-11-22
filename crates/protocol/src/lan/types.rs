use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ProtocolVersion {
    pub major: u64,
    pub minor: u64,
}

/// Every host reply states the protocol version it speaks, next to its body.
#[derive(Serialize)]
pub struct Versioned<T> {
    pub protocol_version: ProtocolVersion,
    #[serde(flatten)]
    pub body: T,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HostState {
    Online,
    ShuttingDown,
}

/// `GET /v1/status`. IDs stay as text so each side checks the canonical form
/// itself.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct StatusReply {
    pub host_id: String,
    pub app_version: String,
    pub state: HostState,
}

/// `POST /v1/shutdown` and `POST /v1/shutdown/test`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ShutdownRequest {
    pub request_id: String,
}

/// `test` is only there, and true, when test mode kept the PC on.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ShutdownReply {
    pub request_id: String,
    pub state: HostState,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "super::utils::present_bool"
    )]
    pub test: Option<bool>,
}

/// `DELETE /v1/pairing`. `revoked` is only false if another removal got there
/// first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct UnpairReply {
    pub revoked: bool,
}

/// `POST /v1/pair/request`: asks the PC to show a code. It carries no secret.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PairCodeRequest {
    pub device_name: String,
}

/// `GET /v1/pair/opaque/info`.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PairInfoReply {
    pub bootstrap_version: u64,
    pub nonce: String,
}

/// `POST /v1/pair/opaque/start`. Byte fields are lowercase hex.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PairStartRequest {
    pub request: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PairStartReply {
    pub attempt_id: String,
    pub session_nonce: String,
    pub response: String,
}

/// `POST /v1/pair/opaque/finish`. `previous_token` names the credential a
/// re-pair replaces, and is only sent to the certificate that issued it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PairFinishRequest {
    pub attempt_id: String,
    pub confirmation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub previous_token: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PairFinishReply {
    pub host_id: String,
    pub token: String,
    pub computer_name: String,
}
