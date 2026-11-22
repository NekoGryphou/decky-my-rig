//! Registers the notifier's notification identity in the user's registry, so
//! its notifications work without a Start menu shortcut.
use super::utils::key_path;
use windows_registry::CURRENT_USER;

pub fn register(app_id: &str, display_name: &str) -> Result<(), &'static str> {
    let key = CURRENT_USER
        .create(key_path(app_id)?)
        .map_err(|_| "Cannot register notification identity.")?;
    key.set_string("DisplayName", display_name)
        .map_err(|_| "Cannot register notification identity.")
}

pub fn unregister(app_id: &str) -> Result<(), &'static str> {
    let path = key_path(app_id)?;
    if CURRENT_USER.open(&path).is_err() {
        return Ok(());
    }
    CURRENT_USER
        .remove_tree(&path)
        .map_err(|_| "Cannot remove notification identity.")
}
