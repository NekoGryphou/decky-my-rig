use super::messages::INVALID_PATH;

pub(super) fn quoted_launch_path(path: &str) -> Result<String, &'static str> {
    if path.is_empty() || path.contains(['"', '\r', '\n']) {
        return Err(INVALID_PATH);
    }
    Ok(format!("\"{path}\""))
}
