use super::{
    consts::{DEFAULT_PORT, SCHEMA_VERSION},
    messages::{NOT_PRIVATE, OPEN_FAILED, READ_FAILED, TOO_LARGE},
    types::{
        Authentication, PcChanges, PcConfig, PcInput, StoredAuthentication, StoredDocument,
        StoredPc,
    },
};
use crate::shared::{PublicError, normalize_host, normalize_mac, normalize_name, normalize_port};
use myrig_protocol::validation::{canonical_uuid, decode_hex, lower_hex};
use std::{collections::HashSet, io};
use uuid::Uuid;
use zeroize::Zeroizing;

pub(super) fn create(
    input: PcInput,
    authentication: Authentication,
) -> Result<PcConfig, PublicError> {
    Ok(PcConfig {
        id: Uuid::new_v4(),
        name: normalize_name(&input.name)?,
        host: normalize_host(&input.host)?,
        port: normalize_port(input.port.as_deref().unwrap_or(DEFAULT_PORT))?,
        mac: normalize_mac(&input.mac)?,
        authentication,
    })
}

pub(super) fn apply(current: &PcConfig, changes: PcChanges) -> Result<PcConfig, PublicError> {
    Ok(PcConfig {
        id: current.id,
        name: changes
            .name
            .map_or(Ok(current.name.clone()), |name| normalize_name(&name))?,
        host: changes
            .host
            .map_or(Ok(current.host), |host| normalize_host(&host))?,
        port: changes
            .port
            .map_or(Ok(current.port), |port| normalize_port(&port))?,
        mac: changes
            .mac
            .map_or(Ok(current.mac.clone()), |mac| normalize_mac(&mac))?,
        authentication: changes
            .authentication
            .unwrap_or_else(|| current.authentication.clone()),
    })
}

/// The whole document, or `None` if it isn't exactly what this client writes.
/// Corrupt data is refused, never repaired or reset.
pub(super) fn decode(raw: &[u8]) -> Option<Vec<PcConfig>> {
    let document: StoredDocument = serde_json::from_slice(raw).ok()?;
    if document.schema_version != SCHEMA_VERSION {
        return None;
    }
    let pcs = document
        .pcs
        .into_iter()
        .map(from_stored)
        .collect::<Option<Vec<_>>>()?;
    let mut identifiers = HashSet::new();
    pcs.iter()
        .all(|pc| identifiers.insert(pc.id))
        .then_some(pcs)
}

/// The document's bytes, which hold every token, in a buffer that wipes itself.
pub(super) fn encode(pcs: &[PcConfig]) -> serde_json::Result<Zeroizing<Vec<u8>>> {
    serde_json::to_vec(&StoredDocument {
        schema_version: SCHEMA_VERSION,
        pcs: pcs.iter().map(to_stored).collect(),
    })
    .map(Zeroizing::new)
}

fn from_stored(stored: StoredPc) -> Option<PcConfig> {
    let pc = PcConfig {
        id: canonical_uuid(&stored.id)?,
        name: normalize_name(&stored.name).ok()?,
        host: normalize_host(&stored.host).ok()?,
        port: normalize_port(&stored.port.to_string()).ok()?,
        mac: normalize_mac(&stored.mac).ok()?,
        authentication: Authentication {
            host_id: canonical_uuid(&stored.authentication.host_id)?,
            token: Zeroizing::new(stored.authentication.token.clone()),
            tls_pin: decode_hex::<32>(&stored.authentication.tls_pin)?,
        },
    };
    decode_hex::<32>(&pc.authentication.token)?;
    let canonical = to_stored(&pc);
    (canonical.name == stored.name && canonical.host == stored.host && canonical.mac == stored.mac)
        .then_some(pc)
}

fn to_stored(pc: &PcConfig) -> StoredPc {
    StoredPc {
        id: pc.id.to_string(),
        name: pc.name.clone(),
        host: pc.host.to_string(),
        port: pc.port,
        mac: pc.mac.clone(),
        authentication: StoredAuthentication {
            host_id: pc.authentication.host_id.to_string(),
            token: pc.authentication.token.to_string(),
            tls_pin: lower_hex(&pc.authentication.tls_pin),
        },
    }
}

/// What a failed open of the settings directory means for the user.
pub(super) fn open_error(error: io::Error) -> PublicError {
    match error.kind() {
        io::ErrorKind::PermissionDenied => PublicError(NOT_PRIVATE),
        _ => PublicError(OPEN_FAILED),
    }
}

/// What a failed read of `pcs.json` means: a file that isn't private, one past
/// the size limit, or plain I/O.
pub(super) fn read_error(error: io::Error) -> PublicError {
    match error.kind() {
        io::ErrorKind::PermissionDenied => PublicError(NOT_PRIVATE),
        io::ErrorKind::InvalidData => PublicError(TOO_LARGE),
        _ => PublicError(READ_FAILED),
    }
}
