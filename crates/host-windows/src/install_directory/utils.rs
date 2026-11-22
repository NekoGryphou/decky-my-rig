use super::{
    consts::*,
    types::{PathRole, Security},
};
use std::path::{Component, Path, Prefix};

pub(super) fn drive_is_registered(root: &str, mounts: &[u16]) -> bool {
    mounts.ends_with(&[0, 0])
        && mounts
            .split(|unit| *unit == 0)
            .take_while(|path| !path.is_empty())
            .any(|path| String::from_utf16(path).is_ok_and(|path| path.eq_ignore_ascii_case(root)))
}

pub(super) fn check_security(security: &Security, role: PathRole) -> Result<(), &'static str> {
    if !TRUSTED_OWNERS.contains(&security.owner.as_str()) {
        return Err(FAILURE);
    }
    for rule in &security.rules {
        if !rule.allow || TRUSTED_OWNERS.contains(&rule.principal.as_str()) {
            continue;
        }
        if rule.inherit_only
            && (!matches!(role, PathRole::CreationParent | PathRole::Destination)
                || !rule.inheritable
                || rule.principal == CREATOR_OWNER)
        {
            continue;
        }
        let forbidden = if matches!(role, PathRole::Ancestor) {
            ANCESTOR_ACCESS
        } else {
            WRITE_ACCESS
        };
        if rule.rights & forbidden != 0 {
            return Err(FAILURE);
        }
    }
    Ok(())
}

pub(super) fn check_path(path: &Path) -> Result<(), &'static str> {
    let text = path.to_str().ok_or(FAILURE)?;
    let mut parts = path.components();
    if !matches!(parts.next(), Some(Component::Prefix(prefix)) if matches!(prefix.kind(), Prefix::Disk(_)))
        || !matches!(parts.next(), Some(Component::RootDir))
        || parts.next().is_none()
    {
        return Err(FAILURE);
    }
    for name in text.split(['\\', '/']).skip(1) {
        let device = name.split('.').next().unwrap_or("").to_ascii_uppercase();
        let numbered_device = device
            .strip_prefix("COM")
            .or_else(|| device.strip_prefix("LPT"))
            .is_some_and(|suffix| suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9'));
        if name.is_empty()
            || matches!(name, "." | "..")
            || name.ends_with(['.', ' '])
            || name.chars().any(|ch| ch < ' ' || "<>:\"|?*".contains(ch))
            || RESERVED_NAMES.contains(&device.as_str())
            || numbered_device
        {
            return Err(FAILURE);
        }
    }
    Ok(())
}
