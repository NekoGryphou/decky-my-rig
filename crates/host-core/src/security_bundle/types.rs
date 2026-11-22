use crate::tls::FixedIdentity;
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StoredSecurity {
    pub(super) schema_version: u32,
    pub(super) host_id: String,
    pub(super) certificate_der: String,
    pub(super) private_key_pkcs8: SecretString,
}

/// The same record as `StoredSecurity`, borrowed for writing a new identity.
#[derive(Serialize)]
pub(super) struct NewSecurity<'a> {
    pub(super) schema_version: u32,
    pub(super) host_id: &'a str,
    pub(super) certificate_der: &'a str,
    pub(super) private_key_pkcs8: &'a str,
}

pub(super) struct SecretString(pub(super) Zeroizing<String>);

/// The caller has to make sure the bundle came from admin provisioning and
/// protected storage; parsing doesn't make network data trustworthy. There's
/// deliberately no Clone, Debug, serialization or way to get the key back out.
pub struct HostSecurity {
    pub(super) identity: FixedIdentity,
}
