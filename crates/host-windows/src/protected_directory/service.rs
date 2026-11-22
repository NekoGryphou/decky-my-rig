use super::{consts::ADMINISTRATORS_ONLY, native::create_directory};
use std::{io, path::Path};

/// Creates new data folders, in order, that only SYSTEM and Administrators can
/// open. A folder that already exists is refused, never taken over, and if
/// anything fails, the folders this call created are removed again.
pub fn create_protected_directories(paths: &[&Path]) -> io::Result<()> {
    for (index, path) in paths.iter().enumerate() {
        let created = if path.to_str().is_none() {
            Err(io::ErrorKind::InvalidInput.into())
        } else {
            create_directory(path, ADMINISTRATORS_ONLY)
        };
        if let Err(error) = created {
            for made in paths[..index].iter().rev() {
                let _ = std::fs::remove_dir(made);
            }
            return Err(error);
        }
    }
    Ok(())
}
