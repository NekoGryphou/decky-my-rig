use super::{
    consts::{MAX_KEY_BYTES, MAX_SECURITY_BYTES, MAX_STORED_CERTIFICATE_BYTES, SCHEMA_VERSION},
    messages::FAILURE,
    types::{HostSecurity, NewSecurity, SecretString, StoredSecurity},
};
use crate::tls::FixedIdentity;
use myrig_protocol::validation::{canonical_uuid, decode_hex_bytes, lower_hex};
use serde::{Deserialize, Deserializer};
use uuid::Uuid;
use zeroize::Zeroizing;

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D: Deserializer<'de>>(value: D) -> Result<Self, D::Error> {
        String::deserialize(value).map(|value| Self(Zeroizing::new(value)))
    }
}

impl HostSecurity {
    pub fn from_trusted_json(bytes: &[u8], expected_host: Uuid) -> Result<Self, &'static str> {
        if bytes.is_empty() || bytes.len() > MAX_SECURITY_BYTES {
            return Err(FAILURE);
        }
        let stored: StoredSecurity = serde_json::from_slice(bytes).map_err(|_| FAILURE)?;
        if stored.schema_version != SCHEMA_VERSION
            || canonical_uuid(&stored.host_id) != Some(expected_host)
        {
            return Err(FAILURE);
        }
        let certificate = decode_hex_bytes(&stored.certificate_der, MAX_STORED_CERTIFICATE_BYTES)
            .ok_or(FAILURE)?;
        let mut key =
            decode_hex_bytes(&stored.private_key_pkcs8.0, MAX_KEY_BYTES).ok_or(FAILURE)?;
        // ! Move the only decoded copy of the key into rustls's owned secret type. The text copies
        // ! along the way are wiped on every return path.
        let identity =
            FixedIdentity::from_der(vec![certificate.to_vec()], std::mem::take(&mut *key))
                .map_err(|_| FAILURE)?;
        Ok(Self { identity })
    }

    pub fn into_identity(self) -> FixedIdentity {
        self.identity
    }
}

/// The protected security file for a new host identity, wiped on drop.
pub fn security_json(
    host_id: Uuid,
    certificate_der: &[u8],
    private_key_pkcs8: &[u8],
) -> Result<Zeroizing<Vec<u8>>, &'static str> {
    let key = Zeroizing::new(lower_hex(private_key_pkcs8));
    let record = NewSecurity {
        schema_version: SCHEMA_VERSION,
        host_id: &host_id.to_string(),
        certificate_der: &lower_hex(certificate_der),
        private_key_pkcs8: &key,
    };
    serde_json::to_vec(&record)
        .map(Zeroizing::new)
        .map_err(|_| "Cannot serialize TLS identity.")
}
