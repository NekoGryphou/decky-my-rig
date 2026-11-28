use crate::shared::native::{wide, with_security};
use std::{io, path::Path};
use windows::{Win32::Storage::FileSystem::CreateDirectoryW, core::PCWSTR};

fn os_error(error: windows::core::Error) -> io::Error {
    io::Error::from_raw_os_error(error.code().0 & 0xFFFF)
}

/// Creates `path` with the security described by `sddl`, in one call. An
/// existing folder gives an `AlreadyExists` error and is left alone.
pub(super) fn create_directory(path: &Path, sddl: &str) -> io::Result<()> {
    let name = wide(&path.to_string_lossy());
    with_security(sddl, |attributes| {
        // ! SAFETY: `name` is NUL-terminated and `attributes` points to a live descriptor for the
        // ! call.
        unsafe { CreateDirectoryW(PCWSTR(name.as_ptr()), Some(attributes.cast_const())) }
            .map_err(os_error)
    })
    .map_err(io::Error::other)?
}
