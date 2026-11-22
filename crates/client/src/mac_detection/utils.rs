use super::messages::DETECTION_FAILED;
use crate::shared::{PublicError, normalize_mac};
use std::collections::BTreeSet;

/// Picks the one valid address among the neighbor entries. If there are several
/// different ones we can't tell, and the user has to type it.
pub(super) fn select_mac(macs: &[String]) -> Result<String, PublicError> {
    let matches: BTreeSet<String> = macs
        .iter()
        .filter_map(|mac| normalize_mac(mac).ok())
        .collect();
    let mut matches = matches.into_iter();
    match (matches.next(), matches.next()) {
        (Some(mac), None) => Ok(mac),
        _ => Err(PublicError(DETECTION_FAILED)),
    }
}
