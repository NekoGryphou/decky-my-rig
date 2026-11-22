use super::consts::CONTROL_CLIENT_ACCESS;
use crate::shared::native::wide;
use std::io;
use tokio::net::windows::named_pipe::NamedPipeClient;
use windows::{
    Win32::Storage::FileSystem::{
        CreateFileW, FILE_FLAG_OVERLAPPED, FILE_SHARE_MODE, OPEN_EXISTING, SECURITY_IDENTIFICATION,
        SECURITY_SQOS_PRESENT,
    },
    core::PCWSTR,
};

pub fn connect_client(name: &str) -> io::Result<NamedPipeClient> {
    // ! SAFETY: the wide name is NUL-terminated and lives until the call returns.
    let handle = unsafe {
        CreateFileW(
            PCWSTR(wide(name).as_ptr()),
            CONTROL_CLIENT_ACCESS,
            FILE_SHARE_MODE(0),
            None,
            OPEN_EXISTING,
            FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
            None,
        )
    }
    // ? CreateFileW errors come as HRESULT_FROM_WIN32, so keep the plain Windows
    //   error code.
    .map_err(|error| io::Error::from_raw_os_error(error.code().0 & 0xffff))?;
    // ? CreateFile supplied overlapped I/O; the Tokio client owns the handle.
    // ! SAFETY: CreateFileW opened `handle` with FILE_FLAG_OVERLAPPED and nothing else owns it.
    unsafe { NamedPipeClient::from_raw_handle(handle.0) }
}
