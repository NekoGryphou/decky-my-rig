//! OPAQUE pairing: the host's approved session and the client's exchange.

use argon2::{Algorithm, Argon2, Params, Version};
use opaque_ke::{CipherSuite, CredentialResponse, Identifiers};
use sha2::{Digest, Sha256, Sha512};
pub mod client;
pub mod session;
pub const REQUEST_LEN: usize =
    <opaque_ke::CredentialRequestLen<Suite> as opaque_ke::generic_array::typenum::Unsigned>::USIZE;
pub const RESPONSE_LEN: usize =
    <opaque_ke::CredentialResponseLen<Suite> as opaque_ke::generic_array::typenum::Unsigned>::USIZE;
pub const CONFIRMATION_LEN: usize = <opaque_ke::CredentialFinalizationLen<Suite> as opaque_ke::generic_array::typenum::Unsigned>::USIZE;
/// Bigger TLS certificates are refused before any pairing work.
pub const MAX_CERTIFICATE_BYTES: usize = 65_536;

fn decode_response(bytes: &[u8]) -> Result<CredentialResponse<Suite>, ()> {
    if bytes.len() != RESPONSE_LEN {
        return Err(());
    }
    CredentialResponse::deserialize(bytes).map_err(|_| ())
}

fn decode_finalization(bytes: &[u8]) -> Result<opaque_ke::CredentialFinalization<Suite>, ()> {
    if bytes.len() != CONFIRMATION_LEN {
        return Err(());
    }
    opaque_ke::CredentialFinalization::deserialize(bytes).map_err(|_| ())
}

struct Suite;
impl CipherSuite for Suite {
    type OprfCs = opaque_ke::Ristretto255;
    type KeyExchange = opaque_ke::TripleDh<opaque_ke::Ristretto255, Sha512>;
    type Ksf = Argon2<'static>;
}

fn stretch() -> Argon2<'static> {
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(19_456, 2, 1, None)
            .expect("constant Argon2id parameters are within the crate's limits"),
    )
}

// ! Each side uses the certificate it saw on its own TLS connection, never a
//   fingerprint the other side sent. The exchange fails unless both saw the same
//   one.
fn context(certificate_der: &[u8], session: &[u8; 32]) -> Vec<u8> {
    let mut result = b"DeckyMyRig/opaque-bootstrap/v1\0".to_vec();
    result.extend_from_slice(&Sha256::digest(certificate_der));
    result.extend_from_slice(session);
    result
}

fn identities() -> Identifiers<'static> {
    Identifiers {
        client: Some(b"DeckyMyRig/bootstrap-client/v1"),
        server: Some(b"DeckyMyRig/bootstrap-host/v1"),
    }
}

#[cfg(test)]
mod tests;
