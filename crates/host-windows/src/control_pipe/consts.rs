use std::time::Duration;

pub(super) const DEADLINE: Duration = Duration::from_secs(2);
pub(super) const REVERT_FAILURE: &str = "Local control impersonation recovery failed.";
pub(super) const MANAGERS_GROUP: &str = "My Rig Managers";
/// Where the installer records the users it made managers. Only administrators
/// can write it.
pub(super) const MANAGERS_KEY: &str = r"Software\MyRig";
pub(super) const MANAGER_USERS_VALUE: &str = "ManagerUsers";
pub(super) const MAX_RECORDED_MANAGERS: usize = 64;
pub(super) const UNAUTHORIZED: &[u8] = br#"{"version":1,"ok":false,"error":"unauthorized"}"#;
pub(super) const INVALID_RESPONSE: &[u8] = br#"{"version":1,"ok":false,"error":"control_failed"}"#;

pub(super) const NORMAL_SLOTS: usize = 4;
pub(super) const EVENT_SLOTS: usize = 4;
pub(super) const READING_SLOTS: usize = 4;
pub(super) const MAX_ACTIVE: usize = NORMAL_SLOTS + EVENT_SLOTS + READING_SLOTS;
pub(super) const CAPACITY_UNAVAILABLE: &[u8] = br#"{"version":1,"ok":false,"error":"unavailable"}"#;
