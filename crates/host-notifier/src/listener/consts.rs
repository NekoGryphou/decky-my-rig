use std::time::Duration;

pub(super) const FIRST_BACKOFF: Duration = Duration::from_secs(1);
pub(super) const MAX_BACKOFF: Duration = Duration::from_secs(30);
