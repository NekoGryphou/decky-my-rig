use super::consts::FAILURE;
use std::{
    ffi::OsStr,
    mem::{size_of, size_of_val},
    os::windows::ffi::OsStrExt,
};
use windows::{
    Win32::{
        Foundation::{HLOCAL, LocalFree},
        Security::{
            Authorization::{
                ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
                SDDL_REVISION_1,
            },
            LookupAccountNameW, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES, SID_NAME_USE,
        },
    },
    core::{PCWSTR, PWSTR},
};

pub(crate) fn wide(value: &str) -> Vec<u16> {
    OsStr::new(value).encode_wide().chain(Some(0)).collect()
}

pub(crate) struct LocalAllocation(pub(crate) *mut std::ffi::c_void);
impl Drop for LocalAllocation {
    fn drop(&mut self) {
        // ! SAFETY: `self.0` came from a successful LocalAlloc-based API and is freed once, on drop.
        unsafe {
            LocalFree(Some(HLOCAL(self.0)));
        }
    }
}

/// Runs `create` with security attributes holding the descriptor described by
/// `sddl`. The descriptor lives until `create` returns.
pub(crate) fn with_security<T>(
    sddl: &str,
    create: impl FnOnce(*mut SECURITY_ATTRIBUTES) -> T,
) -> Result<T, &'static str> {
    let text = wide(sddl);
    let mut pointer = PSECURITY_DESCRIPTOR::default();
    // ! SAFETY: the SDDL text is NUL-terminated and outlives the call; the returned descriptor is freed by `allocation`.
    unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            PCWSTR(text.as_ptr()),
            SDDL_REVISION_1,
            &mut pointer,
            None,
        )
    }
    .map_err(|_| FAILURE)?;
    let allocation = LocalAllocation(pointer.0);
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: pointer.0,
        bInheritHandle: false.into(),
    };
    let result = create(&mut attributes);
    drop(allocation);
    Ok(result)
}

/// `sid` must point to a complete SID. Every caller passes a lookup buffer, or
/// a SID inside a descriptor or token buffer it keeps alive.
pub(crate) fn sid_text(sid: PSID) -> Result<String, &'static str> {
    let mut pointer = PWSTR::null();
    // ! SAFETY: the caller passes a valid SID; `pointer` is a valid out-pointer.
    unsafe { ConvertSidToStringSidW(sid, &mut pointer) }.map_err(|_| FAILURE)?;
    let allocation = LocalAllocation(pointer.0.cast());
    // ! SAFETY: ConvertSidToStringSidW succeeded, so `pointer` is a NUL-terminated string.
    let text = unsafe { pointer.to_string() }.map_err(|_| FAILURE)?;
    drop(allocation);
    Ok(text)
}

pub(crate) fn service_sid_text() -> Result<String, &'static str> {
    let (mut sid, _) = account_sid(&format!(
        "NT SERVICE\\{}",
        crate::shared::consts::SERVICE_NAME
    ))?;
    sid_text(PSID(sid.as_mut_ptr().cast()))
}

/// The SID of a qualified account name, and what kind of account it is.
pub(crate) fn account_sid(account: &str) -> Result<(Vec<u32>, SID_NAME_USE), &'static str> {
    let account = wide(account);
    let mut sid = vec![0u32; 32];
    let mut sid_length = size_of_val(sid.as_slice()) as u32;
    let mut domain = [0u16; 256];
    let mut domain_length = domain.len() as u32;
    let mut kind = SID_NAME_USE::default();
    // ! SAFETY: every buffer is live for the call and each length states its buffer's size.
    unsafe {
        LookupAccountNameW(
            PCWSTR::null(),
            PCWSTR(account.as_ptr()),
            Some(PSID(sid.as_mut_ptr().cast())),
            &mut sid_length,
            Some(PWSTR(domain.as_mut_ptr())),
            &mut domain_length,
            &mut kind,
        )
    }
    .map_err(|_| FAILURE)?;
    Ok((sid, kind))
}

/// The PC's name, used by the service's pipe and registry control.
#[cfg(feature = "service")]
pub(crate) fn computer_name() -> Result<String, &'static str> {
    use windows::Win32::System::WindowsProgramming::{GetComputerNameW, MAX_COMPUTERNAME_LENGTH};
    let mut computer = [0u16; MAX_COMPUTERNAME_LENGTH as usize + 1];
    let mut length = computer.len() as u32;
    // ! SAFETY: `computer` is live for the call and `length` states its size.
    unsafe { GetComputerNameW(Some(PWSTR(computer.as_mut_ptr())), &mut length) }
        .map_err(|_| FAILURE)?;
    String::from_utf16(&computer[..length as usize]).map_err(|_| FAILURE)
}
