use super::super::{
    consts::*,
    pair_request, pairing, shutdown,
    types::*,
    utils::{accepts_json, error, invalid, json_media, read_body, reply, reply_body, single},
};
use super::types::Route;
use crate::host::Host;
use hyper::{
    Method, Request, StatusCode, body::Incoming, header::HeaderValue, server::conn::http1,
    service::service_fn,
};
use hyper_util::rt::{TokioIo, TokioTimer};
use myrig_protocol::lan::{
    HostState, PAIR_REQUEST_PATH, PAIRING_PATH, SHUTDOWN_PATH, SHUTDOWN_TEST_PATH, STATUS_PATH,
    StatusReply, UnpairReply,
};
use serde_json::json;
use std::{
    collections::HashMap,
    convert::Infallible,
    future::Future,
    io,
    net::{IpAddr, SocketAddr},
    sync::Arc,
    time::Instant,
};
use tokio::{
    net::{TcpListener, TcpStream},
    task::JoinSet,
    time::timeout,
};

/// Pairing can only start on this TLS listener with its fixed identity.
pub async fn serve_secure_tls(
    listener: TcpListener,
    host: Arc<Host>,
    identity: crate::tls::FixedIdentity,
    stop: impl Future<Output = ()>,
    on_accept_error: impl Fn(&io::Error),
) -> io::Result<()> {
    let binding = identity.digest();
    serve_accepting(
        || listener.accept(),
        host,
        tokio_rustls::TlsAcceptor::from(identity.config()),
        binding,
        stop,
        on_accept_error,
    )
    .await
}

pub(in crate::http) async fn serve_accepting<A, F>(
    mut accept: A,
    host: Arc<Host>,
    tls: tokio_rustls::TlsAcceptor,
    binding: [u8; 32],
    stop: impl Future<Output = ()>,
    on_accept_error: impl Fn(&io::Error),
) -> io::Result<()>
where
    A: FnMut() -> F,
    F: Future<Output = io::Result<(TcpStream, SocketAddr)>>,
{
    let mut connections = JoinSet::new();
    let mut peers: HashMap<tokio::task::Id, IpAddr> = HashMap::new();
    tokio::pin!(stop);
    let result = loop {
        tokio::select! {
            _ = &mut stop => break Ok(()),
            Some(done) = connections.join_next_with_id(), if !connections.is_empty() => {
                if let Some(id) = finished_id(&done) { peers.remove(&id); }
            },
            accepted = accept() => {
                let (socket, peer) = match accepted {
                    Ok(pair) => pair,
                    Err(error) if transient(&error) => {
                        on_accept_error(&error);
                        continue;
                    }
                    Err(error) => break Err(error),
                };
                // ? Clean up finished tasks before checking whether we're full; there's no queue of
                // ? waiting sockets.
                while let Some(done) = connections.try_join_next_with_id() {
                    if let Some(id) = finished_id(&done) { peers.remove(&id); }
                }
                let from_peer = peers.values().filter(|ip| **ip == peer.ip()).count();
                if connections.len() >= MAX_CONNECTIONS || from_peer >= MAX_CONNECTIONS_PER_PEER {
                    drop(socket);
                    continue;
                }
                let host = host.clone();
                let tls = tls.clone();
                let task = connections.spawn(async move {
                    let _ = timeout(CONNECTION_TIMEOUT, async move {
                        if let Ok(Ok(socket)) = timeout(HEADER_TIMEOUT, tls.accept(socket)).await {
                            serve_http(socket,host,binding).await;
                        }
                    }).await;
                });
                peers.insert(task.id(), peer.ip());
            }
        }
    };
    connections.abort_all();
    while connections.join_next().await.is_some() {}
    host.close_pairing();
    host.wait_pairing_idle().await;
    result
}

async fn serve_http<S>(socket: S, host: Arc<Host>, binding: [u8; 32])
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    let service = service_fn(move |request| {
        let host = host.clone();
        async move { Ok::<_, Infallible>(handle(request, &host, binding).await) }
    });
    let mut builder = http1::Builder::new();
    builder
        .keep_alive(false)
        .max_headers(32)
        .max_buf_size(8192)
        .timer(TokioTimer::new())
        .header_read_timeout(HEADER_TIMEOUT);
    let _ = builder
        .serve_connection(TokioIo::new(socket), service)
        .await;
}

async fn handle(request: Request<Incoming>, host: &Arc<Host>, binding: [u8; 32]) -> Reply {
    let (parts, body) = request.into_parts();
    if parts.uri.path() == PAIR_REQUEST_PATH {
        return pair_request::handle(parts, body, host).await;
    }
    if parts.uri.path().starts_with(pairing::PREFIX) {
        return pairing::handle(parts, body, host, binding).await;
    }
    let token = single(&parts.headers, "authorization").and_then(|v| v.strip_prefix("Bearer "));
    let Some((token, authenticated_device)) =
        token.and_then(|token| Some((token, host.authenticated_device(token)?)))
    else {
        let mut response = error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "Authentication failed.",
        );
        response
            .headers_mut()
            .insert("www-authenticate", HeaderValue::from_static("Bearer"));
        return response;
    };
    let Some(major) = single(&parts.headers, "x-myrig-protocol-major") else {
        return invalid();
    };
    if major.is_empty() || major.starts_with('0') || !major.bytes().all(|b| b.is_ascii_digit()) {
        return invalid();
    }
    if major != "1" {
        return reply(
            StatusCode::BAD_REQUEST,
            json!({
                "error": {
                    "code": "unsupported_protocol",
                    "message": "No supported protocol major matches this request.",
                },
                "supported_protocol_majors": [1],
            }),
        );
    }
    let path = parts.uri.path();
    let first = path.split('/').nth(1).unwrap_or("");
    if first
        .strip_prefix('v')
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) && n != "1")
    {
        return invalid();
    }
    let route = match path {
        STATUS_PATH => Route::Status,
        SHUTDOWN_PATH => Route::Shutdown { test: false },
        SHUTDOWN_TEST_PATH => Route::Shutdown { test: true },
        PAIRING_PATH => Route::Unpair,
        _ => return error(StatusCode::NOT_FOUND, "not_found", "Route not found."),
    };
    let (method, allow) = match route {
        Route::Status => (Method::GET, "GET"),
        Route::Shutdown { .. } => (Method::POST, "POST"),
        Route::Unpair => (Method::DELETE, "DELETE"),
    };
    if parts.method != method {
        let mut response = error(
            StatusCode::METHOD_NOT_ALLOWED,
            "method_not_allowed",
            "Method not allowed.",
        );
        response
            .headers_mut()
            .insert("allow", HeaderValue::from_static(allow));
        return response;
    }
    if parts.uri.query().is_some() || !accepts_json(&parts.headers) {
        return invalid();
    }
    if method == Method::POST && !json_media(&parts.headers) {
        return error(
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "unsupported_media_type",
            "Expected application/json.",
        );
    }
    let bytes = match read_body(body).await {
        Ok(bytes) => bytes,
        Err(BodyError::TooLarge) => {
            return error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "payload_too_large",
                "Request body exceeds limit.",
            );
        }
        Err(BodyError::Invalid) => return invalid(),
    };
    if method != Method::POST && !bytes.is_empty() {
        return invalid();
    }
    let test = match route {
        Route::Shutdown { test } => test,
        Route::Unpair => {
            // ? The save writes a protected file, so keep it off the runtime thread.
            let (owner, token) = (host.clone(), zeroize::Zeroizing::new(token.to_owned()));
            let revoked = tokio::task::spawn_blocking(move || owner.revoke_own(&token)).await;
            return match revoked {
                Ok(Ok(revoked)) => reply_body(StatusCode::OK, UnpairReply { revoked }),
                _ => error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "Host internal error.",
                ),
            };
        }
        Route::Status => return status(host),
    };
    shutdown::handle(host, &bytes, authenticated_device, test).await
}

fn status(host: &Host) -> Reply {
    match host.core.shutting_down(Instant::now()) {
        Ok(pending) => reply_body(
            StatusCode::OK,
            StatusReply {
                host_id: host.host_id.to_string(),
                app_version: env!("CARGO_PKG_VERSION").to_owned(),
                state: if pending {
                    HostState::ShuttingDown
                } else {
                    HostState::Online
                },
            },
        ),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "Host internal error.",
        ),
    }
}

fn finished_id(
    done: &Result<(tokio::task::Id, ()), tokio::task::JoinError>,
) -> Option<tokio::task::Id> {
    match done {
        Ok((id, ())) => Some(*id),
        Err(error) => Some(error.id()),
    }
}

/// An accept error that only ends one connection attempt, not the listener.
fn transient(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::Interrupted
    )
}
