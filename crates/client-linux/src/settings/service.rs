use super::{
    consts::FILE_NAME,
    native::{current_uid, open_at, rename_at, unlink_at},
    types::LinuxSettings,
};
use myrig_client::platform::{Settings, remove_leftover_documents, temporary_document_name};
use myrig_client::shared::read_bounded;
use std::{
    ffi::CString,
    fs::{self, DirBuilder, File, OpenOptions, Permissions},
    io::{self, Write},
    os::unix::{
        fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt},
        io::AsRawFd,
    },
    path::Path,
};

impl LinuxSettings {
    /// Opens the directory, creating it (0700) if `create` is set. Parent
    /// folders are resolved (Fedora Atomic links /home to var/home), but the
    /// directory itself mustn't be a link and has to belong to this user.
    pub(crate) fn open(directory: &Path, create: bool) -> io::Result<Self> {
        let name = directory.file_name().ok_or(io::ErrorKind::InvalidInput)?;
        let parent = directory.parent().ok_or(io::ErrorKind::InvalidInput)?;
        if create {
            DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(parent)?;
        }
        let directory = fs::canonicalize(parent)?.join(name);
        if create {
            match DirBuilder::new().mode(0o700).create(&directory) {
                Err(error) if error.kind() != io::ErrorKind::AlreadyExists => return Err(error),
                _ => {}
            }
        }
        let opened = open_directory(&directory)?;
        if create {
            opened.set_permissions(Permissions::from_mode(0o700))?;
        }
        Ok(Self { directory: opened })
    }
}

impl Settings for LinuxSettings {
    fn lock(&self) -> io::Result<()> {
        self.directory.lock()
    }

    fn read(&self, limit: usize) -> io::Result<Option<Vec<u8>>> {
        let opened = open_at(
            &self.directory,
            FILE_NAME,
            libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK,
            0,
        );
        let file = match opened {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            other => other?,
        };
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.uid() != current_uid() || metadata.nlink() != 1 {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        file.set_permissions(Permissions::from_mode(0o600))?;
        read_bounded(file, limit).map(Some)
    }

    /// Writes a private temp file, syncs it and renames it over `pcs.json`, all
    /// relative to the open directory.
    fn replace(&self, contents: &[u8]) -> io::Result<()> {
        // ? The open descriptor's path, so the directory can't be swapped for another between
        // ? opening and cleaning.
        remove_leftover_documents(Path::new(&format!(
            "/proc/self/fd/{}",
            self.directory.as_raw_fd()
        )));
        let temporary = CString::new(temporary_document_name())
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        let flags = libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW;
        open_at(&self.directory, &temporary, flags, 0o600).and_then(|mut file| {
            let result = file
                .write_all(contents)
                .and_then(|()| file.sync_all())
                .and_then(|()| rename_at(&self.directory, &temporary, FILE_NAME));
            if result.is_err() {
                // * Best effort: the original document isn't touched either way.
                let _ = unlink_at(&self.directory, &temporary);
            }
            result
        })?;
        // * The rename already happened; a failed directory sync can't undo it.
        let _ = self.directory.sync_all();
        Ok(())
    }
}

/// `O_NOFOLLOW` refuses a directory that is itself a link.
fn open_directory(directory: &Path) -> io::Result<File> {
    let opened = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW)
        .open(directory)?;
    let owner = opened.metadata()?.uid();
    if owner != current_uid() {
        return Err(io::ErrorKind::PermissionDenied.into());
    }
    Ok(opened)
}
