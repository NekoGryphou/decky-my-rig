use super::consts::{FORBIDDEN, TOP_LEVEL_FILES};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};

/// Packages reachable from the one at `manifest` through non-dev dependencies.
/// Test-only crates never end up in the binary.
pub(super) fn crate_dependencies<'a>(
    metadata: &'a Value,
    manifest: &Path,
) -> Result<Vec<&'a Value>, String> {
    const INVALID: &str = "unexpected cargo metadata";
    let packages: HashMap<&str, &Value> = metadata["packages"]
        .as_array()
        .ok_or(INVALID)?
        .iter()
        .filter_map(|package| Some((package["id"].as_str()?, package)))
        .collect();
    let nodes: HashMap<&str, &Value> = metadata["resolve"]["nodes"]
        .as_array()
        .ok_or(INVALID)?
        .iter()
        .filter_map(|node| Some((node["id"].as_str()?, node)))
        .collect();
    let roots: Vec<&str> = packages
        .iter()
        .filter(|(_, package)| package["manifest_path"].as_str().map(Path::new) == Some(manifest))
        .map(|(id, _)| *id)
        .collect();
    if roots.len() != 1 {
        return Err("client package missing from cargo metadata".to_owned());
    }
    let mut pending = roots;
    let mut included = HashSet::new();
    while let Some(id) = pending.pop() {
        if !included.insert(id) {
            continue;
        }
        let deps = nodes
            .get(id)
            .and_then(|node| node["deps"].as_array())
            .ok_or(INVALID)?;
        for dependency in deps {
            let kinds = dependency["dep_kinds"].as_array();
            let shipped = kinds.is_none_or(|kinds| {
                kinds.is_empty()
                    || kinds
                        .iter()
                        .any(|kind| kind["kind"].as_str() != Some("dev"))
            });
            if shipped {
                pending.push(dependency["pkg"].as_str().ok_or(INVALID)?);
            }
        }
    }
    included
        .into_iter()
        .map(|id| packages.get(id).copied().ok_or_else(|| INVALID.to_owned()))
        .collect()
}

/// Little-endian ELF64 for `machine` with no PT_INTERP, so the device doesn't
/// need a system loader or a particular C library.
pub(super) fn is_static_elf(data: &[u8], machine: u16) -> bool {
    const PT_INTERP: u32 = 3;
    let u16_at = |at: usize| {
        data.get(at..at + 2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
    };
    let u32_at = |at: usize| {
        data.get(at..at + 4)
            .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    };
    let u64_at = |at: usize| {
        data.get(at..at + 8)
            .and_then(|b| b.try_into().ok())
            .map(u64::from_le_bytes)
    };
    if data.len() < 64 || !data.starts_with(b"\x7fELF\x02\x01") || u16_at(18) != Some(machine) {
        return false;
    }
    let (Some(offset), Some(size), Some(count)) = (u64_at(32), u16_at(54), u16_at(56)) else {
        return false;
    };
    let (Ok(offset), size, count) = (
        usize::try_from(offset),
        usize::from(size),
        usize::from(count),
    ) else {
        return false;
    };
    if size < 4 || offset.saturating_add(size * count) > data.len() {
        return false;
    }
    (0..count).all(|index| u32_at(offset + size * index) != Some(PT_INTERP))
}

/// A PE executable for `machine`: "MZ", then "PE\0\0" at the offset the DOS
/// header gives, then the machine type.
pub(super) fn is_pe(data: &[u8], machine: u16) -> bool {
    let Some(offset) = data
        .get(0x3c..0x40)
        .and_then(|bytes| bytes.try_into().ok())
        .map(u32::from_le_bytes)
        .and_then(|offset| usize::try_from(offset).ok())
    else {
        return false;
    };
    data.starts_with(b"MZ")
        && data.get(offset..offset + 4) == Some(b"PE\0\0".as_slice())
        && data
            .get(offset + 4..offset + 6)
            .is_some_and(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) == machine)
}

/// The fixed plugin files plus every adapter module, so none gets forgotten.
pub(super) fn release_files(source: &Path) -> Result<Vec<String>, String> {
    let mut modules = Vec::new();
    let mut pending = vec![source.join("py_modules/my_rig")];
    while let Some(directory) = pending.pop() {
        let entries = std::fs::read_dir(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        for entry in entries {
            let path = entry.map_err(|error| error.to_string())?.path();
            if path.is_dir() {
                if path.file_name() != Some("__pycache__".as_ref()) {
                    pending.push(path);
                }
            } else if path.extension() == Some("py".as_ref()) {
                let relative = path
                    .strip_prefix(source)
                    .map_err(|error| error.to_string())?;
                let relative = relative.to_str().ok_or("non-UTF-8 module path")?;
                modules.push(relative.replace('\\', "/"));
            }
        }
    }
    modules.sort();
    Ok(TOP_LEVEL_FILES
        .iter()
        .map(|file| (*file).to_owned())
        .chain(modules)
        .collect())
}

pub(super) fn reject_credentials(relative: &str, data: &[u8]) -> Result<(), String> {
    let text = String::from_utf8_lossy(data);
    if FORBIDDEN.iter().any(|value| text.contains(value)) {
        return Err(format!(
            "credential-like material in release file: {relative}"
        ));
    }
    if has_credential_url(&text.to_ascii_lowercase()) {
        return Err(format!(
            "credential-bearing URL in release file: {relative}"
        ));
    }
    Ok(())
}

/// `http(s)://user:password@` anywhere in lowercase `text`.
fn has_credential_url(text: &str) -> bool {
    let part = |rest: &str, stops: &str| {
        rest.find(|c: char| c.is_whitespace() || stops.contains(c))
            .unwrap_or(rest.len())
    };
    text.match_indices("://").any(|(at, _)| {
        if !(text[..at].ends_with("http") || text[..at].ends_with("https")) {
            return false;
        }
        let rest = &text[at + 3..];
        let user = part(rest, "/:@");
        if user == 0 || !rest[user..].starts_with(':') {
            return false;
        }
        let rest = &rest[user + 1..];
        let password = part(rest, "/@");
        password != 0 && rest[password..].starts_with('@')
    })
}

pub(super) fn valid_version(version: &str) -> bool {
    !version.is_empty()
        && version
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".+-".contains(c))
}

pub(super) fn hex_sha256(data: &[u8]) -> String {
    Sha256::digest(data)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// A deflated archive with fixed timestamps and sorted entries under `root`,
/// so the same payload always gives the same bytes. Plain ZIP only: a payload
/// that would need ZIP64 (65,535 entries or 4 GiB) is refused.
pub(super) fn deterministic_zip(
    root: &str,
    payload: &BTreeMap<String, Vec<u8>>,
) -> Result<Vec<u8>, String> {
    let too_large = || "the plugin archive is beyond plain ZIP limits".to_owned();
    // - 1980-01-01 00:00, the earliest DOS date.
    const DATE: u16 = (1 << 5) | 1;
    const VERSION: u16 = 20;
    const UNIX: u16 = 3 << 8;
    const DEFLATE: u16 = 8;
    let mut archive = Vec::new();
    let mut central = Vec::new();
    for (relative, data) in payload {
        let name = format!("{root}{relative}");
        let compressed = miniz_oxide::deflate::compress_to_vec(data, 6);
        let crc = crc32fast::hash(data);
        let mode: u32 = if relative.starts_with("bin/myrig-") {
            0o100755
        } else {
            0o100644
        };
        let offset = u32::try_from(archive.len()).map_err(|_| too_large())?;
        let sizes = [compressed.len(), data.len()].map(u32::try_from);
        let [Ok(packed), Ok(size)] = sizes else {
            return Err(too_large());
        };
        let name_length = u16::try_from(name.len()).map_err(|_| too_large())?;
        let shared = |out: &mut Vec<u8>| {
            for value in [VERSION, 0, DEFLATE, 0, DATE] {
                out.extend(value.to_le_bytes());
            }
            for value in [crc, packed, size] {
                out.extend(value.to_le_bytes());
            }
            out.extend(name_length.to_le_bytes());
            out.extend(0u16.to_le_bytes());
        };
        archive.extend(0x0403_4b50u32.to_le_bytes());
        shared(&mut archive);
        archive.extend(name.as_bytes());
        archive.extend(&compressed);

        central.extend(0x0201_4b50u32.to_le_bytes());
        central.extend((UNIX | VERSION).to_le_bytes());
        shared(&mut central);
        // - Comment length, disk number, internal attributes.
        central.extend([0u8; 6]);
        central.extend((mode << 16).to_le_bytes());
        central.extend(offset.to_le_bytes());
        central.extend(name.as_bytes());
    }
    let start = u32::try_from(archive.len()).map_err(|_| too_large())?;
    let count = u16::try_from(payload.len()).map_err(|_| too_large())?;
    let central_length = u32::try_from(central.len()).map_err(|_| too_large())?;
    archive.extend(&central);
    archive.extend(0x0605_4b50u32.to_le_bytes());
    archive.extend([0u8; 4]);
    archive.extend(count.to_le_bytes());
    archive.extend(count.to_le_bytes());
    archive.extend(central_length.to_le_bytes());
    archive.extend(start.to_le_bytes());
    archive.extend(0u16.to_le_bytes());
    Ok(archive)
}

/// Writes the `bin/` files of `payload` into `out` without the prefix. That's
/// the folder Decky's store build copies to the plugin's `bin/`.
pub(super) fn write_bin(out: &Path, payload: &BTreeMap<String, Vec<u8>>) -> Result<(), String> {
    for (relative, data) in payload {
        let name = relative
            .strip_prefix("bin/")
            .ok_or_else(|| format!("not an agent file: {relative}"))?;
        let path = out.join(name);
        std::fs::write(&path, data).map_err(|error| format!("{}: {error}", path.display()))?;
    }
    Ok(())
}
