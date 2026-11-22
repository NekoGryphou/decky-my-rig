use super::{
    consts::{FIRST_BACKOFF, HEALTHY, MAX_BACKOFF},
    types::{Bind, ListenerStop},
};
use crate::{host::Host, http::serve_secure_tls, tls::FixedIdentity};
use std::{io, net::Ipv4Addr, sync::Arc};
use tokio::{
    net::TcpListener,
    sync::watch,
    time::{Instant, sleep},
};

/// A plain bind on every interface, for platforms without anything better.
pub fn bind_any(port: u16) -> io::Result<std::net::TcpListener> {
    std::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, port))
}

fn listen(bind: Bind, port: u16) -> io::Result<TcpListener> {
    let listener = bind(port)?;
    listener.set_nonblocking(true)?;
    TcpListener::from_std(listener)
}

/// Keeps local management working when the LAN listener can't start, or stops
/// on an accept error. The bind is retried with a capped backoff, so a port
/// freed later or a passing resource shortage fixes itself. A new saved port
/// retries right away. Each failure streak is reported once, and a streak ends
/// when a listener serves for `HEALTHY` or the port changes.
pub async fn run(
    host: Arc<Host>,
    identity: FixedIdentity,
    bind: Bind,
    stop: watch::Receiver<bool>,
    on_error: impl Fn(u16, &io::Error),
) {
    let on_error = &on_error;
    let serving = host.clone();
    run_with(
        host,
        bind,
        stop,
        on_error,
        |listener, port, listener_stop| {
            serve_secure_tls(
                listener,
                serving.clone(),
                identity.clone(),
                listener_stop,
                move |error| on_error(port, error),
            )
        },
    )
    .await;
}

/// `run` with the serving step passed in, so tests can make accepting fail.
pub(super) async fn run_with<S, F>(
    host: Arc<Host>,
    bind: Bind,
    mut stop: watch::Receiver<bool>,
    on_error: impl Fn(u16, &io::Error),
    mut serve: S,
) where
    S: FnMut(TcpListener, u16, ListenerStop) -> F,
    F: Future<Output = io::Result<()>>,
{
    let mut ports = host.subscribe_port();
    let mut backoff = FIRST_BACKOFF;
    let mut reported = false;
    loop {
        if *stop.borrow_and_update() {
            break;
        }
        let port = *ports.borrow_and_update();
        let (failure, healthy) = match listen(bind, port) {
            Ok(listener) => {
                host.set_listener_available();
                let (mut listener_stop, mut listener_ports) = (stop.clone(), ports.clone());
                let ends: ListenerStop = Box::pin(async move {
                    tokio::select! {
                        _ = listener_stop.changed() => {},
                        _ = listener_ports.changed() => {},
                    }
                });
                let started = Instant::now();
                let result = serve(listener, port, ends).await;
                host.set_listener_unavailable();
                (result.err(), started.elapsed() >= HEALTHY)
            }
            Err(error) => {
                host.set_listener_unavailable();
                (Some(error), false)
            }
        };
        if healthy {
            backoff = FIRST_BACKOFF;
            reported = false;
        }
        let Some(error) = failure else {
            backoff = FIRST_BACKOFF;
            reported = false;
            continue;
        };
        if !reported {
            on_error(port, &error);
            reported = true;
        }
        tokio::select! {
            _ = stop.changed() => {},
            _ = ports.changed() => {
                backoff = FIRST_BACKOFF;
                reported = false;
            },
            () = sleep(backoff) => backoff = (backoff * 2).min(MAX_BACKOFF),
        }
    }
    host.set_listener_unavailable();
}
