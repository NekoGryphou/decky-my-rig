use rustls::crypto::CryptoProvider;
use std::sync::{Arc, Mutex};

/// Accepts the leaf certificate whose SHA-256 matches the pin (any leaf when
/// there's no pin yet, for pairing) and remembers the certificate it saw. The
/// TLS handshake still proves the peer has the private key.
#[derive(Debug)]
pub(in crate::host_client) struct PinVerifier {
    pub(super) pin: Option<[u8; 32]>,
    pub(super) provider: Arc<CryptoProvider>,
    pub(super) observed: Mutex<Option<Vec<u8>>>,
}
