pub(super) const ARP_CACHE: &str = "/proc/net/arp";
pub(super) const MAX_CACHE_BYTES: u64 = 1_048_576;
/// ARP hardware type for Ethernet, and the "complete" entry flag.
pub(super) const ETHERNET: u32 = 0x1;
pub(super) const COMPLETE: u32 = 0x2;
