//! A TLS peer running a real OPAQUE host session, where a test can edit the
//! replies. It records every request it gets.
use crate::host_client::fixtures::identity;
use myrig_pairing_crypto::session::{ApprovedSession, AttemptId};
use myrig_protocol::{
    lan::{PAIR_FINISH_PATH, PAIR_INFO_PATH, PAIR_START_PATH},
    validation::{decode_hex, decode_hex_bytes, lower_hex},
};
use serde_json::{Value, json};
use std::{
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

pub(crate) const PEER_HOST_ID: &str = "44444444-4444-4444-8444-444444444444";
const NONCE: [u8; 32] = [9; 32];

/// Edits a reply before it is sent; returns false to drop `cache-control`.
pub(crate) type Edit = fn(&str, &mut Value) -> bool;

pub(crate) struct PairingPeer {
    pub(crate) address: SocketAddr,
    pub(crate) pin: [u8; 32],
    /// (path, JSON body) of each request, in order.
    pub(crate) requests: Arc<Mutex<Vec<(String, Value)>>>,
}

impl PairingPeer {
    pub(crate) fn paths(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .map(|(path, _)| path.clone())
            .collect()
    }
}

pub(crate) fn unchanged(_: &str, _: &mut Value) -> bool {
    true
}

pub(crate) async fn pairing_peer(code: &str, edit: Edit) -> PairingPeer {
    pairing_peer_busy_once(code, edit, &[]).await
}

/// A peer that first answers each path in `busy` with 503, like a host whose
/// code is briefly held by another request.
pub(crate) async fn pairing_peer_busy_once(
    code: &str,
    edit: Edit,
    busy: &[&'static str],
) -> PairingPeer {
    spawn_peer(code, edit, busy, false).await
}

/// A peer whose every device slot is taken: the finish request gets 409 with
/// `device_limit_reached`, as the host sends it.
pub(crate) async fn pairing_peer_at_device_limit(code: &str) -> PairingPeer {
    spawn_peer(code, unchanged, &[], true).await
}

async fn spawn_peer(code: &str, edit: Edit, busy: &[&'static str], at_limit: bool) -> PairingPeer {
    let busy = Arc::new(Mutex::new(busy.to_vec()));
    let identity = identity();
    let session = Arc::new(Mutex::new(
        ApprovedSession::prepare(code.as_bytes(), &NONCE, identity.leaf_der()).unwrap(),
    ));
    let acceptor = tokio_rustls::TlsAcceptor::from(identity.config());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(Mutex::new(Vec::new()));
    let recorded = requests.clone();
    tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let Ok(mut stream) = acceptor.accept(socket).await else {
                continue;
            };
            let (path, body) = read_request(&mut stream).await;
            recorded.lock().unwrap().push((path.clone(), body.clone()));
            let held = {
                let mut busy = busy.lock().unwrap();
                let held = busy.contains(&path.as_str());
                busy.retain(|busy| *busy != path);
                held
            };
            let (status, mut value) = if at_limit && path == PAIR_FINISH_PATH {
                (
                    "409 Conflict",
                    json!({"protocol_version":{"major":1,"minor":0},
                        "error":{"code":"device_limit_reached","message":"Device limit reached."}}),
                )
            } else if held {
                (
                    "503 Service Unavailable",
                    json!({"protocol_version":{"major":1,"minor":0},
                        "error":{"code":"pairing_busy","message":"Pairing is busy."}}),
                )
            } else {
                answer(&session, &path, &body)
            };
            let no_store = edit(&path, &mut value);
            let text = value.to_string();
            let reply = format!(
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\n{}content-length: {}\r\nconnection: close\r\n\r\n{text}",
                if no_store {
                    "cache-control: no-store\r\n"
                } else {
                    ""
                },
                text.len()
            );
            let _ = stream.write_all(reply.as_bytes()).await;
            let _ = stream.shutdown().await;
        }
    });
    PairingPeer {
        address,
        pin: identity.digest(),
        requests,
    }
}

async fn read_request<S: AsyncReadExt + Unpin>(stream: &mut S) -> (String, Value) {
    let mut data = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let read = stream.read(&mut buffer).await.unwrap_or(0);
        data.extend_from_slice(&buffer[..read]);
        let text = String::from_utf8_lossy(&data).into_owned();
        if let Some(end) = text.find("\r\n\r\n") {
            let length = text[..end]
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|v| v.trim().parse::<usize>().unwrap())
                })
                .unwrap_or(0);
            if read == 0 || data.len() >= end + 4 + length {
                let path = text.split(' ').nth(1).unwrap_or("").to_owned();
                let body = serde_json::from_slice(&data[end + 4..]).unwrap_or(Value::Null);
                return (path, body);
            }
        }
        if read == 0 {
            return (String::new(), Value::Null);
        }
    }
}

fn answer(session: &Mutex<ApprovedSession>, path: &str, body: &Value) -> (&'static str, Value) {
    let version = json!({"major":1,"minor":0});
    let mut session = session.lock().unwrap();
    let text = |name: &str| {
        body.get(name)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    match path {
        PAIR_INFO_PATH => (
            "200 OK",
            json!({"protocol_version":version,"bootstrap_version":1,"nonce":lower_hex(&NONCE)}),
        ),
        PAIR_START_PATH => {
            match decode_hex_bytes(&text("request"), 96).map(|request| session.start(&request)) {
                Some(Ok(challenge)) => (
                    "200 OK",
                    json!({"protocol_version":version,"attempt_id":lower_hex(challenge.attempt.as_bytes()),
                    "session_nonce":lower_hex(&NONCE),"response":lower_hex(&challenge.response)}),
                ),
                _ => ("403 Forbidden", json!({"protocol_version":version})),
            }
        }
        PAIR_FINISH_PATH => {
            let attempt = decode_hex::<32>(&text("attempt_id")).map(AttemptId::from_bytes);
            let confirmation = decode_hex_bytes(&text("confirmation"), 64);
            match attempt
                .zip(confirmation)
                .map(|(id, c)| session.finish(id, &c))
            {
                Some(Ok(_)) => (
                    "200 OK",
                    json!({"protocol_version":version,"host_id":PEER_HOST_ID,
                        "token":lower_hex(&[0xab; 32]),"computer_name":"Peer PC"}),
                ),
                _ => ("403 Forbidden", json!({"protocol_version":version})),
            }
        }
        _ => ("404 Not Found", json!({"protocol_version":version})),
    }
}
