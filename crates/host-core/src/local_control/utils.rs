use super::{
    consts::{EVENT_WAIT, OPEN_PAIRING_REPLY_CAPACITY},
    types::{OpenedPairing, Request},
};
use crate::host::{Host, PairingApproval, PersistError, RegistryError};
use myrig_protocol::{
    local_control::{MIN_SETTABLE_PORT, REQUEST_LIMIT, RESPONSE_LIMIT},
    validation::{canonical_uuid, lower_hex, valid_name, valid_pairing_code},
};
use serde_json::{Value, json};
use uuid::Uuid;
use zeroize::Zeroizing;

/// Only a scheduling hint; it doesn't allow anything to be dispatched.
pub fn is_event_wait_request(bytes: &[u8]) -> bool {
    !bytes.is_empty()
        && bytes.len() <= REQUEST_LIMIT
        && matches!(
            serde_json::from_slice::<Request>(bytes),
            Ok(Request::WaitEvents { version: 1, .. })
        )
}

/// Only the protected, authorized local pipe may call this. The approval
/// callback captures the identity from startup, and the JSON can't pick a key
/// or nonce.
pub async fn execute_secure<F, Fut>(
    host: &Host,
    bytes: &[u8],
    approval: PairingApproval,
    persist: impl FnOnce(&[u8]) -> Result<(), PersistError>,
    approve: F,
) -> Zeroizing<Vec<u8>>
where
    F: FnOnce(String, PairingApproval) -> Fut,
    Fut: std::future::Future<Output = Result<zeroize::Zeroizing<String>, &'static str>>,
{
    if bytes.is_empty() || bytes.len() > REQUEST_LIMIT {
        return error("invalid_request").into();
    }
    match serde_json::from_slice::<Request>(bytes) {
        Ok(Request::OpenPairing {
            version: 1,
            host_id,
            name,
        }) => {
            let host_id = match target_host(host, &host_id) {
                Ok(host_id) => host_id,
                Err(reply) => return reply.into(),
            };
            if !valid_name(&name) {
                return error("invalid_request").into();
            }
            match approve(name, approval).await {
                Ok(code) if valid_pairing_code(&code) => {
                    let expires_in_seconds = myrig_pairing_crypto::session::LIFETIME.as_secs();
                    // ! The code goes straight from its wiped string into a wiped, pre-sized
                    // ! buffer: no JSON value or copy that outlives the reply, and no growth that
                    // ! would leave a stale copy behind.
                    let mut reply = Zeroizing::new(Vec::with_capacity(OPEN_PAIRING_REPLY_CAPACITY));
                    match serde_json::to_writer(
                        &mut *reply,
                        &OpenedPairing {
                            version: 1,
                            ok: true,
                            host_id: host_id.to_string(),
                            code: code.as_str(),
                            expires_in_seconds,
                        },
                    ) {
                        Ok(()) => reply,
                        Err(_) => error("unavailable").into(),
                    }
                }
                _ => error("unavailable").into(),
            }
        }
        Ok(Request::WaitEvents {
            version: 1,
            after,
            epoch,
        }) => {
            let events = host.events();
            let baseline_sequence = events.latest_sequence();
            // * A cursor past the newest event comes from before a restart, so resend everything.
            // * So does a caller that last heard from another process.
            let restarted = epoch
                .as_deref()
                .is_some_and(|known| known != PROCESS_EPOCH.as_str());
            let events = if restarted || baseline_sequence < after {
                events.events_after(0)
            } else {
                events.wait_after(after, EVENT_WAIT).await
            };
            let host_id = host.host_id().to_string();
            json!({
                "closed": host.events().is_closed(),
                "version": 1,
                "ok": true,
                "host_id": host_id,
                "epoch": PROCESS_EPOCH.as_str(),
                "baseline_sequence": baseline_sequence,
                "events": events,
            })
            .to_string()
            .into_bytes()
            .into()
        }
        _ => execute(host, bytes, persist).into(),
    }
}

fn error(code: &'static str) -> Vec<u8> {
    json!({"version":1,"ok":false,"error":code})
        .to_string()
        .into_bytes()
}

pub fn execute(
    host: &Host,
    bytes: &[u8],
    persist: impl FnOnce(&[u8]) -> Result<(), PersistError>,
) -> Vec<u8> {
    if bytes.is_empty() || bytes.len() > REQUEST_LIMIT {
        return error("invalid_request");
    }
    let Ok(request) = serde_json::from_slice::<Request>(bytes) else {
        return error("invalid_request");
    };
    let bytes = match dispatch(host, request, persist) {
        Ok(value) => value.to_string().into_bytes(),
        Err(reply) => reply,
    };
    if bytes.len() > RESPONSE_LIMIT {
        return error("unavailable");
    }
    bytes
}

/// Runs one management request. An `Err` holds the encoded error reply.
fn dispatch(
    host: &Host,
    request: Request,
    persist: impl FnOnce(&[u8]) -> Result<(), PersistError>,
) -> Result<Value, Vec<u8>> {
    Ok(match request {
        Request::Status { version: 1 } => {
            let (port, test_mode) = host.management_status().map_err(registry_error)?;
            json!({"version":1,"ok":true,"host_id":host.host_id().to_string(),
                "port":port,"test_mode":test_mode,"service_version":env!("CARGO_PKG_VERSION"),
                "listener_state":host.listener_state()})
        }
        Request::List { version: 1 } => {
            let devices = host.paired_devices().map_err(registry_error)?;
            json!({"version":1,"ok":true,"host_id":host.host_id().to_string(),"paired_devices":devices})
        }
        Request::Remove {
            version: 1,
            host_id,
            device_id,
        } => {
            let device_id = canonical_uuid(&device_id).ok_or_else(|| error("invalid_request"))?;
            let host_id = target_host(host, &host_id)?;
            let removed = host
                .remove_device_checked(device_id, persist)
                .map_err(registry_error)?;
            json!({"version":1,"ok":true,"host_id":host_id.to_string(),"removed":removed})
        }
        Request::Rename {
            version: 1,
            host_id,
            device_id,
            name,
        } => {
            let device_id = canonical_uuid(&device_id).ok_or_else(|| error("invalid_request"))?;
            let host_id = target_host(host, &host_id)?;
            if !valid_name(&name) {
                return Err(error("invalid_request"));
            }
            let renamed = host
                .rename_device_checked(device_id, &name, persist)
                .map_err(registry_error)?;
            json!({"version":1,"ok":true,"host_id":host_id.to_string(),"renamed":renamed})
        }
        Request::ClosePairing {
            version: 1,
            host_id,
        } => {
            let host_id = target_host(host, &host_id)?;
            host.close_pairing();
            json!({"version":1,"ok":true,"host_id":host_id.to_string()})
        }
        Request::SetPort {
            version: 1,
            host_id,
            port,
        } => {
            let host_id = target_host(host, &host_id)?;
            if port < MIN_SETTABLE_PORT {
                return Err(error("invalid_request"));
            }
            host.set_port_checked(port, persist)
                .map_err(registry_error)?;
            let port = host.stored_port().map_err(registry_error)?;
            json!({"version":1,"ok":true,"host_id":host_id.to_string(),"port":port})
        }
        Request::SetTestMode {
            version: 1,
            host_id,
            enabled,
        } => {
            let host_id = target_host(host, &host_id)?;
            host.set_test_mode_checked(enabled, persist)
                .map_err(registry_error)?;
            let test_mode = host.test_mode().map_err(registry_error)?;
            json!({"version":1,"ok":true,"host_id":host_id.to_string(),"test_mode":test_mode})
        }
        _ => return Err(error("invalid_request")),
    })
}

/// The request's host ID, if it's canonical and is this host.
fn target_host(host: &Host, text: &str) -> Result<Uuid, Vec<u8>> {
    let host_id = canonical_uuid(text).ok_or_else(|| error("invalid_request"))?;
    if host_id == host.host_id() {
        Ok(host_id)
    } else {
        Err(error("wrong_host"))
    }
}

fn registry_error(error_value: RegistryError) -> Vec<u8> {
    error(match error_value {
        RegistryError::PersistenceFailed => "storage_failed",
        RegistryError::PersistenceConflict => "storage_conflict",
        RegistryError::RecoveryRequired => "recovery_required",
        RegistryError::Unavailable => "unavailable",
    })
}

/// Identifies this service process. A new one means the event sequence restarted.
static PROCESS_EPOCH: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    // ! The epoch only has to differ between processes. The clock stands in when the OS random
    // ! source is unavailable.
    let mut bytes = [0_u8; 16];
    if getrandom::getrandom(&mut bytes).is_err() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_nanos());
        bytes = nanos.to_be_bytes();
    }
    lower_hex(&bytes)
});
