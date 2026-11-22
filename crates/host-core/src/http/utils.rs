use super::{
    consts::{BODY_LIMIT, BODY_TIMEOUT},
    types::*,
};
use http_body_util::{BodyExt, Full, LengthLimitError, Limited};
use hyper::{
    Response, StatusCode,
    body::{Bytes, Incoming},
    header::{HeaderMap, HeaderValue},
};
use myrig_protocol::lan::{PROTOCOL_MAJOR, PROTOCOL_MINOR, ProtocolVersion, Versioned};
use serde::Serialize;
use serde_json::{Value, json};
use tokio::time::timeout;

/// A JSON reply. Every reply states the protocol version it speaks.
pub(super) fn reply(status: StatusCode, mut value: Value) -> Reply {
    value["protocol_version"] = json!({"major":PROTOCOL_MAJOR,"minor":PROTOCOL_MINOR});
    json_response(status, Bytes::from(value.to_string()))
}
/// A typed reply body, with the version like every other reply.
pub(super) fn reply_body<T: Serialize>(status: StatusCode, body: T) -> Reply {
    let versioned = Versioned {
        protocol_version: ProtocolVersion {
            major: PROTOCOL_MAJOR,
            minor: PROTOCOL_MINOR,
        },
        body,
    };
    match serde_json::to_vec(&versioned) {
        Ok(bytes) => json_response(status, Bytes::from(bytes)),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Host internal error.",
        ),
    }
}
fn json_response(status: StatusCode, bytes: Bytes) -> Reply {
    let mut response = Response::new(Full::new(bytes));
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert("content-type", HeaderValue::from_static("application/json"));
    response
}
pub(super) fn error(status: StatusCode, code: &'static str, message: &'static str) -> Reply {
    reply(status, json!({"error":{"code":code,"message":message}}))
}
pub(super) fn invalid() -> Reply {
    error(
        StatusCode::BAD_REQUEST,
        "invalid_request",
        "Invalid request.",
    )
}
pub(super) fn pairing_unavailable() -> Reply {
    error(
        StatusCode::FORBIDDEN,
        "pairing_unavailable",
        "Pairing is unavailable.",
    )
}
/// Another request is holding the code for a moment; the device can ask again.
pub(super) fn pairing_busy() -> Reply {
    error(
        StatusCode::SERVICE_UNAVAILABLE,
        "pairing_busy",
        "Pairing is busy.",
    )
}
/// Reads a request body within the size and time limits.
pub(super) async fn read_body(body: Incoming) -> Result<Bytes, BodyError> {
    match timeout(BODY_TIMEOUT, Limited::new(body, BODY_LIMIT).collect()).await {
        Ok(Ok(collected)) => Ok(collected.to_bytes()),
        Ok(Err(error)) if error.is::<LengthLimitError>() => Err(BodyError::TooLarge),
        _ => Err(BodyError::Invalid),
    }
}
pub(super) fn single<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let mut values = headers.get_all(name).iter();
    let value = values.next()?.to_str().ok()?;
    if values.next().is_some() {
        return None;
    }
    Some(value)
}

pub(super) fn accepts_json(headers: &HeaderMap) -> bool {
    // ? The more specific type wins, so application/json;q=0 overrides a
    //   wildcard.
    let mut best: Option<(u8, f32)> = None;
    for header in headers.get_all("accept") {
        let Ok(header) = header.to_str() else {
            return false;
        };
        for range in header.split(',') {
            let mut parts = range.split(';');
            let media = parts.next().unwrap_or("").trim().to_ascii_lowercase();
            let specificity = match media.as_str() {
                "application/json" => 2,
                "application/*" => 1,
                "*/*" => 0,
                _ => continue,
            };
            let mut quality = 1.0;
            for param in parts {
                let Some((key, value)) = param.trim().split_once('=') else {
                    return false;
                };
                if key.trim().eq_ignore_ascii_case("q") {
                    let Ok(q) = value.trim().parse::<f32>() else {
                        return false;
                    };
                    if !q.is_finite() || !(0.0..=1.0).contains(&q) {
                        return false;
                    }
                    quality = q;
                } else {
                    return false;
                }
            }
            if best.is_none_or(|(s, q)| specificity > s || (specificity == s && quality > q)) {
                best = Some((specificity, quality));
            }
        }
    }
    best.is_some_and(|(_, quality)| quality > 0.0)
}

pub(super) fn json_media(headers: &HeaderMap) -> bool {
    let Some(value) = single(headers, "content-type") else {
        return false;
    };
    let mut parts = value.split(';');
    if !parts
        .next()
        .unwrap_or("")
        .trim()
        .eq_ignore_ascii_case("application/json")
    {
        return false;
    }
    match (parts.next(), parts.next()) {
        (None, None) => true,
        (Some(parameter), None) => parameter
            .trim()
            .split_once('=')
            .is_some_and(|(key, value)| {
                key.trim().eq_ignore_ascii_case("charset")
                    && value.trim().trim_matches('"').eq_ignore_ascii_case("utf-8")
            }),
        _ => false,
    }
}
