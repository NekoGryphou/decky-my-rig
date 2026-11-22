use crate::shared::{consts::FAILURE, native::wide};
use std::os::windows::io::{FromRawHandle, OwnedHandle};
use windows::{
    Win32::{
        Foundation::{ERROR_ALREADY_EXISTS, GetLastError},
        System::Threading::CreateMutexW,
    },
    core::PCWSTR,
};

/// Held while the process runs. Dropping it lets another copy start.
pub struct InstanceGuard {
    _mutex: OwnedHandle,
}

/// None if another copy already holds `name` (a `Local\` mutex name).
pub fn acquire(name: &str) -> Result<Option<InstanceGuard>, &'static str> {
    let name = wide(name);
    let handle =
        // ! SAFETY: `name` is NUL-terminated and live for the call.
        unsafe { CreateMutexW(None, false, PCWSTR(name.as_ptr())) }.map_err(|_| FAILURE)?;
    // ! SAFETY: GetLastError has no preconditions; it reads the CreateMutexW result.
    let existed = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
    // ! SAFETY: CreateMutexW succeeded, so `handle` is a new handle that nothing else owns.
    let handle = unsafe { OwnedHandle::from_raw_handle(handle.0) };
    Ok((!existed).then_some(InstanceGuard { _mutex: handle }))
}
