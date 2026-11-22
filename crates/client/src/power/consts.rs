use std::time::Duration;

pub(super) const STATUS_DEADLINE: Duration = Duration::from_secs(3);
pub(super) const TRANSITION_TIMEOUT: Duration = Duration::from_secs(120);
pub(super) const MAX_STATUS_PROBES: usize = 4;
/// A PC only shows as offline after this many failed checks in a row.
pub(super) const OFFLINE_AFTER_FAILED_PROBES: u32 = 2;
