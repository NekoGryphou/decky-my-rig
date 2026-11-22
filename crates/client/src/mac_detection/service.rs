use super::{messages::DETECTION_FAILED, utils::select_mac};
use crate::{platform::Platform, shared::PublicError};
use std::net::Ipv4Addr;

pub fn detect_mac(platform: &dyn Platform, host: Ipv4Addr) -> Result<String, PublicError> {
    let macs = platform
        .neighbor_macs(host)
        .map_err(|_| PublicError(DETECTION_FAILED))?;
    select_mac(&macs)
}
