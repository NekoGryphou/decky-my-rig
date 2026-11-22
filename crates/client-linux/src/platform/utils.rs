use myrig_client::platform::SETTINGS_FOLDER;
use std::{ffi::OsString, path::PathBuf};

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
