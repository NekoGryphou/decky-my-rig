use std::time::Duration;

pub(super) const WOL_PORT: u16 = 9;
pub(super) const MAGIC_PACKET_SIZE: usize = 102;
pub(super) const SEND_TIMEOUT: Duration = Duration::from_secs(1);
/// Prefix lengths that leave room for hosts besides the network and broadcast
/// addresses.
pub(super) const USABLE_PREFIXES: std::ops::RangeInclusive<u8> = 1..=30;
