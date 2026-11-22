use std::path::PathBuf;
use windows::{
    Win32::{
        System::{
            Com::CoTaskMemFree,
            SystemInformation::{ComputerNamePhysicalDnsHostname, GetComputerNameExW},
        },
        UI::Shell::{FOLDERID_LocalAppData, KF_FLAG_DEFAULT, SHGetKnownFolderPath},
    },
    core::PWSTR,
};

/// `%LOCALAPPDATA%`: per user and never roamed, so credentials stay on this PC.
pub(super) fn local_app_data() -> Option<PathBuf> {
    // ! SAFETY: the folder ID is a static GUID; the returned string is freed below.
    let pointer =
        unsafe { SHGetKnownFolderPath(&FOLDERID_LocalAppData, KF_FLAG_DEFAULT, None) }.ok()?;
    // ! SAFETY: `pointer` is the NUL-terminated string SHGetKnownFolderPath returned.
    let path = unsafe { pointer.to_string() }.ok().map(PathBuf::from);
    // ! SAFETY: `pointer` came from SHGetKnownFolderPath and is freed exactly once.
    unsafe { CoTaskMemFree(Some(pointer.0.cast())) };
    path
}

pub(super) fn hostname() -> Option<String> {
    let mut buffer = [0u16; 256];
    let mut length = buffer.len() as u32;
    // ! SAFETY: `buffer` is live for the call and `length` states its size.
    unsafe {
        GetComputerNameExW(
            ComputerNamePhysicalDnsHostname,
            Some(PWSTR(buffer.as_mut_ptr())),
            &mut length,
        )
    }
    .ok()?;
    String::from_utf16(buffer.get(..length as usize)?).ok()
}
