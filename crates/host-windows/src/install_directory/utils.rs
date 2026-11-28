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
    let owner_rights_limit_owner = security
        .rules
        .iter()
        .any(|rule| rule.allow && rule.principal == OWNER_RIGHTS);
    let trusted_owner = TRUSTED_OWNERS.contains(&security.owner.as_str())
        || (security.owner == LOCAL_SERVICE && owner_rights_limit_owner);
    if !trusted_owner {
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
        if name.is_empty()
            || matches!(name, "." | "..")
            || name.ends_with(['.', ' '])
            || name.chars().any(|ch| ch < ' ' || "<>:\"|?*".contains(ch))
            || is_device_name(name)
        {
            return Err(FAILURE);
        }
    }
    Ok(())
}

/// Windows treats these names as devices in any folder, with or without an
/// extension and ignoring case and trailing spaces before the extension.
fn is_device_name(name: &str) -> bool {
    let stem = name
        .split('.')
        .next()
        .unwrap_or("")
        .trim_end_matches(' ')
        .to_uppercase();
    let numbered = |prefix: &str| {
        stem.strip_prefix(prefix).is_some_and(|suffix| {
            matches!(
                suffix,
                "1" | "2"
                    | "3"
                    | "4"
                    | "5"
                    | "6"
                    | "7"
                    | "8"
                    | "9"
                    | "\u{b9}"
                    | "\u{b2}"
                    | "\u{b3}"
            )
        })
    };
    RESERVED_NAMES.contains(&stem.as_str()) || numbered("COM") || numbered("LPT")
}
