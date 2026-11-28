use super::{
    consts::FALLBACK_DEVICE_NAME,
    messages::{FAILED, IDENTITY_CHANGED, UNREACHABLE, UNREACHABLE_FINISH},
    types::Failure,
};
use crate::{
    host_client::{HostError, HttpReply, parse_reply},
    platform::Platform,
    shared::normalize_name,
};
use serde::de::DeserializeOwned;
use std::time::{Duration, Instant};

/// This device's name as the PC will show it: the hostname if it's a valid
/// name, otherwise a generic one.
pub fn device_name(platform: &dyn Platform) -> String {
    device_name_from(&platform.hostname().unwrap_or_default())
}

pub(crate) fn device_name_from(hostname: &str) -> String {
    let hostname = hostname.strip_suffix('\n').unwrap_or(hostname);
    match normalize_name(hostname) {
        Ok(name) if name == hostname => name,
        _ => FALLBACK_DEVICE_NAME.to_owned(),
    }
}

pub(super) fn remaining(deadline: Instant) -> Result<Duration, Failure> {
    let left = deadline.saturating_duration_since(Instant::now());
    if left.is_zero() {
        return Err(Failure::Public(FAILED));
    }
    Ok(left)
}

/// One bootstrap reply: exactly the route's fields, plus protocol 1.0.
pub(super) fn bootstrap_reply<T: DeserializeOwned>(
    reply: &HttpReply,
    fields: &[&str],
) -> Result<T, Failure> {
    if reply.cache_control.as_deref() != Some("no-store") {
        return Err(Failure::Public(FAILED));
    }
    let value = parse_reply(reply, 200).map_err(Failure::Host)?;
    let exact = value.as_object().is_some_and(|object| {
        object.len() == fields.len() + 1 && fields.iter().all(|field| object.contains_key(*field))
    });
    let version_exact = value["protocol_version"]
        .as_object()
        .is_some_and(|version| {
            version.len() == 2 && version.get("minor").and_then(|minor| minor.as_u64()) == Some(0)
        });
    if !exact || !version_exact {
        return Err(Failure::Public(FAILED));
    }
    serde_json::from_value(value).map_err(|_| Failure::Public(FAILED))
}

pub(super) fn public_message(failure: Failure, finishing: bool) -> &'static str {
    match failure {
        Failure::Public(message) => message,
        Failure::Host(HostError::PinMismatch) => IDENTITY_CHANGED,
        Failure::Host(HostError::Connection | HostError::Timeout) if finishing => {
            UNREACHABLE_FINISH
        }
        Failure::Host(HostError::Connection | HostError::Timeout) => UNREACHABLE,
        Failure::Host(_) => FAILED,
    }
}
