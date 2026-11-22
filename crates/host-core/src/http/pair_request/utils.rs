use super::super::{
    types::Reply,
    utils::{accepts_json, error, invalid, json_media, read_body, reply, single},
};
use crate::host::{Host, PairRequestError};
use hyper::{Method, StatusCode, body::Incoming, http::request::Parts};
use myrig_protocol::checked_json::CheckedJson;
use serde_json::{Value, json};
use std::time::Instant;

pub(super) fn parse_device_name(bytes: &[u8]) -> Option<String> {
    let CheckedJson(value) = serde_json::from_slice(bytes).ok()?;
    let object = value.as_object()?;
    if object.len() != 1 {
        return None;
    }
    object
        .get("device_name")
        .and_then(Value::as_str)
        .map(str::to_owned)
}

/// Unauthenticated on purpose: it reveals nothing, and only asks the PC to show
/// a code to whoever is in front of it.
pub(in crate::http) async fn handle(parts: Parts, body: Incoming, host: &Host) -> Reply {
    if parts.method != Method::POST
        || parts.uri.query().is_some()
        || !accepts_json(&parts.headers)
        || !json_media(&parts.headers)
        || single(&parts.headers, "x-myrig-protocol-major") != Some("1")
        || parts.headers.contains_key("authorization")
    {
        return invalid();
    }
    let Ok(bytes) = read_body(body).await else {
        return invalid();
    };
    let Some(device_name) = parse_device_name(&bytes) else {
        return invalid();
    };
    match host.request_pairing(&device_name, Instant::now()) {
        Ok(()) => reply(StatusCode::ACCEPTED, json!({})),
        Err(PairRequestError::Invalid) => invalid(),
        Err(PairRequestError::Busy) => error(
            StatusCode::CONFLICT,
            "pairing_busy",
            "A pairing code is already shown.",
        ),
        Err(PairRequestError::Unavailable) => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "unavailable",
            "Pairing is unavailable.",
        ),
    }
}
