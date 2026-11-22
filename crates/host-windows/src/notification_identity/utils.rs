use super::consts::APP_ID_ROOT;

/// Registry path under the current user that names a notification sender.
pub(super) fn key_path(app_id: &str) -> Result<String, &'static str> {
    if app_id.is_empty() || app_id.contains(['\\', '\0']) {
        return Err("Invalid notification identity.");
    }
    Ok(format!(r"{APP_ID_ROOT}\{app_id}"))
}
