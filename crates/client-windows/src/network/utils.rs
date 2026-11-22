use std::net::Ipv4Addr;

/// Netmask for an on-link prefix length. Longer prefixes are clamped.
pub(super) fn netmask(prefix: u8) -> Ipv4Addr {
    Ipv4Addr::from(
        u32::MAX
            .checked_shl(32 - u32::from(prefix.min(32)))
            .unwrap_or(0),
    )
}
