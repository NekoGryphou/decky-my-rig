use std::time::Duration;

pub(super) const FIRST_BACKOFF: Duration = Duration::from_secs(1);
pub(super) const MAX_BACKOFF: Duration = Duration::from_secs(30);
/// A listener that has served this long resets the failure streak.
pub(super) const HEALTHY: Duration = MAX_BACKOFF;
