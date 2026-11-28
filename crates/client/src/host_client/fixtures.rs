//! A real host service on loopback and a scripted TLS peer for client tests.
use myrig_host_core::{
    host::Host,
    http::serve_secure_tls,
    paired_devices::RegistryConfig,
    shutdown::{Core, Power, PowerError},
    tls::FixedIdentity,
};
use myrig_protocol::validation::lower_hex;
use sha2::{Digest, Sha256};
use std::{
    net::SocketAddr,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

pub(crate) const HOST_ID: &str = "03c85317-9d93-4fb3-a04b-91a0e16668c6";

/// The bearer token whose 32 bytes all equal `byte`.
pub(crate) fn token(byte: u8) -> String {
    lower_hex(&[byte; 32])
}

pub(crate) fn identity() -> FixedIdentity {
    let generated = rcgen::generate_simple_self_signed(vec!["myrig.local".to_owned()]).unwrap();
    FixedIdentity::from_der(
        vec![generated.cert.der().to_vec()],
        generated.signing_key.serialize_der(),
    )
    .unwrap()
}

struct CountingPower(Arc<AtomicUsize>);

impl Power for CountingPower {
    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn cancel_shutdown(&self) -> Result<(), PowerError> {
        Ok(())
    }
}

/// A host whose registry holds the devices with tokens `token(1)` and `token(2)`.
pub(crate) struct RealHost {
    pub(crate) address: SocketAddr,
    pub(crate) pin: [u8; 32],
    pub(crate) power_calls: Arc<AtomicUsize>,
}

pub(crate) async fn real_host(test_mode: bool) -> RealHost {
    real_host_with(test_mode, None).await
}

/// Like `real_host`, with a pairing code already shown when `code` is set.
pub(crate) async fn real_host_with(test_mode: bool, code: Option<&str>) -> RealHost {
    let digest = |byte: u8| lower_hex(&Sha256::digest([byte; 32]));
    let config = serde_json::json!({"schema_version":3,"host_id":HOST_ID,"port":47991,
        "test_mode":test_mode,"paired_devices":[
        {"device_id":"11111111-1111-4111-8111-111111111111","name":"Device","token_sha256":digest(1)},
        {"device_id":"22222222-2222-4222-8222-222222222222","name":"Device","token_sha256":digest(2)}]});
    let power_calls = Arc::new(AtomicUsize::new(0));
    let host = Arc::new(Host::new(
        RegistryConfig::from_json(&serde_json::to_vec(&config).unwrap()).unwrap(),
        Core::new(Box::new(CountingPower(power_calls.clone()))),
        "Fixture PC".into(),
        Arc::new(|_| Ok(())),
    ));
    let identity = identity();
    let pin = identity.digest();
    if let Some(code) = code {
        host.open_secure_pairing(
            host.pairing_approval(),
            "Steam Deck",
            code,
            [3; 32],
            identity.leaf_der(),
            &[8; 32],
            uuid::Uuid::new_v4(),
            Arc::new(|_| Ok(())),
        )
        .await
        .unwrap();
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    tokio::spawn(serve_secure_tls(
        listener,
        host,
        identity,
        std::future::pending(),
        |_| {},
    ));
    RealHost {
        address,
        pin,
        power_calls,
    }
}

/// A TLS peer that answers every request with `reply` (or never answers),
/// counting completed handshakes and received requests.
pub(crate) struct ScriptedPeer {
    pub(crate) address: SocketAddr,
    pub(crate) pin: [u8; 32],
    pub(crate) certificate: Vec<u8>,
    pub(crate) requests: Arc<AtomicUsize>,
}

pub(crate) async fn scripted_peer(reply: Option<Vec<u8>>, alpn: bool) -> ScriptedPeer {
    let identity = identity();
    let mut config = (*identity.config()).clone();
    if !alpn {
        config.alpn_protocols.clear();
    }
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let counter = requests.clone();
    tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let (acceptor, counter, reply) = (acceptor.clone(), counter.clone(), reply.clone());
            tokio::spawn(async move {
                let Ok(mut stream) = acceptor.accept(socket).await else {
                    return;
                };
                let mut buffer = [0u8; 4096];
                if !matches!(stream.read(&mut buffer).await, Ok(read) if read > 0) {
                    return;
                }
                counter.fetch_add(1, Ordering::SeqCst);
                match reply {
                    Some(reply) => {
                        let _ = stream.write_all(&reply).await;
                        let _ = stream.shutdown().await;
                    }
                    None => std::future::pending::<()>().await,
                }
            });
        }
    });
    ScriptedPeer {
        address,
        pin: identity.digest(),
        certificate: identity.leaf_der().to_vec(),
        requests,
    }
}

/// An HTTP response carrying `body` as JSON after the given extra headers.
pub(crate) fn http_reply(status: &str, extra_headers: &str, body: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\n{extra_headers}connection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}
