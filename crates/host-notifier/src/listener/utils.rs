use super::{
    consts::{FIRST_BACKOFF, MAX_BACKOFF},
    messages,
    types::Notifier,
};
use myrig_protocol::{
    events::{EventKind, HostEvent},
    local_control::client::{
        ControlRequest, OpenPairingReply, Reply, ReplyError, WaitReply, encode, exchange, parse,
    },
};
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
    loop {
        let batch = match connect().await {
            Ok(Some(mut stream)) => {
                request::<WaitReply, _>(&mut stream, &ControlRequest::WaitEvents { after }).await
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
        backoff = FIRST_BACKOFF;
        // ? History is only skipped at first startup. A lower baseline means the
        //   host restarted, and its new events have to be delivered.
        if batch.baseline_sequence < after {
            after = 0;
        }
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
    }
}

async fn deliver<C, Connect, Stream>(
    event: &HostEvent,
    host_id: &str,
    connect: &mut C,
    notifier: &dyn Notifier,
) where
    C: FnMut() -> Connect,
    Connect: Future<Output = io::Result<Option<Stream>>>,
    Stream: AsyncRead + AsyncWrite + Unpin,
{
    match event.kind {
        EventKind::TestShutdown => notifier.notify(
            &messages::test_shutdown_title(&event.device_name),
            messages::TEST_SHUTDOWN_BODY,
        ),
        EventKind::PairingRequest if notifier.shows_pairing_codes() => {
            // * If the service refused to open a code, nothing is shown; the
            //   device tells its user the code is missing when they give up.
            let Ok(Some(mut stream)) = connect().await else {
                return;
            };
            let open = ControlRequest::OpenPairing {
                host_id,
                name: &event.device_name,
            };
            if let Ok(pairing) = request::<OpenPairingReply, _>(&mut stream, &open).await {
                notifier.notify(
                    &messages::pairing_code_title(&pairing.code),
                    &messages::pairing_code_body(&event.device_name),
                );
            }
        }
        EventKind::PairingRequest => {}
    }
}

/// Sends one request and parses the reply. An authorization refusal comes back
/// as `PermissionDenied`.
async fn request<T: Reply, S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    request: &ControlRequest<'_>,
) -> io::Result<T> {
    let reply = exchange(stream, &encode(request)).await?;
    parse(&reply).map_err(|error| match error {
        ReplyError::Unauthorized => io::ErrorKind::PermissionDenied.into(),
        ReplyError::Host(code) => io::Error::other(code),
        ReplyError::Invalid => io::ErrorKind::InvalidData.into(),
    })
}
