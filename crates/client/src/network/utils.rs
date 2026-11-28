use super::{
    consts::{MAGIC_PACKET_SIZE, USABLE_PREFIXES},
    errors::WakeError,
    types::Interface,
};
use crate::shared::normalize_mac;
use std::{collections::BTreeSet, net::Ipv4Addr};

/// Six 0xFF bytes, then the MAC address sixteen times.
pub(super) fn magic_packet(mac: &str) -> Result<[u8; MAGIC_PACKET_SIZE], WakeError> {
    let mac = normalize_mac(mac).map_err(|_| WakeError::InvalidMac)?;
    let mut octets = [0u8; 6];
    for (octet, pair) in octets.iter_mut().zip(mac.split(':')) {
        *octet = u8::from_str_radix(pair, 16).map_err(|_| WakeError::InvalidMac)?;
    }
    let mut packet = [0xff; MAGIC_PACKET_SIZE];
    for chunk in packet[6..].as_chunks_mut::<6>().0 {
        *chunk = octets;
    }
    Ok(packet)
}

/// The interface an address and netmask describe, if it can send a directed
/// broadcast: the mask has to be contiguous with room for hosts, and the
/// address has to be a host in it. The platform already filtered out interfaces
/// that are down or can't broadcast.
pub(super) fn interface_from(address: Ipv4Addr, netmask: Ipv4Addr) -> Option<Interface> {
    if !usable_unicast(address) {
        return None;
    }
    let mask = u32::from(netmask);
    let prefix = u8::try_from(mask.leading_ones()).ok()?;
    if mask.checked_shl(u32::from(prefix)).unwrap_or(0) != 0 || !USABLE_PREFIXES.contains(&prefix) {
        return None;
    }
    let interface = Interface { address, prefix };
    is_host(address, interface).then_some(interface)
}

/// The source address and directed broadcast of the interface with the longest
/// prefix whose subnet has `target` as a host.
pub(super) fn select_directed_broadcast(
    target: Ipv4Addr,
    interfaces: &[Interface],
) -> Result<(Ipv4Addr, Ipv4Addr), WakeError> {
    let candidates: BTreeSet<Interface> = interfaces
        .iter()
        .copied()
        .filter(|interface| USABLE_PREFIXES.contains(&interface.prefix))
        .filter(|interface| usable_unicast(target) && is_host(target, *interface))
        .collect();
    // ? Interfaces with the longest prefix that contain the PC are on its subnet, so they share one
    // ? broadcast address (Ethernet and Wi-Fi on a docked Deck). We send from the lowest address.
    let best = candidates
        .iter()
        .map(|interface| interface.prefix)
        .max()
        .ok_or(WakeError::NoInterface)?;
    candidates
        .iter()
        .find(|interface| interface.prefix == best)
        .map(|interface| (interface.address, broadcast(*interface)))
        .ok_or(WakeError::NoInterface)
}

fn usable_unicast(address: Ipv4Addr) -> bool {
    !address.is_unspecified() && !address.is_multicast() && !address.is_broadcast()
}

fn mask(prefix: u8) -> u32 {
    u32::MAX.checked_shl(32 - u32::from(prefix)).unwrap_or(0)
}

fn broadcast(interface: Interface) -> Ipv4Addr {
    Ipv4Addr::from(u32::from(interface.address) | !mask(interface.prefix))
}

/// True if the address is in the interface's subnet and isn't its network or
/// broadcast address.
fn is_host(address: Ipv4Addr, interface: Interface) -> bool {
    let mask = mask(interface.prefix);
    let network = u32::from(interface.address) & mask;
    let value = u32::from(address);
    value & mask == network && value != network && value != network | !mask
}
