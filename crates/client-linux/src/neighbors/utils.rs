use super::consts::{COMPLETE, ETHERNET};
use std::net::Ipv4Addr;

/// The hardware address of each complete Ethernet row for `host` in
/// `/proc/net/arp`.
pub(super) fn complete_ethernet_entries(cache: &str, host: Ipv4Addr) -> Vec<String> {
    let host = host.to_string();
    let hex = |text: &str| u32::from_str_radix(text.strip_prefix("0x")?, 16).ok();
    cache
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>())
        .filter(|fields| fields.len() == 6 && fields[0] == host)
        .filter(|fields| {
            hex(fields[1]) == Some(ETHERNET)
                && hex(fields[2]).is_some_and(|flags| flags & COMPLETE != 0)
        })
        .map(|fields| fields[3].to_owned())
        .collect()
}
