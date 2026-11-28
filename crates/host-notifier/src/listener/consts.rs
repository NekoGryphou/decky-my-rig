use std::time::Duration;

pub(super) const FIRST_BACKOFF: Duration = Duration::from_secs(1);
pub(super) const MAX_BACKOFF: Duration = Duration::from_secs(30);
/// A pairing request older than this has no code left on the PC (the PC's
/// pairing sessions last 120 seconds), so showing a new code for it is wrong.
pub(super) const PAIRING_REQUEST_MAX_AGE: Duration = Duration::from_secs(120);
/// The service holds a wait for up to 25 seconds, so this allows for that and a
/// margin for the reply to arrive.
pub(super) const WAIT_TIMEOUT: Duration = Duration::from_secs(35);
pub(super) const OPEN_TIMEOUT: Duration = Duration::from_secs(5);
