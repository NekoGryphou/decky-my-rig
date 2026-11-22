use super::{native::ipv4_addresses, utils::can_broadcast};
use std::{io, net::Ipv4Addr};

/// Address and netmask of every interface that's up, can broadcast, and isn't
/// loopback or point-to-point.
pub(crate) fn broadcast_interfaces() -> io::Result<Vec<(Ipv4Addr, Ipv4Addr)>> {
    Ok(ipv4_addresses()?
        .into_iter()
        .filter(|(flags, _, _)| can_broadcast(*flags))
        .map(|(_, address, netmask)| (address, netmask))
        .collect())
}
