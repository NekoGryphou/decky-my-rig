use super::super::{
    REQUEST_LIMIT, RESPONSE_LIMIT,
    framing::{read_frame, write_frame},
};
use super::{
    consts::{HOST_ERRORS, UNAUTHORIZED_ERROR},
    types::{
        ControlRequest, ErrorReply, ListReply, OpenPairingReply, PortReply, RemoveReply,
        RenameReply, Reply, ReplyError, StatusReply, TestModeReply, WaitReply,
    },
};
use crate::local_control::MIN_SETTABLE_PORT;
use crate::validation::{canonical_uuid, valid_name, valid_pairing_code};
use serde_json::json;
use std::io;
use tokio::io::{AsyncRead, AsyncWrite};

/// One framed request and its framed reply on a connected control stream.
pub async fn exchange<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    request: &[u8],
) -> io::Result<Vec<u8>> {
    write_frame(stream, request, REQUEST_LIMIT).await?;
    read_frame(stream, RESPONSE_LIMIT).await
}

pub fn encode(request: &ControlRequest) -> Vec<u8> {
    match request {
        ControlRequest::Status => json!({"version":1,"operation":"status"}),
        ControlRequest::List => json!({"version":1,"operation":"list"}),
        ControlRequest::WaitEvents { after, epoch } => {
            json!({"version":1,"operation":"wait_events","after":after,"epoch":epoch})
        }
        ControlRequest::OpenPairing { host_id, name } => {
            json!({"version":1,"operation":"open_pairing","host_id":host_id,"name":name})
        }
        ControlRequest::SetPort { host_id, port } => {
            json!({"version":1,"operation":"set_port","host_id":host_id,"port":port})
        }
        ControlRequest::SetTestMode { host_id, enabled } => {
            json!({"version":1,"operation":"set_test_mode","host_id":host_id,"enabled":enabled})
        }
        ControlRequest::Remove { host_id, device_id } => {
            json!({"version":1,"operation":"remove","host_id":host_id,"device_id":device_id})
        }
        ControlRequest::Rename { host_id, device_id, name } => {
            json!({"version":1,"operation":"rename","host_id":host_id,"device_id":device_id,"name":name})
        }
    }
    .to_string()
    .into_bytes()
}

/// The expected success reply, or why the service didn't give one.
pub fn parse<T: Reply>(bytes: &[u8]) -> Result<T, ReplyError> {
    if bytes.is_empty() || bytes.len() > RESPONSE_LIMIT {
        return Err(ReplyError::Invalid);
    }
    if let Ok(error) = serde_json::from_slice::<ErrorReply>(bytes) {
        if error.version != 1 || error.ok {
            return Err(ReplyError::Invalid);
        }
        if error.error == UNAUTHORIZED_ERROR {
            return Err(ReplyError::Unauthorized);
        }
        return Err(HOST_ERRORS
            .iter()
            .find(|code| **code == error.error)
            .map_or(ReplyError::Invalid, |code| ReplyError::Host(code)));
    }
    let reply: T = serde_json::from_slice(bytes).map_err(|_| ReplyError::Invalid)?;
    let (version, ok, host_id) = reply.envelope();
    if version != 1 || !ok || canonical_uuid(host_id).is_none() || !reply.fields_valid() {
        return Err(ReplyError::Invalid);
    }
    Ok(reply)
}

macro_rules! envelope {
    ($($reply:ty),*) => {$(
        impl Reply for $reply {
            fn envelope(&self) -> (u32, bool, &str) {
                (self.version, self.ok, &self.host_id)
            }
        }
    )*};
}
envelope!(RemoveReply, RenameReply, TestModeReply);

impl Reply for StatusReply {
    fn envelope(&self) -> (u32, bool, &str) {
        (self.version, self.ok, &self.host_id)
    }
    fn fields_valid(&self) -> bool {
        self.port >= MIN_SETTABLE_PORT
            && !self.service_version.is_empty()
            && matches!(self.listener_state.as_str(), "listening" | "unavailable")
    }
}

impl Reply for ListReply {
    fn envelope(&self) -> (u32, bool, &str) {
        (self.version, self.ok, &self.host_id)
    }
    fn fields_valid(&self) -> bool {
        self.paired_devices
            .iter()
            .all(|device| canonical_uuid(&device.device_id).is_some() && valid_name(&device.name))
    }
}

impl Reply for PortReply {
    fn envelope(&self) -> (u32, bool, &str) {
        (self.version, self.ok, &self.host_id)
    }
    fn fields_valid(&self) -> bool {
        self.port >= MIN_SETTABLE_PORT
    }
}

impl Reply for OpenPairingReply {
    fn envelope(&self) -> (u32, bool, &str) {
        (self.version, self.ok, &self.host_id)
    }
    fn fields_valid(&self) -> bool {
        (1..=super::super::PAIRING_CODE_LIFETIME.as_secs()).contains(&self.expires_in_seconds)
            && valid_pairing_code(&self.code)
    }
}

impl Reply for WaitReply {
    fn envelope(&self) -> (u32, bool, &str) {
        (self.version, self.ok, &self.host_id)
    }
    fn fields_valid(&self) -> bool {
        self.events.len() <= crate::events::EVENT_CAPACITY
            && self
                .events
                .iter()
                .all(|event| valid_name(&event.device_name))
            && self
                .events
                .windows(2)
                .all(|pair| pair[0].sequence < pair[1].sequence)
    }
}

impl Drop for OpenPairingReply {
    fn drop(&mut self) {
        zeroize::Zeroize::zeroize(&mut self.code);
    }
}
