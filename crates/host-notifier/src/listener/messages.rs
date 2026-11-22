pub const UNAUTHORIZED_TITLE: &str = "My Rig notifications are off";
pub const UNAUTHORIZED: &str =
    "Ask an administrator to add you to My Rig Managers, then sign out and back in.";
pub(super) const TEST_SHUTDOWN_BODY: &str = "You can turn off the test mode in the settings.";

pub(super) fn test_shutdown_title(device_name: &str) -> String {
    format!("Test shutdown from “{device_name}”.")
}

/// Kept in one piece, so Windows' copy-code action copies all eight digits.
pub(super) fn pairing_code_title(code: &str) -> String {
    format!("Pairing code: {code}")
}

pub(super) fn pairing_code_body(device_name: &str) -> String {
    format!("“{device_name}” wants to pair with this PC.")
}
