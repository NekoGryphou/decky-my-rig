pub(super) const UNAVAILABLE: &str = "My Rig service is not available.";
pub(super) const UNAUTHORIZED: &str = "Not authorized. Ask an administrator to add you to My Rig Managers, then sign out and back in.";

pub(super) fn host_error(code: &str) -> String {
    format!("Host error: {code}")
}
