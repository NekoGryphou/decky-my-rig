use std::time::Duration;

/// Minimum time between accepted requests, so nobody on the network can flood
/// the PC with notifications.
pub(crate) const REQUEST_INTERVAL: Duration = Duration::from_secs(10);
/// The longest wait failed guesses can cause, which works out to about 72
/// guesses a day.
pub(super) const MAX_REQUEST_INTERVAL: Duration = Duration::from_secs(3600);
