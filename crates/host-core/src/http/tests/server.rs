use hyper::header::{HeaderMap, HeaderValue};
use sha2::{Digest, Sha256};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::net::TcpListener;

use super::super::{consts::*, fixtures::empty_registry_host, server::serve_accepting, utils::*};
use super::fixtures::*;

#[test]
fn accept_quality_and_media_type() {
    let mut headers = HeaderMap::new();
    for (value, expected) in [
        ("application/json", true),
        ("*/*", true),
        ("text/plain", false),
        ("application/json;q=0,*/*;q=1", false),
        ("application/json;q=0.5", true),
        ("application/json;q=NaN", false),
    ] {
        headers.insert("accept", value.parse().unwrap());
        assert_eq!(accepts_json(&headers), expected);
    }
    headers.insert(
        "content-type",
        HeaderValue::from_static("application/json; charset=UTF-8"),
    );
    assert!(json_media(&headers));
}

#[tokio::test]
async fn transient_accept_error_is_reported_and_next_tls_status_is_served() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let host = empty_registry_host();
    {
        let mut config = host.config.write().unwrap();
        let mut registry = config.clone();
        registry
            .insert_credential(device(1), "Device", Sha256::digest([1u8; 32]).into())
            .unwrap();
        *config = registry;
    }
    let generated = rcgen::generate_simple_self_signed(vec!["myrig.local".to_owned()]).unwrap();
    let mut roots = rustls::RootCertStore::empty();
    roots.add(generated.cert.der().clone()).unwrap();
    let mut client_config = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    client_config.alpn_protocols = vec![b"http/1.1".to_vec()];
    let identity = crate::tls::FixedIdentity::from_der(
        vec![generated.cert.der().to_vec()],
        generated.signing_key.serialize_der(),
    )
    .unwrap();
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let errors = Arc::new(Mutex::new(Vec::new()));
    let recorded = errors.clone();
    let server = tokio::spawn(async move {
        let listener_ref = &listener;
        let mut failures = std::collections::VecDeque::from([
            std::io::ErrorKind::ConnectionAborted,
            std::io::ErrorKind::ConnectionReset,
            std::io::ErrorKind::Interrupted,
        ]);
        serve_accepting(
            move || {
                let fail = failures.pop_front();
                async move {
                    if let Some(kind) = fail {
                        Err(kind.into())
                    } else {
                        listener_ref.accept().await
                    }
                }
            },
            host,
            tokio_rustls::TlsAcceptor::from(identity.config()),
            identity.digest(),
            async {
                let _ = stop_rx.await;
            },
            move |error| {
                recorded.lock().unwrap().push(error.kind());
            },
        )
        .await
    });
    let response = tokio::time::timeout(Duration::from_secs(2), async {
        let socket = tokio::net::TcpStream::connect(address).await?;
        let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
        let mut stream = connector.connect(rustls::pki_types::ServerName::try_from("myrig.local").unwrap(), socket).await?;
        let request = format!("GET /v1/status HTTP/1.1\r\nHost: myrig.local\r\nAuthorization: Bearer {}\r\nX-MyRig-Protocol-Major: 1\r\nAccept: application/json\r\nConnection: close\r\n\r\n", "01".repeat(32));
        stream.write_all(request.as_bytes()).await?;
        let mut headers = Vec::new();
        let mut buffer = [0u8; 1024];
        while !headers.windows(4).any(|part| part == b"\r\n\r\n") {
            let count = stream.read(&mut buffer).await?;
            if count == 0 { return Err(std::io::ErrorKind::UnexpectedEof.into()); }
            headers.extend_from_slice(&buffer[..count]);
            if headers.len() > 8192 { return Err(std::io::ErrorKind::InvalidData.into()); }
        }
        Ok::<_,std::io::Error>(headers)
    }).await;
    let _ = stop_tx.send(());
    let served = server.await.unwrap();
    assert!(
        served.is_ok(),
        "per-connection error stopped the listener: {served:?}"
    );
    assert!(
        response
            .unwrap()
            .unwrap()
            .starts_with(b"HTTP/1.1 200 OK\r\n")
    );
    assert_eq!(
        *errors.lock().unwrap(),
        vec![
            std::io::ErrorKind::ConnectionAborted,
            std::io::ErrorKind::ConnectionReset,
            std::io::ErrorKind::Interrupted
        ]
    );
}

#[tokio::test]
async fn fatal_accept_error_returns_to_owner_without_retry() {
    let generated = rcgen::generate_simple_self_signed(vec!["myrig.local".to_owned()]).unwrap();
    let identity = crate::tls::FixedIdentity::from_der(
        vec![generated.cert.der().to_vec()],
        generated.signing_key.serialize_der(),
    )
    .unwrap();
    let attempts = std::cell::Cell::new(0);
    let result = serve_accepting(
        || {
            attempts.set(attempts.get() + 1);
            assert_eq!(attempts.get(), 1, "fatal listener failure was retried");
            std::future::ready(Err::<(tokio::net::TcpStream, std::net::SocketAddr), _>(
                std::io::Error::from(std::io::ErrorKind::PermissionDenied),
            ))
        },
        empty_registry_host(),
        tokio_rustls::TlsAcceptor::from(identity.config()),
        identity.digest(),
        std::future::pending(),
        |_| panic!("fatal listener errors belong to the lifecycle owner"),
    )
    .await;
    assert_eq!(
        result.unwrap_err().kind(),
        std::io::ErrorKind::PermissionDenied
    );
    assert_eq!(attempts.get(), 1);
}

#[tokio::test]
async fn one_peer_cannot_use_every_connection_slot() {
    use tokio::io::AsyncReadExt;
    let host = empty_registry_host();
    let generated = rcgen::generate_simple_self_signed(vec!["myrig.local".to_owned()]).unwrap();
    let identity = crate::tls::FixedIdentity::from_der(
        vec![generated.cert.der().to_vec()],
        generated.signing_key.serialize_der(),
    )
    .unwrap();
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let address = listener.local_addr().unwrap();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let server = tokio::spawn(async move {
        let listener_ref = &listener;
        serve_accepting(
            move || listener_ref.accept(),
            host,
            tokio_rustls::TlsAcceptor::from(identity.config()),
            identity.digest(),
            async {
                let _ = stop_rx.await;
            },
            |_| {},
        )
        .await
    });
    // - Idle connections that never send a TLS hello keep their slots.
    let mut held = Vec::new();
    for _ in 0..MAX_CONNECTIONS_PER_PEER {
        held.push(tokio::net::TcpStream::connect(address).await.unwrap());
    }
    let mut extra = tokio::net::TcpStream::connect(address).await.unwrap();
    let mut byte = [0u8; 1];
    let refused = tokio::time::timeout(Duration::from_secs(1), extra.read(&mut byte)).await;
    assert!(matches!(refused, Ok(Ok(0) | Err(_))), "{refused:?}");
    // - The refusal above shows the server dealt with the extra connection. The
    //   held connection is still open, with nothing to read.
    let still_open = held[0].try_read(&mut byte);
    assert!(
        matches!(&still_open, Err(error) if error.kind() == std::io::ErrorKind::WouldBlock),
        "{still_open:?}"
    );
    stop_tx.send(()).unwrap();
    server.await.unwrap().unwrap();
}
