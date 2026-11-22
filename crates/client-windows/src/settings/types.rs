use std::{fs::File, path::PathBuf, sync::Mutex};

/// An opened settings directory and, once locked, its lock file.
pub(crate) struct WindowsSettings {
    pub(super) directory: PathBuf,
    /// Held open without delete sharing, so the folder can't be renamed or
    /// removed while it's in use.
    pub(super) _handle: File,
    pub(super) owners: Vec<String>,
    pub(super) lock: Mutex<Option<File>>,
}
