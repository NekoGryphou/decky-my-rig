use std::{
    fs::File,
    io,
    mem::size_of,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
};
use windows::{
    Win32::{
        Foundation::{HANDLE, HLOCAL, LocalFree},
        Security::{
            ACL,
            Authorization::{
                ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
                GetSecurityInfo, SDDL_REVISION_1, SE_FILE_OBJECT, SetSecurityInfo,
            },
            DACL_SECURITY_INFORMATION, GetSecurityDescriptorDacl, GetSecurityDescriptorOwner,
            GetTokenInformation, OWNER_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION,
            PSECURITY_DESCRIPTOR, PSID, TOKEN_OWNER, TOKEN_QUERY, TOKEN_USER, TokenOwner,
            TokenUser,
        },
        Storage::FileSystem::{BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    },
    core::{BOOL, PCWSTR, PWSTR},
};

fn failure(error: windows::core::Error) -> io::Error {
    io::Error::from_raw_os_error(error.code().0 & 0xffff)
}

/// `sid` must point to a complete SID kept alive by the caller.
fn sid_text(sid: PSID) -> io::Result<String> {
    let mut text = PWSTR::null();
    // ! SAFETY: the caller passes a valid SID; `text` is a valid out-pointer.
    unsafe { ConvertSidToStringSidW(sid, &mut text) }.map_err(failure)?;
    // ! SAFETY: ConvertSidToStringSidW succeeded, so `text` is NUL-terminated.
    let result = unsafe { text.to_string() }.map_err(|_| io::ErrorKind::InvalidData.into());
    // ! SAFETY: `text` was allocated by ConvertSidToStringSidW and is freed once.
    unsafe { LocalFree(Some(HLOCAL(text.0.cast()))) };
    result
}

/// This process's user SID first, then the default owner of the objects it
/// creates (Administrators when elevated).
pub(super) fn current_owners() -> io::Result<Vec<String>> {
    let mut token = HANDLE::default();
    // ! SAFETY: the process handle is a pseudo-handle and `token` a valid out-pointer.
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.map_err(failure)?;
    // ! SAFETY: OpenProcessToken succeeded, so `token` is a new handle nothing else owns.
    let token = unsafe { OwnedHandle::from_raw_handle(token.0) };
    let token = HANDLE(token.as_raw_handle());
    let mut owners = Vec::new();
    for class in [TokenUser, TokenOwner] {
        let mut buffer = [0u64; 64];
        let mut needed = 0;
        // ! SAFETY: `buffer` is live for the call and its size is stated.
        unsafe {
            GetTokenInformation(
                token,
                class,
                Some(buffer.as_mut_ptr().cast()),
                size_of::<[u64; 64]>() as u32,
                &mut needed,
            )
        }
        .map_err(failure)?;
        let sid = if class == TokenUser {
            // ! SAFETY: GetTokenInformation filled `buffer`, 8-byte aligned, with a TOKEN_USER.
            unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid }
        } else {
            // ! SAFETY: GetTokenInformation filled `buffer`, 8-byte aligned, with a TOKEN_OWNER.
            unsafe { (*buffer.as_ptr().cast::<TOKEN_OWNER>()).Owner }
        };
        owners.push(sid_text(sid)?);
    }
    owners.dedup();
    Ok(owners)
}

/// The owner SID of an open file or directory.
pub(super) fn owner_of(file: &File) -> io::Result<String> {
    let mut owner = PSID::default();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // ! SAFETY: `file` is open for the call; the descriptor is freed below.
    unsafe {
        GetSecurityInfo(
            HANDLE(file.as_raw_handle()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION,
            Some(&mut owner),
            None,
            None,
            None,
            Some(&mut descriptor),
        )
    }
    .ok()
    .map_err(failure)?;
    let result = if owner.0.is_null() {
        Err(io::ErrorKind::PermissionDenied.into())
    } else {
        sid_text(owner)
    };
    // ! SAFETY: `descriptor` came from GetSecurityInfo and is freed once.
    unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
    result
}

/// Makes `user` the owner of the open `file` and replaces its ACL with one
/// protected entry giving `user` full control. With `inherited`, the entry also
/// applies to everything created inside a directory later. The handle needs
/// `READ_CONTROL`, `WRITE_DAC` and `WRITE_OWNER`.
pub(super) fn secure(file: &File, user: &str, inherited: bool) -> io::Result<()> {
    let flags = if inherited { "OICI" } else { "" };
    let sddl: Vec<u16> = format!("O:{user}D:P(A;{flags};FA;;;{user})")
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // ! SAFETY: the SDDL text is NUL-terminated and outlives the call; the
    //   descriptor is freed below.
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(sddl.as_ptr()),
            SDDL_REVISION_1,
            &mut descriptor,
            None,
        )
    }
    .map_err(failure)?;
    let result = apply(file, descriptor);
    // ! SAFETY: `descriptor` came from the SDDL converter and is freed once.
    unsafe { LocalFree(Some(HLOCAL(descriptor.0))) };
    result
}

/// `descriptor` must be a valid descriptor holding an owner and a DACL.
fn apply(file: &File, descriptor: PSECURITY_DESCRIPTOR) -> io::Result<()> {
    let mut owner = PSID::default();
    let mut defaulted = BOOL::default();
    let mut present = BOOL::default();
    let mut dacl: *mut ACL = std::ptr::null_mut();
    // ! SAFETY: `descriptor` is valid (see above) and every out-pointer is live.
    unsafe {
        GetSecurityDescriptorOwner(descriptor, &mut owner, &mut defaulted).map_err(failure)?;
        GetSecurityDescriptorDacl(descriptor, &mut present, &mut dacl, &mut defaulted)
            .map_err(failure)?;
    }
    // ! SAFETY: `file` is open with the access this needs; `owner` and `dacl`
    //   point into `descriptor`, which outlives the call.
    unsafe {
        SetSecurityInfo(
            HANDLE(file.as_raw_handle()),
            SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION
                | DACL_SECURITY_INFORMATION
                | PROTECTED_DACL_SECURITY_INFORMATION,
            Some(owner),
            None,
            Some(dacl),
            None,
        )
    }
    .ok()
    .map_err(failure)
}

/// How many names the open file has. More than one means a hard link.
pub(super) fn link_count(file: &File) -> io::Result<u32> {
    let mut information = BY_HANDLE_FILE_INFORMATION::default();
    // ! SAFETY: `file` is open for the call and `information` a valid out-pointer.
    unsafe { GetFileInformationByHandle(HANDLE(file.as_raw_handle()), &mut information) }
        .map_err(failure)?;
    Ok(information.nNumberOfLinks)
}
