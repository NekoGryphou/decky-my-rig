use std::time::Duration;

/// How long a notifier's event request waits for news before getting an empty
/// reply.
pub(super) const EVENT_WAIT: Duration = Duration::from_secs(25);
