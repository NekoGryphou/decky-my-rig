use super::consts::SECURITY_FILE;
use crate::security_bundle::{HostSecurity, MAX_SECURITY_BYTES};
use std::{fs::File, io::Read, path::Path};
use uuid::Uuid;

/// Reads the TLS identity installed for `expected_host`.
pub fn load_security(directory: &Path, expected_host: Uuid) -> Result<HostSecurity, &'static str> {
    let bytes = read_bounded(&directory.join(SECURITY_FILE), MAX_SECURITY_BYTES)
        .ok_or("Host security material is unavailable.")?;
    HostSecurity::from_trusted_json(&bytes, expected_host)
}

/// The file's contents, or None if it can't be read or is bigger than `limit`.
pub(super) fn read_bounded(path: &Path, limit: usize) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)
        .ok()?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() <= limit).then_some(bytes)
}
