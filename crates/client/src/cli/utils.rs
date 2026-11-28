use super::{
    consts::{DEFAULT_PORT, REFUSED},
    messages::SHUTDOWN_STUCK,
    types::{Command, Failure},
};
use crate::{
    shared::{PublicError, normalize_port},
    store::PcConfig,
};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

/// The command named by `arguments`, or `None` for a usage error.
pub(super) fn parse(arguments: &[String]) -> Option<Command> {
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let owned = |text: &str| text.to_owned();
    Some(match words.as_slice() {
        ["list"] => Command::List,
        ["status"] => Command::Status(None),
        ["status", name] => Command::Status(Some(owned(name))),
        ["wake", name] => Command::Wake(owned(name)),
        ["stop", name] => Command::Stop {
            name: owned(name),
            yes: false,
        },
        ["stop", name, "--yes"] => Command::Stop {
            name: owned(name),
            yes: true,
        },
        ["pair", host] => Command::Pair {
            host: owned(host),
            port: owned(DEFAULT_PORT),
        },
        ["pair", host, "--port", port] if normalize_port(port).is_ok() => Command::Pair {
            host: owned(host),
            port: owned(port),
        },
        ["rename", name, new_name] => Command::Rename {
            name: owned(name),
            new_name: owned(new_name),
        },
        ["remove", name] => Command::Remove {
            name: owned(name),
            yes: false,
        },
        ["remove", name, "--yes"] => Command::Remove {
            name: owned(name),
            yes: true,
        },
        ["agent", "--store", store] => Command::Agent {
            store: PathBuf::from(store),
        },
        _ => return None,
    })
}

/// Finds the one saved PC with this name or ID.
pub(super) fn resolve(pcs: &[PcConfig], name: &str) -> Result<PcConfig, Failure> {
    let matches: Vec<&PcConfig> = pcs
        .iter()
        .filter(|pc| pc.name == name || pc.id.to_string() == name)
        .collect();
    match matches.as_slice() {
        [pc] => Ok((*pc).clone()),
        [] => Err(refused(format!("No saved PC is named {name}."))),
        several => Err(refused(format!(
            "Several PCs are named {name}; use an ID:\n{}",
            several
                .iter()
                .map(|pc| format!("  {}", pc.id))
                .collect::<Vec<_>>()
                .join("\n")
        ))),
    }
}

pub(super) fn refused(message: impl Into<String>) -> Failure {
    Failure {
        code: REFUSED,
        message: message.into(),
    }
}

/// Stops `runtime`, waiting at most `bound` for its blocking tasks. A store
/// write stuck behind another process's lock would otherwise keep the process
/// alive for as long as that process holds it.
pub(super) fn stop_runtime(
    runtime: tokio::runtime::Runtime,
    bound: Duration,
) -> Result<(), PublicError> {
    let started = Instant::now();
    runtime.shutdown_timeout(bound);
    if started.elapsed() >= bound {
        return Err(PublicError(SHUTDOWN_STUCK));
    }
    Ok(())
}
