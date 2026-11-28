use std::time::Duration;

pub(super) const OK: u8 = 0;
pub(super) const REFUSED: u8 = 1;
pub(super) const USAGE: u8 = 2;
pub(super) const UNREACHABLE: u8 = 3;
pub(super) const DEFAULT_PORT: &str = "47991";
/// The longest a finished command waits for a save that is still blocked.
pub(super) const SHUTDOWN_BOUND: Duration = Duration::from_secs(10);
