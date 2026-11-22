use super::{
    consts::{
        BACKUP_SEMANTICS, FILE_NAME, LOCK_NAME, OPEN_REPARSE_POINT, READ_AND_SECURE, REPARSE_POINT,
        SHARE_NO_DELETE, WRITE_AND_SECURE,
    },
    native::{current_owners, link_count, owner_of, secure},
    types::WindowsSettings,
};
use myrig_client::platform::{Settings, is_temporary_document, temporary_document_name};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
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
            .access_mode(READ_AND_SECURE)
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
        let mut file =
            match self.open_inside(FILE_NAME, OpenOptions::new().access_mode(READ_AND_SECURE)) {
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
        // ? Like Linux's 0600 on every read: a document copied in from somewhere
        //   doesn't keep a wider ACL.
        secure(&file, &self.owners[0], false)?;
        let mut raw = Vec::new();
        (&mut file).take(limit as u64 + 1).read_to_end(&mut raw)?;
        if raw.len() > limit {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(Some(raw))
    }

    /// Writes a new private file next to the document, flushes it and renames
    /// it over `pcs.json`.
    fn replace(&self, contents: &[u8]) -> io::Result<()> {
        // * Temp files left by killed saves. This runs under the store lock.
        for entry in fs::read_dir(&self.directory)
            .into_iter()
            .flatten()
            .flatten()
        {
            if entry
                .file_name()
                .to_str()
                .is_some_and(is_temporary_document)
            {
                let _ = fs::remove_file(entry.path());
            }
        }
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
                // ? Owned by this user even when running elevated, so a normal
                //   run can read it.
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
