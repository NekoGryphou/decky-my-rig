use super::{
    consts::{IDLE_POLL, STEADY_POLL, TRANSITION_POLL},
    messages::OPERATION_FAILED,
};
use crate::{shared::PublicError, store::PcChanges};
use serde_json::Value;
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncBufReadExt};

pub(super) fn text<'a>(params: &'a Value, name: &str) -> Result<&'a str, PublicError> {
    params[name].as_str().ok_or(PublicError(OPERATION_FAILED))
}

pub(super) fn optional_text<'a>(
    params: &'a Value,
    name: &str,
) -> Result<Option<&'a str>, PublicError> {
    match &params[name] {
        Value::Null => Ok(None),
        value => value
            .as_str()
            .map(Some)
            .ok_or(PublicError(OPERATION_FAILED)),
    }
}

/// The settings form sends the port as text, other callers may send a number.
pub(super) fn port(params: &Value, name: &str) -> Result<String, PublicError> {
    match &params[name] {
        Value::String(text) => Ok(text.clone()),
        Value::Number(number) if number.is_u64() => Ok(number.to_string()),
        _ => Err(PublicError(OPERATION_FAILED)),
    }
}

/// Edits to name, host, port and MAC. Credentials never change this way.
pub(super) fn changes(params: &Value) -> Result<PcChanges, PublicError> {
    let changes = &params["changes"];
    let allowed = ["name", "host", "port", "mac"];
    let object = changes.as_object().ok_or(PublicError(OPERATION_FAILED))?;
    if object.keys().any(|key| !allowed.contains(&key.as_str())) {
        return Err(PublicError(OPERATION_FAILED));
    }
    let field = |name| optional_text(changes, name).map(|value| value.map(str::to_owned));
    Ok(PcChanges {
        name: field("name")?,
        host: field("host")?,
        port: match changes.get("port") {
            None => None,
            Some(_) => Some(port(changes, "port")?),
        },
        mac: field("mac")?,
        authentication: None,
    })
}

/// Reads one line into `line`, keeping at most `limit` bytes. Returns `None` at
/// the end of input and `Some(false)` when the line was too long and got thrown
/// away.
pub(super) async fn next_line<R: AsyncBufRead + Unpin>(
    input: &mut R,
    line: &mut Vec<u8>,
    limit: usize,
) -> std::io::Result<Option<bool>> {
    line.clear();
    let mut within = true;
    let mut read_any = false;
    loop {
        let buffer = input.fill_buf().await?;
        if buffer.is_empty() {
            return Ok(read_any.then_some(within));
        }
        read_any = true;
        let (chunk, ended) = match buffer.iter().position(|byte| *byte == b'\n') {
            Some(end) => (&buffer[..end], Some(end + 1)),
            None => (buffer, None),
        };
        if within && line.len() + chunk.len() <= limit {
            line.extend_from_slice(chunk);
        } else {
            within = false;
            line.clear();
        }
        let used = ended.unwrap_or(buffer.len());
        input.consume(used);
        if ended.is_some() {
            return Ok(Some(within));
        }
    }
}

/// How long to wait before checking the PCs again: short while one is changing
/// state, longer when no plugin view is open.
pub(super) fn poll_interval(transition: bool, attended: bool) -> Duration {
    if transition {
        TRANSITION_POLL
    } else if attended {
        STEADY_POLL
    } else {
        IDLE_POLL
    }
}
