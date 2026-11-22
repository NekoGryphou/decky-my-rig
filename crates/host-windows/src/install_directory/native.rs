use super::{
    consts::FAILURE,
    types::{AccessRule, Security},
};
use super::{
    consts::{VOLUME_GUID_UNITS, VOLUME_PATH_UNITS},
    utils::drive_is_registered,
};
use crate::shared::native::{LocalAllocation, sid_text, wide};
use std::{fs, io, os::windows::fs::MetadataExt, path::Path};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
use windows::Win32::Storage::FileSystem::{
    GetDriveTypeW, GetVolumeNameForVolumeMountPointW, GetVolumePathNamesForVolumeNameW,
};
use windows::Win32::System::WindowsProgramming::DRIVE_FIXED;
use windows::{
    Win32::Security::{
        ACL,
        Authorization::{
            DENY_ACCESS, GRANT_ACCESS, GetExplicitEntriesFromAclW, GetNamedSecurityInfoW,
            SE_FILE_OBJECT, TRUSTEE_IS_SID,
        },
        DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, GetSecurityDescriptorOwner,
        INHERIT_ONLY_ACE, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
    },
    core::{BOOL, PCWSTR},
};

pub(super) fn read(path: &Path) -> Result<Security, &'static str> {
    let name = wide(path.to_str().ok_or(FAILURE)?);
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // ! SAFETY: `name` is NUL-terminated and live; the descriptor is freed by `_allocation`.
    unsafe {
        GetNamedSecurityInfoW(
            PCWSTR(name.as_ptr()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            None,
            None,
            None,
            None,
            &mut descriptor,
        )
    }
    .ok()
    .map_err(|_| FAILURE)?;
    let _allocation = LocalAllocation(descriptor.0);
    decode(descriptor)
}

pub(super) fn check_drive(root: &Path) -> Result<(), &'static str> {
    let text = root.to_str().ok_or(FAILURE)?;
    let root = wide(text);
    // ! SAFETY: `root` is NUL-terminated and live for the call.
    let fixed = unsafe { GetDriveTypeW(PCWSTR(root.as_ptr())) } == DRIVE_FIXED;
    if !fixed {
        return Err(FAILURE);
    }
    let mut volume = [0u16; VOLUME_GUID_UNITS];
    // ! SAFETY: `root` is NUL-terminated and `volume` is a live output buffer.
    unsafe { GetVolumeNameForVolumeMountPointW(PCWSTR(root.as_ptr()), &mut volume) }
        .map_err(|_| FAILURE)?;
    let mut mounts = vec![0u16; VOLUME_PATH_UNITS];
    let mut returned = 0;
    // ! SAFETY: `volume` is NUL-terminated and `mounts` is a live output buffer.
    unsafe {
        GetVolumePathNamesForVolumeNameW(PCWSTR(volume.as_ptr()), Some(&mut mounts), &mut returned)
    }
    .map_err(|_| FAILURE)?;
    let mounts = mounts.get(..returned as usize).ok_or(FAILURE)?;
    // ? Mount-manager paths leave out drive letters that only exist in the
    //   installer's login session.
    if drive_is_registered(text, mounts) {
        Ok(())
    } else {
        Err(FAILURE)
    }
}

/// `descriptor` must point to a valid security descriptor. `read` passes one
/// from GetNamedSecurityInfoW, the tests one from the SDDL converter.
pub(super) fn decode(descriptor: PSECURITY_DESCRIPTOR) -> Result<Security, &'static str> {
    let mut owner = PSID::default();
    let mut defaulted = BOOL::default();
    let mut present = BOOL::default();
    let mut dacl: *mut ACL = std::ptr::null_mut();
    // ! SAFETY: `descriptor` is a valid security descriptor (see `decode`); all out-pointers are live.
    unsafe {
        GetSecurityDescriptorOwner(descriptor, &mut owner, &mut defaulted).map_err(|_| FAILURE)?;
        GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted)
            .map_err(|_| FAILURE)?;
    }
    if !present.as_bool() || dacl.is_null() {
        return Err(FAILURE);
    }
    let owner = sid_text(owner)?;
    let mut count = 0;
    let mut entries = std::ptr::null_mut();
    // ! SAFETY: `dacl` is the descriptor's present, non-null DACL; `entries` is freed by `_allocation`.
    unsafe { GetExplicitEntriesFromAclW(dacl, &mut count, &mut entries) }
        .ok()
        .map_err(|_| FAILURE)?;
    let _allocation = LocalAllocation(entries.cast());
    let mut rules = Vec::new();
    if count != 0 {
        if entries.is_null() {
            return Err(FAILURE);
        }
        // ! SAFETY: GetExplicitEntriesFromAclW returned `count` entries at the non-null `entries`.
        for entry in unsafe { std::slice::from_raw_parts(entries, count as usize) } {
            if entry.Trustee.TrusteeForm != TRUSTEE_IS_SID
                || !matches!(entry.grfAccessMode, GRANT_ACCESS | DENY_ACCESS)
            {
                return Err(FAILURE);
            }
            rules.push(AccessRule {
                principal: sid_text(PSID(entry.Trustee.ptstrName.0.cast()))?,
                rights: entry.grfAccessPermissions,
                inherit_only: entry.grfInheritance.contains(INHERIT_ONLY_ACE),
                inheritable: entry.grfInheritance.0 & 3 != 0,
                allow: entry.grfAccessMode == GRANT_ACCESS,
            });
        }
    }
    Ok(Security { owner, rules })
}

pub(super) fn metadata(path: &Path) -> io::Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, FAILURE));
    }
    Ok(metadata)
}
