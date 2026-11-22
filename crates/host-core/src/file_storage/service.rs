use super::{errors::StorageError, types::FileStorage, utils::read_bounded};
use crate::paired_devices::MAX_CONFIG_BYTES;
use std::{io::Write, path::Path};

impl FileStorage {
    pub fn open(path: &Path) -> Result<Self, &'static str> {
        if !path.is_file() {
            return Err("Host configuration is missing.");
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
        if read_bounded(&self.path, MAX_CONFIG_BYTES).as_deref() != Some(expected) {
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
