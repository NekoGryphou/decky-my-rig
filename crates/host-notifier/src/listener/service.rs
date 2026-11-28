use super::{
    consts::{FIRST_BACKOFF, MAX_BACKOFF, WAIT_TIMEOUT},
    types::Notifier,
    utils::{deliver, request},
};
use myrig_protocol::local_control::client::{ControlRequest, WaitReply};
use std::{future::Future, io, time::Duration};
use tokio::io::{AsyncRead, AsyncWrite};

/// Delivers host events until `connect` stops giving a stream. Failures just
/// back off.
pub async fn run<C, Connect, S, Sleep, Stream>(
    mut connect: C,
    notifier: &dyn Notifier,
    mut sleep: S,
) where
    C: FnMut() -> Connect,
    Connect: Future<Output = io::Result<Option<Stream>>>,
    S: FnMut(Duration) -> Sleep,
    Sleep: Future<Output = ()>,
    Stream: AsyncRead + AsyncWrite + Unpin,
{
    let mut after = 0;
    let mut baseline = true;
    let mut backoff = FIRST_BACKOFF;
    let mut authorization_reported = false;
    let mut known_epoch: Option<String> = None;
    loop {
        let batch = match connect().await {
            Ok(Some(mut stream)) => {
                request::<WaitReply, _>(
                    &mut stream,
                    &ControlRequest::WaitEvents {
                        after,
                        epoch: known_epoch.as_deref(),
                    },
                    WAIT_TIMEOUT,
                )
                .await
            }
            Ok(None) => return,
            Err(error) => Err(error),
        };
        let batch = match batch {
            Ok(batch) => batch,
            Err(error) => {
                if error.kind() == io::ErrorKind::PermissionDenied && !authorization_reported {
                    notifier.unauthorized();
                    authorization_reported = true;
                }
                sleep(backoff).await;
                backoff = (backoff * 2).min(MAX_BACKOFF);
                continue;
            }
        };
        authorization_reported = false;
        // ! A stopping service answers at once, so a reconnect without a wait would spin for as
        // ! long as its pipe stays up.
        let closed = batch.closed;
        if !closed {
            backoff = FIRST_BACKOFF;
        }
        // ? History is only skipped at first startup. A lower baseline means the host restarted,
        // ? and its new events have to be delivered.
        if batch.baseline_sequence < after {
            after = 0;
        }
        // ! A new service process restarts its sequence whatever the numbers look like, so its
        // ! events are read from the start.
        if known_epoch
            .as_deref()
            .is_some_and(|known| known != batch.epoch)
        {
            after = 0;
        }
        known_epoch = Some(batch.epoch.clone());
        for event in batch.events {
            if event.sequence <= after {
                continue;
            }
            after = event.sequence;
            if !baseline || event.sequence > batch.baseline_sequence {
                deliver(&event, &batch.host_id, &mut connect, notifier).await;
            }
        }
        baseline = false;
        if closed {
            sleep(backoff).await;
            backoff = (backoff * 2).min(MAX_BACKOFF);
        }
    }
}
