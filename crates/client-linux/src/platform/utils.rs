use super::consts::MAX_HOSTNAME_BYTES;
use myrig_client::{platform::SETTINGS_FOLDER, shared::read_bounded};
use std::{
    ffi::OsString,
    fs::File,
    path::{Path, PathBuf},
};

/// `$XDG_CONFIG_HOME/decky-my-rig` if that's an absolute path, otherwise
/// `$HOME/.config/decky-my-rig`.
pub(super) fn config_dir_from(
    xdg_config_home: Option<OsString>,
    home: Option<OsString>,
) -> Option<PathBuf> {
    let base = xdg_config_home
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| home.map(|home| PathBuf::from(home).join(".config")))?;
    Some(base.join(SETTINGS_FOLDER))
}

/// The hostname file's text, or `None` if it can't be read or isn't text.
pub(super) fn read_hostname(path: &Path) -> Option<String> {
    let raw = read_bounded(File::open(path).ok()?, MAX_HOSTNAME_BYTES).ok()?;
    String::from_utf8(raw).ok()
}
