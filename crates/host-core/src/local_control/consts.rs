use std::time::Duration;

/// How long a notifier's event request waits for news before getting an empty
/// reply.
pub(super) const EVENT_WAIT: Duration = Duration::from_secs(25);

/// Room for the whole open-pairing reply, so writing it never reallocates.
pub(super) const OPEN_PAIRING_REPLY_CAPACITY: usize = 256;
