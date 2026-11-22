use super::{
    consts::FAILURE,
    native,
    types::{InstallDirectory, PathRole},
    utils::{check_path, check_security},
};
use std::{
    fs,
    path::{Path, PathBuf},
};

impl InstallDirectory {
    /// Read-only check used by the installer. It doesn't fix ACLs on unrelated
    /// folders.
    pub fn validate(path: &Path) -> Result<(), &'static str> {
        check_path(path)?;
        native::check_drive(path.ancestors().last().ok_or(FAILURE)?)?;
        let ancestors: Vec<_> = path.ancestors().collect();
        let mut previous = None;
        for current in ancestors.into_iter().rev() {
            match native::metadata(current) {
                Ok(metadata) => {
                    if !metadata.is_dir() {
                        return Err(FAILURE);
                    }
                    let role = if current == path {
                        PathRole::Destination
                    } else {
                        PathRole::Ancestor
                    };
                    check_security(&native::read(current)?, role)?;
                    previous = Some(current);
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    check_security(
                        &native::read(previous.ok_or(FAILURE)?)?,
                        PathRole::CreationParent,
                    )?;
                    return Ok(());
                }
                Err(_) => return Err(FAILURE),
            }
        }
        Self::check_contents(path)
    }

    fn check_contents(path: &Path) -> Result<(), &'static str> {
        let mut folders = vec![PathBuf::from(path)];
        while let Some(folder) = folders.pop() {
            for entry in fs::read_dir(folder).map_err(|_| FAILURE)? {
                let path = entry.map_err(|_| FAILURE)?.path();
                let metadata = native::metadata(&path).map_err(|_| FAILURE)?;
                let role = if metadata.is_dir() {
                    PathRole::Destination
                } else {
                    PathRole::File
                };
                check_security(&native::read(&path)?, role)?;
                if metadata.is_dir() {
                    folders.push(path);
                }
            }
        }
        Ok(())
    }
}
