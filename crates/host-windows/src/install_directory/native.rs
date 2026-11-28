use super::{
    consts::FAILURE,
    types::{AccessRule, Security},
};
use super::{
    consts::{VOLUME_GUID_UNITS, VOLUME_PATH_UNITS},
    utils::drive_is_registered,
};
use crate::shared::native::{LocalAllocation, sid_text, wide};
use std::{ffi::c_void, fs, io, mem::size_of, os::windows::fs::MetadataExt, path::Path};
use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
use windows::Win32::Storage::FileSystem::{
    GetDriveTypeW, GetVolumeNameForVolumeMountPointW, GetVolumePathNamesForVolumeNameW,
};
use windows::Win32::System::WindowsProgramming::DRIVE_FIXED;
use windows::{
    Win32::Security::{
        ACCESS_ALLOWED_ACE, ACE_HEADER, ACL, ACL_SIZE_INFORMATION, AclSizeInformation,
        Authorization::{GetNamedSecurityInfoW, SE_FILE_OBJECT},
        DACL_SECURITY_INFORMATION, GetAce, GetAclInformation, GetSecurityDescriptorDacl,
        GetSecurityDescriptorOwner, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
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
    // ? Mount-manager paths leave out drive letters that only exist in the installer's login
    // ? session.
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
    // ! SAFETY: `descriptor` is a valid security descriptor (see `decode`); all out-pointers are
    // ! live.
    unsafe {
        GetSecurityDescriptorOwner(descriptor, &mut owner, &mut defaulted).map_err(|_| FAILURE)?;
        GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted)
            .map_err(|_| FAILURE)?;
    }
    if !present.as_bool() || dacl.is_null() {
        return Err(FAILURE);
    }
    let owner = sid_text(owner)?;
    let rules = access_rules(dacl)?;
    Ok(Security { owner, rules })
}

// Every entry in the DACL, inherited ones included. GetExplicitEntriesFromAclW
// leaves inherited entries out, but a parent's grants reach a child through them,
// so skipping them would let a user-writable folder pass as safe.
fn access_rules(dacl: *mut ACL) -> Result<Vec<AccessRule>, &'static str> {
    let mut info = ACL_SIZE_INFORMATION::default();
    // ! SAFETY: `dacl` is a non-null DACL from a valid descriptor, and `info` is the size-
    // ! information structure AclSizeInformation fills.
    unsafe {
        GetAclInformation(
            dacl,
            (&mut info as *mut ACL_SIZE_INFORMATION).cast(),
            size_of::<ACL_SIZE_INFORMATION>() as u32,
            AclSizeInformation,
        )
    }
    .map_err(|_| FAILURE)?;
    let mut rules = Vec::new();
    for index in 0..info.AceCount {
        let mut ace: *mut c_void = std::ptr::null_mut();
        // ! SAFETY: `index` is below AceCount, so GetAce returns the entry at that index, which
        // ! lives as long as the DACL.
        unsafe { GetAce(dacl, index, &mut ace) }.map_err(|_| FAILURE)?;
        if ace.is_null() {
            return Err(FAILURE);
        }
        // ! SAFETY: the header is the first field of every ACE type, so it can be read before the
        // ! type is known.
        let header = unsafe { &*(ace as *const ACE_HEADER) };
        let allow = match header.AceType {
            ACCESS_ALLOWED_ACE_TYPE => true,
            ACCESS_DENIED_ACE_TYPE => false,
            // Audit entries grant nothing.
            ACCESS_ALLOWED_AUDIT_TYPE | ACCESS_DENIED_AUDIT_TYPE => continue,
            // An object-specific or unknown entry can't be judged here.
            _ => return Err(FAILURE),
        };
        // ! SAFETY: the allowed and denied ACE types share this layout, and their SID starts at
        // ! SidStart, which the ACE size bounds.
        let (mask, sid) = unsafe {
            let entry = &*(ace as *const ACCESS_ALLOWED_ACE);
            (
                entry.Mask,
                PSID(std::ptr::addr_of!(entry.SidStart).cast_mut().cast()),
            )
        };
        rules.push(AccessRule {
            principal: sid_text(sid)?,
            rights: mask,
            inherit_only: header.AceFlags & INHERIT_ONLY_FLAG != 0,
            inheritable: header.AceFlags & (OBJECT_INHERIT_FLAG | CONTAINER_INHERIT_FLAG) != 0,
            allow,
        });
    }
    Ok(rules)
}

const ACCESS_ALLOWED_ACE_TYPE: u8 = 0;
const ACCESS_DENIED_ACE_TYPE: u8 = 1;
const ACCESS_ALLOWED_AUDIT_TYPE: u8 = 2;
const ACCESS_DENIED_AUDIT_TYPE: u8 = 3;
const OBJECT_INHERIT_FLAG: u8 = 0x01;
const CONTAINER_INHERIT_FLAG: u8 = 0x02;
const INHERIT_ONLY_FLAG: u8 = 0x08;

pub(super) fn metadata(path: &Path) -> io::Result<fs::Metadata> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, FAILURE));
    }
    Ok(metadata)
}
