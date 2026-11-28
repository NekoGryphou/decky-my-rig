use super::{errors::StorageError, types::FileStorage, utils::read_bounded};
use crate::paired_devices::MAX_CONFIG_BYTES;
use std::{io::Write, path::Path};

impl FileStorage {
    pub fn open(path: &Path) -> Result<Self, &'static str> {
        if !path.is_file() {
            return Err("Host configuration is missing.");
        }
        // ? A save that crashed mid-write leaves its temp file behind; the names are tempfile's, so
        // ? only files with that exact shape are removed.
        if let Some(entries) = path.parent().and_then(|dir| std::fs::read_dir(dir).ok()) {
            for entry in entries.flatten() {
                if is_leftover_temp(&entry.file_name().to_string_lossy()) {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    pub fn read(&self) -> Result<Vec<u8>, &'static str> {
        read_bounded(&self.path, MAX_CONFIG_BYTES).ok_or("Cannot read host configuration.")
    }

    /// Writes `candidate` only if the file still holds `expected`. It writes a
    /// temp file in the same folder, flushes it, then renames it over the
    /// original.
    pub fn replace(&self, expected: &[u8], candidate: &[u8]) -> Result<(), StorageError> {
        // ! An unreadable file is an I/O failure, not a conflict: the caller re- reads and can tell
        // ! an unchanged file from a changed one.
        let current = read_bounded(&self.path, MAX_CONFIG_BYTES).ok_or(StorageError::Io)?;
        if current != expected {
            return Err(StorageError::Conflict);
        }
        let directory = self.path.parent().ok_or(StorageError::Io)?;
        let mut staged =
            tempfile::NamedTempFile::new_in(directory).map_err(|_| StorageError::Io)?;
        staged.write_all(candidate).map_err(|_| StorageError::Io)?;
        staged.as_file().sync_all().map_err(|_| StorageError::Io)?;
        staged.persist(&self.path).map_err(|_| StorageError::Io)?;
        Ok(())
    }
}

/// `.tmp` and six letters or digits, the shape tempfile gives its files.
fn is_leftover_temp(name: &str) -> bool {
    name.strip_prefix(".tmp").is_some_and(|suffix| {
        suffix.len() == 6 && suffix.bytes().all(|b| b.is_ascii_alphanumeric())
    })
}
