use super::consts::{MAX_REQUEST_INTERVAL, REQUEST_INTERVAL};
use myrig_pairing_crypto::session::MAX_STARTS;
use std::time::Duration;

/// How long before the network can ask for another code. It doubles for every
/// code's worth of failed guesses since the last successful pairing.
pub(super) fn request_interval(failed_starts: u32) -> Duration {
    let spent_codes = (failed_starts / u32::from(MAX_STARTS)).min(16);
    REQUEST_INTERVAL
        .saturating_mul(1 << spent_codes)
        .min(MAX_REQUEST_INTERVAL)
}
