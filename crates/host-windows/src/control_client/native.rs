use super::consts::{CONTROL_CLIENT_ACCESS, PIPE_BUSY_WAIT_MS};
use crate::shared::native::wide;
use std::{
    io,
    time::{Duration, Instant},
};
use tokio::net::windows::named_pipe::NamedPipeClient;
use windows::{
    Win32::{
        Foundation::ERROR_PIPE_BUSY,
        Storage::FileSystem::{
            CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_MODE, OPEN_EXISTING,
            SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
        },
        System::Pipes::WaitNamedPipeW,
    },
    core::PCWSTR,
};

/// Connects to the control pipe. When every instance is busy it waits up to
/// `PIPE_BUSY_WAIT_MS` for one to free up, so concurrent clients queue instead
/// of failing.
pub fn connect_client(name: &str) -> io::Result<NamedPipeClient> {
    let wide_name = wide(name);
    let mut error = match open(&wide_name) {
        Ok(client) => return Ok(client),
        Err(error) => error,
    };
    if error.raw_os_error() != i32::try_from(ERROR_PIPE_BUSY.0).ok() {
        return Err(error);
    }
    // ! WaitNamedPipeW returns when an instance is free; another client can still take it first, so
    // ! the open is tried again after each wait, within the same bound.
    let deadline = Instant::now() + Duration::from_millis(PIPE_BUSY_WAIT_MS.into());
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(error);
        }
        // ! SAFETY: the wide name is NUL-terminated and lives until the call returns.
        unsafe {
            WaitNamedPipeW(
                PCWSTR(wide_name.as_ptr()),
                u32::try_from(left.as_millis())
                    .unwrap_or(PIPE_BUSY_WAIT_MS)
                    .max(1),
            )
            .ok()
            .map_err(|wait| io::Error::from_raw_os_error(wait.code().0 & 0xffff))?;
        }
        match open(&wide_name) {
            Ok(client) => return Ok(client),
            Err(next) if next.raw_os_error() == i32::try_from(ERROR_PIPE_BUSY.0).ok() => {
                error = next;
            }
            Err(next) => return Err(next),
        }
    }
}

fn open(wide_name: &[u16]) -> io::Result<NamedPipeClient> {
    // ! SAFETY: the wide name is NUL-terminated and lives until the call returns.
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide_name.as_ptr()),
            CONTROL_CLIENT_ACCESS,
            FILE_SHARE_MODE(0),
            None,
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
            None,
        )
    }
    // ? CreateFileW errors come as HRESULT_FROM_WIN32, so keep the plain Windows error code.
    .map_err(|error| io::Error::from_raw_os_error(error.code().0 & 0xffff))?;
    // ? CreateFile supplied overlapped I/O; the Tokio client owns the handle.
    // ! SAFETY: CreateFileW opened `handle` with FILE_FLAG_OVERLAPPED and nothing else owns it.
    unsafe { NamedPipeClient::from_raw_handle(handle.0) }
}
