use super::types::ServiceExit;
use crate::install_directory::InstallDirectory;
use std::path::Path;

/// The identity is only trusted from a folder that SYSTEM and Administrators
/// own and nobody else can write. `load_security` only parses the file, so
/// this is what stops a planted identity from being served.
pub(super) fn check_security_folder(folder: &Path) -> Result<(), ServiceExit> {
    InstallDirectory::validate_data_folder(folder).map_err(|_| ServiceExit::Identity)
}
