use std::sync::Arc;

/// The fixed identity and the right to start pairing are built together, so no
/// caller can swap in another config or fingerprint.
#[derive(Clone)]
pub struct FixedIdentity {
    pub(super) config: Arc<rustls::ServerConfig>,
    pub(super) leaf_der: Vec<u8>,
    pub(super) digest: [u8; 32],
}
