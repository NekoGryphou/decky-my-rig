use super::{
    consts::MAX_LINE,
    messages::{INPUT_FAILED, OPERATION_FAILED},
    types::{Agent, Request},
    utils::{changes, drain, next_line, optional_text, poll_interval, port, text},
};
use crate::{
    drafts::Drafts,
    pairing::device_name,
    platform::Platform,
    power::{Controller, LiveEnvironment},
    shared::PublicError,
    store::Store,
};
use myrig_protocol::api::PowerState;
use serde_json::{Value, json};
use std::{
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{
    io::{AsyncBufRead, AsyncWrite, AsyncWriteExt},
    sync::{Notify, mpsc},
    task::JoinSet,
    time::sleep,
};

/// Lines waiting for stdout; a stalled reader holds senders back instead of
/// growing memory.
const OUTPUT_QUEUE: usize = 64;
/// Requests running at once; more input waits until one finishes.
const MAX_REQUESTS: usize = 32;

/// Serves one plugin session over `input`/`output` until `input` ends.
pub async fn serve<R, W>(
    platform: Arc<dyn Platform>,
    store: &Path,
    input: R,
    output: W,
) -> Result<(), PublicError>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let store = Arc::new(Store::open(store, platform.clone())?);
    let device = device_name(platform.as_ref());
    let agent = Arc::new(Agent {
        controller: Arc::new(Controller::new(store.clone(), LiveEnvironment(platform))),
        drafts: Drafts::new(store, device),
        changed: Notify::new(),
        attended: AtomicBool::new(false),
    });
    let (lines, receiver) = mpsc::channel::<String>(OUTPUT_QUEUE);
    let writer = tokio::spawn(write_lines(receiver, output));
    agent.controller.publish(false);
    let mut background = JoinSet::new();
    background.spawn(poll(agent.clone()));
    background.spawn(push_snapshots(agent.clone(), lines.clone()));
    let mut requests = JoinSet::new();
    let mut input = input;
    let mut line = Vec::new();
    let mut read_error = false;
    loop {
        let within = match next_line(&mut input, &mut line, MAX_LINE).await {
            Ok(Some(within)) => within,
            Ok(None) => break,
            Err(_) => {
                read_error = true;
                break;
            }
        };
        // - Clean up finished requests as we go instead of at the end.
        while requests.try_join_next().is_some() {}
        while requests.len() >= MAX_REQUESTS {
            requests.join_next().await;
        }
        let request = std::str::from_utf8(&line)
            .ok()
            .filter(|_| within)
            .and_then(|text| serde_json::from_str::<Request>(text).ok());
        let (agent, lines) = (agent.clone(), lines.clone());
        requests.spawn(async move {
            let reply = match request {
                Some(request) => match dispatch(&agent, &request.method, &request.params).await {
                    Ok(result) => json!({"id":request.id,"result":result}),
                    Err(error) => json!({"id":request.id,"error":error.0}),
                },
                None => json!({"id":null,"error":OPERATION_FAILED}),
            };
            let _ = lines.send(reply.to_string()).await;
        });
    }
    // * Let requests that are still running (like a pairing) finish, up to a limit.
    drain(&mut requests, || agent.drafts.holds_credential()).await;
    background.shutdown().await;
    agent.drafts.close().await;
    agent.controller.close();
    drop(lines);
    let _ = writer.await;
    if read_error {
        return Err(PublicError(INPUT_FAILED));
    }
    Ok(())
}

async fn dispatch(agent: &Agent, method: &str, params: &Value) -> Result<Value, PublicError> {
    let controller = &agent.controller;
    let detect = |host: std::net::Ipv4Addr| controller.detect_mac(&host.to_string());
    let value = match method {
        "settings" => json!(controller.settings()),
        "set_attention" => {
            let visible = params["visible"]
                .as_bool()
                .ok_or(PublicError(OPERATION_FAILED))?;
            // - A view just opened, so fetch fresh status right away.
            if visible && !agent.attended.swap(true, Ordering::Relaxed) {
                agent.changed.notify_one();
            } else if !visible {
                agent.attended.store(false, Ordering::Relaxed);
            }
            Value::Null
        }
        "detect_mac" => json!(controller.detect_mac(text(params, "host")?)?),
        "request_pairing_code" => {
            agent
                .drafts
                .request_code(text(params, "host")?, &port(params, "port")?)
                .await?;
            Value::Null
        }
        "begin_pairing" => json!(
            agent
                .drafts
                .begin(
                    text(params, "draft_id")?,
                    text(params, "host")?,
                    &port(params, "port")?,
                    text(params, "code")?,
                    optional_text(params, "identifier")?,
                    detect,
                )
                .await?
        ),
        "cancel_pairing" => {
            agent.drafts.cancel(text(params, "draft_id")?).await?;
            Value::Null
        }
        "save_pairing" => {
            let result = agent
                .drafts
                .save(
                    text(params, "draft_id")?,
                    text(params, "name")?,
                    text(params, "mac")?,
                )
                .await;
            changed(agent, || result.map(|saved| json!(saved)))?
        }
        "update_pc" => {
            let result = controller
                .update_pc(text(params, "identifier")?, changes(params)?)
                .await;
            changed(agent, || result.map(|saved| json!(saved)))?
        }
        "delete_pc" => {
            let result = controller.delete_pc(text(params, "identifier")?).await;
            changed(agent, || result.map(|deleted| json!(deleted)))?
        }
        "start" => {
            let result = controller.start(text(params, "identifier")?).await;
            changed(agent, || result.map(|()| Value::Null))?
        }
        "stop" => {
            let result = controller.stop(text(params, "identifier")?).await;
            changed(agent, || result.map(|test| json!(test)))?
        }
        _ => return Err(PublicError(OPERATION_FAILED)),
    };
    Ok(value)
}

/// Runs a call that may change PCs, then publishes the new state right away and
/// wakes the poller.
fn changed(
    agent: &Agent,
    call: impl FnOnce() -> Result<Value, PublicError>,
) -> Result<Value, PublicError> {
    let result = call();
    agent.controller.publish(false);
    agent.changed.notify_one();
    result
}

/// Checks every PC every 15 s while a view is open, every 60 s otherwise, and
/// every 2 s while a PC is starting, stopping or being checked.
async fn poll(agent: Arc<Agent>) {
    loop {
        agent.controller.refresh().await;
        let frame = agent.controller.publish(true);
        let transition = frame.pcs.iter().any(|row| {
            matches!(
                row.state,
                PowerState::Checking | PowerState::Starting | PowerState::Stopping
            )
        });
        let interval = poll_interval(transition, agent.attended.load(Ordering::Relaxed));
        tokio::select! {
            () = sleep(interval) => {}
            () = agent.changed.notified() => {}
        }
    }
}

async fn push_snapshots(agent: Arc<Agent>, lines: mpsc::Sender<String>) {
    let mut after = -1;
    while let Ok(frame) = agent.controller.snapshots().watch(after).await {
        after = i64::try_from(frame.revision).unwrap_or(i64::MAX);
        if lines
            .send(json!({"event":"snapshot","frame":frame}).to_string())
            .await
            .is_err()
            || frame.closed
        {
            return;
        }
    }
}

async fn write_lines<W: AsyncWrite + Unpin>(mut lines: mpsc::Receiver<String>, mut output: W) {
    while let Some(line) = lines.recv().await {
        let written = async {
            output.write_all(line.as_bytes()).await?;
            output.write_all(b"\n").await?;
            output.flush().await
        };
        if written.await.is_err() {
            return;
        }
    }
}
