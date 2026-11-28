use myrig_client::platform::InputEcho;
use std::path::PathBuf;
use windows::{
    Win32::{
        Foundation::HANDLE,
        System::{
            Com::CoTaskMemFree,
            Console::{
                CONSOLE_MODE, ENABLE_ECHO_INPUT, GetConsoleMode, GetStdHandle, STD_INPUT_HANDLE,
                SetConsoleMode,
            },
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

/// Console echo off until dropped, which restores the mode it found.
pub(super) struct Echo {
    handle: HANDLE,
    original: CONSOLE_MODE,
}

impl Echo {
    /// Turns echo off on the console's input. `None` when input isn't a console,
    /// where there is no echo to hide.
    pub(super) fn hide() -> Option<Self> {
        // ! SAFETY: the standard-input handle is a pseudo-handle owned by the process; it isn't
        // ! closed here.
        let handle = unsafe { GetStdHandle(STD_INPUT_HANDLE) }.ok()?;
        let mut original = CONSOLE_MODE::default();
        // ! SAFETY: `original` is a valid out- pointer for the console mode.
        unsafe { GetConsoleMode(handle, &mut original) }.ok()?;
        // ! SAFETY: `handle` is a console input handle, as GetConsoleMode accepted.
        unsafe { SetConsoleMode(handle, original & !ENABLE_ECHO_INPUT) }.ok()?;
        Some(Self { handle, original })
    }
}

impl Drop for Echo {
    fn drop(&mut self) {
        // ! SAFETY: `handle` and `original` came from the same console input. A failure here can't
        // ! be reported from drop, so it is ignored.
        let _ = unsafe { SetConsoleMode(self.handle, self.original) };
    }
}

impl InputEcho for Echo {}
