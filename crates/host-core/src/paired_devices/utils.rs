use myrig_protocol::validation::decode_hex;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// SHA-256 of a bearer token's 32 bytes, or None unless the token is exactly 64
/// lowercase hex digits.
pub(super) fn token_digest(token: &str) -> Option<[u8; 32]> {
    decode_hex::<32>(token).map(|decoded| Sha256::digest(decoded).into())
}

pub(super) fn digests_match(actual: &[u8; 32], expected: &[u8; 32]) -> bool {
    actual.ct_eq(expected).into()
}
