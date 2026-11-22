use super::types::Command;
use myrig_protocol::local_control::client::ControlRequest;

/// The command these arguments describe, if they're valid.
pub fn parse_command(args: &[String]) -> Option<Command> {
    match args {
        [operation] if operation == "status" => Some(Command::Status),
        [operation] if operation == "list" => Some(Command::List),
        [operation, name]
            if operation == "pair" && myrig_protocol::validation::valid_name(name) =>
        {
            Some(Command::Pair(name.clone()))
        }
        [operation, value] if operation == "set-port" => {
            let port = value.parse::<u16>().ok()?;
            if port < myrig_protocol::local_control::MIN_SETTABLE_PORT {
                return None;
            }
            Some(Command::SetPort(port))
        }
        [operation, value] if operation == "test-mode" => match value.as_str() {
            "on" => Some(Command::SetTestMode(true)),
            "off" => Some(Command::SetTestMode(false)),
            _ => None,
        },
        [operation, value] if operation == "remove" => Some(Command::Remove(device_id(value)?)),
        [operation, value, name]
            if operation == "rename" && myrig_protocol::validation::valid_name(name) =>
        {
            Some(Command::Rename(device_id(value)?, name.clone()))
        }
        _ => None,
    }
}

fn device_id(value: &str) -> Option<String> {
    let id = uuid::Uuid::parse_str(value).ok()?;
    if id.to_string() != value {
        return None;
    }
    Some(value.to_owned())
}

/// The request a change sends to the host it was confirmed against.
pub fn request_for_command<'a>(command: &'a Command, host_id: &'a str) -> ControlRequest<'a> {
    match command {
        Command::Status => ControlRequest::Status,
        Command::List => ControlRequest::List,
        Command::Pair(name) => ControlRequest::OpenPairing { host_id, name },
        Command::SetPort(port) => ControlRequest::SetPort {
            host_id,
            port: *port,
        },
        Command::SetTestMode(enabled) => ControlRequest::SetTestMode {
            host_id,
            enabled: *enabled,
        },
        Command::Remove(device_id) => ControlRequest::Remove { host_id, device_id },
        Command::Rename(device_id, name) => ControlRequest::Rename {
            host_id,
            device_id,
            name,
        },
    }
}
