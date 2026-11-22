use super::super::{
    types::Reply,
    utils::{
        accepts_json, invalid, json_media, pairing_busy, pairing_unavailable, read_body,
        reply_body, single,
    },
};
use super::types::FinishRequest;
use crate::host::{Host, PairingRefusal};
use hyper::{Method, StatusCode, body::Incoming, header::HeaderValue, http::request::Parts};
use myrig_pairing_crypto::{CONFIRMATION_LEN, REQUEST_LEN, session::AttemptId};
use myrig_protocol::checked_json::CheckedJson;
use myrig_protocol::lan::{
    PAIR_FINISH_PATH, PAIR_INFO_PATH, PAIR_START_PATH, PairFinishReply, PairInfoReply,
    PairStartReply,
};
use myrig_protocol::validation::{decode_hex, lower_hex};
use serde_json::{Map, Value};
use std::sync::Arc;

/// The unauthenticated OPAQUE pairing routes. `binding` is the digest of this
/// host's TLS certificate.
pub(in crate::http) async fn handle(
    parts: Parts,
    body: Incoming,
    host: &Arc<Host>,
    binding: [u8; 32],
) -> Reply {
    let path = parts.uri.path();
    let info = path == PAIR_INFO_PATH;
    if !matches!(path, PAIR_INFO_PATH | PAIR_START_PATH | PAIR_FINISH_PATH)
        || parts.method != if info { Method::GET } else { Method::POST }
        || parts.uri.query().is_some()
        || single(&parts.headers, "accept").is_none()
        || !accepts_json(&parts.headers)
        || single(&parts.headers, "x-myrig-protocol-major") != Some("1")
        || parts.headers.contains_key("authorization")
        || (!info && !json_media(&parts.headers))
    {
        return invalid();
    }
    let Ok(bytes) = read_body(body).await else {
        return invalid();
    };
    let mut response = if info {
        if !bytes.is_empty() {
            return invalid();
        }
        let nonce = match host.secure_pairing_info(binding) {
            Ok(nonce) => nonce,
            Err(refusal) => return refused(refusal),
        };
        reply_body(
            StatusCode::OK,
            PairInfoReply {
                bootstrap_version: 1,
                nonce: lower_hex(&nonce),
            },
        )
    } else {
        let Ok(CheckedJson(value)) = serde_json::from_slice::<CheckedJson>(&bytes) else {
            return invalid();
        };
        let Some(object) = value.as_object() else {
            return invalid();
        };
        if path.ends_with("/start") {
            let Some(request) = object
                .get("request")
                .and_then(Value::as_str)
                .and_then(decode_hex::<REQUEST_LEN>)
            else {
                return invalid();
            };
            if object.len() != 1 {
                return invalid();
            }
            let result = match host.secure_pairing_start(&request, binding).await {
                Ok(result) => result,
                Err(refusal) => return refused(refusal),
            };
            reply_body(
                StatusCode::OK,
                PairStartReply {
                    attempt_id: lower_hex(result.challenge.attempt.as_bytes()),
                    session_nonce: lower_hex(&result.nonce),
                    response: lower_hex(&result.challenge.response),
                },
            )
        } else {
            let Some(request) = finish_request(object) else {
                return invalid();
            };
            let mut token = match host
                .secure_pairing_finish(
                    AttemptId::from_bytes(request.attempt),
                    &request.confirmation,
                    binding,
                    request.previous,
                )
                .await
            {
                Ok(token) => token,
                Err(refusal) => return refused(refusal),
            };
            reply_body(
                StatusCode::OK,
                PairFinishReply {
                    host_id: host.host_id.to_string(),
                    token: std::mem::take(&mut *token),
                    computer_name: host.computer_name.clone(),
                },
            )
        }
    };
    response
        .headers_mut()
        .insert("cache-control", HeaderValue::from_static("no-store"));
    response
}

fn refused(refusal: PairingRefusal) -> Reply {
    match refusal {
        PairingRefusal::Busy => pairing_busy(),
        PairingRefusal::Unavailable => pairing_unavailable(),
    }
}

/// The finish body: the attempt, its confirmation and, when a device re-pairs,
/// the token of the credential it replaces. Nothing else is accepted.
pub(super) fn finish_request(object: &Map<String, Value>) -> Option<FinishRequest> {
    let attempt = decode_hex::<32>(object.get("attempt_id")?.as_str()?)?;
    let confirmation = decode_hex::<CONFIRMATION_LEN>(object.get("confirmation")?.as_str()?)?;
    let previous = match object.get("previous_token") {
        None => None,
        Some(value) => {
            let text = value.as_str()?;
            decode_hex::<32>(text)?;
            Some(zeroize::Zeroizing::new(text.to_owned()))
        }
    };
    (object.len() == 2 + usize::from(previous.is_some())).then_some(FinishRequest {
        attempt,
        confirmation,
        previous,
    })
}
