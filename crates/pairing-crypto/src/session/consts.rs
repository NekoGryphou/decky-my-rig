use std::time::Duration;

pub const LIFETIME: Duration = Duration::from_secs(120);
pub(super) const ATTEMPT_LIFETIME: Duration = Duration::from_secs(15);
/// How many guesses one code allows. A real device only needs one.
pub const MAX_STARTS: u8 = 3;
pub(super) const RECORD_ID: &[u8] = b"local-session";
