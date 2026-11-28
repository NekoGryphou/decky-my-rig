use super::{
    consts::{
        BACKUP_SEMANTICS, FILE_NAME, LOCK_NAME, OPEN_REPARSE_POINT, READ_AND_SECURE, READ_ONLY,
        REPARSE_POINT, SHARE_NO_DELETE, WRITE_AND_SECURE,
    },
    native::{current_owners, is_private, link_count, owner_of, secure},
    types::WindowsSettings,
};
use myrig_client::platform::{Settings, remove_leftover_documents, temporary_document_name};
use myrig_client::shared::read_bounded;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
    sync::{Mutex, PoisonError},
};

impl WindowsSettings {
    /// Opens the directory, creating it and making it private if `create` is
    /// set. Its parent folders are resolved (for a profile moved to another
    /// drive behind a junction), but the directory itself mustn't be a reparse
    /// point.
    pub(crate) fn open(directory: &Path, create: bool) -> io::Result<Self> {
        let name = directory.file_name().ok_or(io::ErrorKind::InvalidInput)?;
        let parent = directory.parent().ok_or(io::ErrorKind::InvalidInput)?;
        if create {
            fs::create_dir_all(parent)?;
        }
        let directory = fs::canonicalize(parent)?.join(name);
        if create {
            match fs::create_dir(&directory) {
                Err(error) if error.kind() != io::ErrorKind::AlreadyExists => return Err(error),
                _ => {}
            }
        }
        // ? Opened as the reparse point itself, so the check below can see it.
        let handle = OpenOptions::new()
            .access_mode(if create { READ_AND_SECURE } else { READ_ONLY })
            .share_mode(SHARE_NO_DELETE)
            .custom_flags(BACKUP_SEMANTICS | OPEN_REPARSE_POINT)
            .open(&directory)?;
        let metadata = handle.metadata()?;
        if !metadata.is_dir() || metadata.file_attributes() & REPARSE_POINT != 0 {
            return Err(io::ErrorKind::InvalidInput.into());
        }
        let owners = current_owners()?;
        if !owners.contains(&owner_of(&handle)?) {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        if create {
            secure(&handle, &owners[0], true)?;
        } else if !is_private(&handle, &owners)? {
            // ! Only creating the folder writes its ACL; every other open just checks it.
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        Ok(Self {
            directory,
            _handle: handle,
            owners,
            lock: Mutex::new(None),
        })
    }

    fn open_inside(&self, name: &str, options: &mut OpenOptions) -> io::Result<File> {
        options
            .custom_flags(OPEN_REPARSE_POINT)
            .open(self.directory.join(name))
    }
}

impl Settings for WindowsSettings {
    fn lock(&self) -> io::Result<()> {
        let file = self.open_inside(
            LOCK_NAME,
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false),
        )?;
        file.lock()?;
        *self.lock.lock().unwrap_or_else(PoisonError::into_inner) = Some(file);
        Ok(())
    }

    fn read(&self, limit: usize) -> io::Result<Option<Vec<u8>>> {
        let file = match self.open_inside(FILE_NAME, OpenOptions::new().access_mode(READ_ONLY)) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            other => other?,
        };
        let metadata = file.metadata()?;
        if !metadata.is_file()
            || metadata.file_attributes() & REPARSE_POINT != 0
            || link_count(&file)? != 1
            || !self.owners.contains(&owner_of(&file)?)
        {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        // ! Reads only check the ACL. A document copied in from somewhere with a wider one is
        // ! refused, saves included, until the user fixes its ACL or deletes it: nothing here
        // ! repairs a file it didn't write.
        if !is_private(&file, &self.owners)? {
            return Err(io::ErrorKind::PermissionDenied.into());
        }
        read_bounded(file, limit).map(Some)
    }

    /// Writes a new private file next to the document, flushes it and renames
    /// it over `pcs.json`.
    fn replace(&self, contents: &[u8]) -> io::Result<()> {
        remove_leftover_documents(&self.directory);
        let name = temporary_document_name();
        let result = self
            .open_inside(
                &name,
                OpenOptions::new()
                    .write(true)
                    .access_mode(WRITE_AND_SECURE)
                    .create_new(true),
            )
            .and_then(|mut file| {
                // ? Owned by this user even when running elevated, so a normal run can read it.
                secure(&file, &self.owners[0], false)?;
                file.write_all(contents)?;
                file.sync_all()
            })
            .and_then(|()| fs::rename(self.directory.join(&name), self.directory.join(FILE_NAME)));
        if result.is_err() {
            // * Best effort: the original document isn't touched either way.
            let _ = fs::remove_file(self.directory.join(&name));
        }
        result
    }
}
