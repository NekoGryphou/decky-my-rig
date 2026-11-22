use super::{
    consts::{MAGIC_PACKET_SIZE, SEND_TIMEOUT, WOL_PORT},
    errors::WakeError,
    types::Interface,
    utils::{interface_from, magic_packet, select_directed_broadcast},
};
use crate::platform::Platform;
use std::{
    collections::BTreeSet,
    net::{Ipv4Addr, SocketAddrV4},
};
use tokio::{net::UdpSocket, time::timeout};

/// Sends one magic packet for `mac` to `target`'s subnet. It's never retried.
pub async fn wake(platform: &dyn Platform, mac: &str, target: Ipv4Addr) -> Result<(), WakeError> {
    let packet = magic_packet(mac)?;
    let interfaces = broadcast_interfaces(platform).map_err(|_| WakeError::Send)?;
    let (source, broadcast) = select_directed_broadcast(target, &interfaces)?;
    send_packet(&packet, source, SocketAddrV4::new(broadcast, WOL_PORT)).await
}

/// This machine's usable interfaces, without duplicates, sorted by address.
pub(super) fn broadcast_interfaces(platform: &dyn Platform) -> std::io::Result<Vec<Interface>> {
    let interfaces: BTreeSet<Interface> = platform
        .broadcast_interfaces()?
        .into_iter()
        .filter_map(|(address, netmask)| interface_from(address, netmask))
        .collect();
    Ok(interfaces.into_iter().collect())
}

pub(super) async fn send_packet(
    packet: &[u8; MAGIC_PACKET_SIZE],
    source: Ipv4Addr,
    destination: SocketAddrV4,
) -> Result<(), WakeError> {
    let send = async {
        let socket = UdpSocket::bind(SocketAddrV4::new(source, 0)).await?;
        socket.set_broadcast(true)?;
        socket.send_to(packet, destination).await
    };
    match timeout(SEND_TIMEOUT, send).await {
        Ok(Ok(MAGIC_PACKET_SIZE)) => Ok(()),
        _ => Err(WakeError::Send),
    }
}
