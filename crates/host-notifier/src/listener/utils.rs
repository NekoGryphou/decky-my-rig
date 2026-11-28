use super::{
    consts::{OPEN_TIMEOUT, PAIRING_REQUEST_MAX_AGE},
    messages,
    types::Notifier,
};
use myrig_protocol::{
    events::{EventKind, HostEvent},
    local_control::client::{
        ControlRequest, OpenPairingReply, Reply, ReplyError, encode, exchange, parse,
    },
};
use std::{
    future::Future,
    io,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::timeout,
};

pub(super) async fn deliver<C, Connect, Stream>(
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
        EventKind::PairingRequest
            if notifier.shows_pairing_codes() && is_fresh(event.time_unix) =>
        {
            // * If the service refused to open a code, nothing is shown; the device tells its user
            // * the code is missing when they give up.
            let Ok(Some(mut stream)) = connect().await else {
                return;
            };
            let open = ControlRequest::OpenPairing {
                host_id,
                name: &event.device_name,
            };
            if let Ok(pairing) =
                request::<OpenPairingReply, _>(&mut stream, &open, OPEN_TIMEOUT).await
            {
                notifier.notify(
                    &messages::pairing_code_title(&pairing.code),
                    &messages::pairing_code_body(&event.device_name),
                );
            }
        }
        EventKind::PairingRequest => {}
    }
}

/// True when a pairing request is recent enough that its code can still be on
/// the PC. Events replayed after a reconnect can be much older.
fn is_fresh(time_unix: u64) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    now.saturating_sub(time_unix) <= PAIRING_REQUEST_MAX_AGE.as_secs()
}

/// Sends one request and parses the reply. An authorization refusal comes back
/// as `PermissionDenied`.
pub(super) async fn request<T: Reply, S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    request: &ControlRequest<'_>,
    limit: Duration,
) -> io::Result<T> {
    // ! A service that never answers must not hold the notifier forever. The timeout counts as a
    // ! failure, so the caller backs off and retries.
    let reply = match timeout(limit, exchange(stream, &encode(request))).await {
        Ok(reply) => reply?,
        Err(_) => return Err(io::ErrorKind::TimedOut.into()),
    };
    parse(&reply).map_err(|error| match error {
        ReplyError::Unauthorized => io::ErrorKind::PermissionDenied.into(),
        ReplyError::Host(code) => io::Error::other(code),
        ReplyError::Invalid => io::ErrorKind::InvalidData.into(),
    })
}
