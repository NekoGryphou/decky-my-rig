use std::path::PathBuf;

/// The service's config file, replaced atomically.
pub struct FileStorage {
    pub(super) path: PathBuf,
}
