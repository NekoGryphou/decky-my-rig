use std::{
    ffi::CStr,
    fs::File,
    io,
    os::fd::{AsRawFd, FromRawFd},
};

pub(super) fn current_uid() -> u32 {
    // ! SAFETY: getuid has no preconditions and cannot fail.
    unsafe { libc::getuid() }
}

/// Opens `name` inside the already open `directory` rather than by path, so a
/// directory swapped for a symlink afterwards isn't followed.
pub(super) fn open_at(directory: &File, name: &CStr, flags: i32, mode: u32) -> io::Result<File> {
    // ! SAFETY: `name` is NUL-terminated and `directory` is an open descriptor.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags | libc::O_CLOEXEC,
            mode,
        )
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // ! SAFETY: `fd` was just returned by openat and is owned by nothing else.
    Ok(unsafe { File::from_raw_fd(fd) })
}

pub(super) fn rename_at(directory: &File, from: &CStr, to: &CStr) -> io::Result<()> {
    let fd = directory.as_raw_fd();
    // ! SAFETY: both names are NUL-terminated; both refer to `directory`.
    if unsafe { libc::renameat(fd, from.as_ptr(), fd, to.as_ptr()) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

pub(super) fn unlink_at(directory: &File, name: &CStr) -> io::Result<()> {
    // ! SAFETY: `name` is NUL-terminated and `directory` is an open descriptor.
    if unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), 0) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}
