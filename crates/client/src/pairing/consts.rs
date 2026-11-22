use std::time::Duration;

pub(super) const PAIR_DEADLINE: Duration = Duration::from_secs(10);
pub(super) const CODE_REQUEST_DEADLINE: Duration = Duration::from_secs(5);
/// How long we wait for the PC to revoke an unsaved pairing we're dropping.
pub(super) const REVOKE_DEADLINE: Duration = Duration::from_secs(3);
/// 202 means the PC shows a new code, 409 means a code is already showing.
pub(super) const CODE_SHOWN_STATUSES: [u16; 2] = [202, 409];
pub(super) const BOOTSTRAP_VERSION: u64 = 1;
pub(super) const FALLBACK_DEVICE_NAME: &str = "My device";
/// The PC holds on to its code briefly while it handles another request.
pub(super) const BUSY_STATUS: u16 = 503;
pub(super) const BUSY_RETRY: Duration = Duration::from_millis(250);
