use crate::{
    command::{Command, request_for_command},
    output::{format_list, format_status, messages},
    shared::CliError,
};
use myrig_protocol::local_control::client::{
    ControlRequest, ListReply, OpenPairingReply, PortReply, RemoveReply, RenameReply, Reply,
    StatusReply, TestModeReply, encode, parse,
};
use std::io::{self, ErrorKind};

pub fn connection_error(error: io::Error) -> CliError {
    match error.kind() {
        ErrorKind::PermissionDenied => CliError::Unauthorized,
        _ => CliError::Unavailable,
    }
}

/// Runs one command. A change first reads the host's identity and only accepts
/// a reply from that same host. A lost reply is never retried.
pub async fn execute_command<F, R>(command: &Command, mut send: F) -> Result<String, CliError>
where
    F: FnMut(Vec<u8>) -> R,
    R: std::future::Future<Output = Result<Vec<u8>, CliError>>,
{
    match command {
        Command::Status => {
            let status: StatusReply = parse(&send(encode(&ControlRequest::Status)).await?)?;
            return Ok(format_status(&status));
        }
        Command::List => {
            let list: ListReply = parse(&send(encode(&ControlRequest::List)).await?)?;
            return Ok(format_list(&list));
        }
        _ => {}
    }
    let status: StatusReply = parse(&send(encode(&ControlRequest::Status)).await?)?;
    let host_id = status.host_id.as_str();
    let request = encode(&request_for_command(command, host_id));
    match command {
        Command::Pair(_) => {
            let pairing: OpenPairingReply = from_host(host_id, &send(request).await?)?;
            Ok(messages::pairing(&pairing.code, pairing.expires_in_seconds))
        }
        Command::SetPort(expected) => {
            let saved: PortReply = from_host(host_id, &send(request).await?)?;
            confirmed(saved.port == *expected)?;
            Ok(messages::port(saved.port))
        }
        Command::SetTestMode(expected) => {
            let saved: TestModeReply = from_host(host_id, &send(request).await?)?;
            confirmed(saved.test_mode == *expected)?;
            Ok(messages::test_mode(saved.test_mode))
        }
        Command::Remove(device_id) => {
            let name = paired_name(host_id, device_id, &mut send).await?;
            let removed: RemoveReply = from_host(host_id, &send(request).await?)?;
            if !removed.removed {
                return Err(CliError::NotPaired);
            }
            Ok(messages::removed(&name))
        }
        // * `renamed: false` just means it already had that name; the device
        //   exists.
        Command::Rename(device_id, name) => {
            paired_name(host_id, device_id, &mut send).await?;
            from_host::<RenameReply>(host_id, &send(request).await?)?;
            Ok(messages::renamed(name))
        }
        Command::Status | Command::List => Err(CliError::Unavailable),
    }
}

/// The device's current name on the confirmed host.
async fn paired_name<F, R>(host_id: &str, device_id: &str, send: &mut F) -> Result<String, CliError>
where
    F: FnMut(Vec<u8>) -> R,
    R: std::future::Future<Output = Result<Vec<u8>, CliError>>,
{
    let list: ListReply = from_host(host_id, &send(encode(&ControlRequest::List)).await?)?;
    list.paired_devices
        .into_iter()
        .find(|device| device.device_id == device_id)
        .map(|device| device.name)
        .ok_or(CliError::NotPaired)
}

/// The reply, if it's a valid success from the host the command confirmed.
fn from_host<T: Reply>(host_id: &str, bytes: &[u8]) -> Result<T, CliError> {
    let reply: T = parse(bytes)?;
    confirmed(reply.envelope().2 == host_id)?;
    Ok(reply)
}

fn confirmed(condition: bool) -> Result<(), CliError> {
    condition.then_some(()).ok_or(CliError::Unavailable)
}
