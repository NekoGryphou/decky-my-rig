// ! The record of the inputs an agent was built from. A stamp beside the agent holds the hash of
// ! those inputs, so the check compares contents: a checkout or an artifact download can give every
// ! file a new timestamp.
use super::utils::hex_sha256;
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

/// The hash of the workspace files and of each crate's `Cargo.toml`, `build.rs`
/// and `src/`. Paths are sorted and written relative to `root`, so the hash
/// depends on what the files say and where they are, not on when they were
/// written or where the checkout lives.
pub(super) fn input_hash(root: &Path, crate_dirs: &[PathBuf]) -> Result<String, String> {
    // ? Both sides get one spelling, or names that should be relative come out absolute.
    let root = root
        .canonicalize()
        .map_err(|error| format!("{}: {error}", root.display()))?;
    let crate_dirs = crate_dirs
        .iter()
        .map(|dir| {
            dir.canonicalize()
                .map_err(|error| format!("{}: {error}", dir.display()))
        })
        .collect::<Result<Vec<PathBuf>, String>>()?;
    let root = root.as_path();
    let mut paths: Vec<PathBuf> = ["Cargo.toml", "Cargo.lock"]
        .iter()
        .map(|name| root.join(name))
        .collect();
    for directory in &crate_dirs {
        paths.push(directory.join("Cargo.toml"));
        paths.push(directory.join("build.rs"));
        paths.push(directory.join("src"));
    }
    let mut files = Vec::new();
    for path in &paths {
        collect_files(path, &mut files)?;
    }
    let mut named: Vec<(String, PathBuf)> = files
        .into_iter()
        .map(|path| (relative_name(root, &path), path))
        .collect();
    named.sort();
    let mut bytes = Vec::new();
    for (name, path) in named {
        let data = fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        // - Length prefixes keep a name and its contents from running together.
        bytes.extend((name.len() as u64).to_le_bytes());
        bytes.extend(name.as_bytes());
        bytes.extend((data.len() as u64).to_le_bytes());
        bytes.extend(data);
    }
    Ok(hex_sha256(&bytes))
}

/// Where the stamp for `binary` lives: the same folder, `<file name>.stamp`.
pub(super) fn stamp_path(binary: &Path) -> PathBuf {
    let name = binary
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    binary.with_file_name(format!("{name}.stamp"))
}

pub(super) fn write_stamp(binary: &Path, hash: &str) -> Result<(), String> {
    let path = stamp_path(binary);
    fs::write(&path, format!("{hash}\n")).map_err(|error| format!("{}: {error}", path.display()))
}

/// Refuses an agent whose stamp is missing or holds another hash than the
/// current inputs.
pub(super) fn check_stamp(binary: &Path, hash: &str) -> Result<(), String> {
    let path = stamp_path(binary);
    let recorded = match fs::read_to_string(&path) {
        Ok(recorded) => recorded,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(format!(
                "{} has no build stamp: rebuild it before packing",
                binary.display()
            ));
        }
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    if recorded.trim() == hash {
        Ok(())
    } else {
        Err(format!(
            "{} was built from other inputs: rebuild it before packing",
            binary.display()
        ))
    }
}

/// Adds the files under `path`, or nothing when it does not exist: an optional
/// `build.rs` is not an input unless it is there.
fn collect_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    if metadata.is_dir() {
        let entries = fs::read_dir(path).map_err(|error| format!("{}: {error}", path.display()))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("{}: {error}", path.display()))?;
            collect_files(&entry.path(), files)?;
        }
    } else {
        files.push(path.to_path_buf());
    }
    Ok(())
}

fn relative_name(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .map(|part| part.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}
