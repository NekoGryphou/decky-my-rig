use super::{
    Interface, WakeError, broadcast_interfaces, interface_from, magic_packet,
    select_directed_broadcast, send_packet,
};
use crate::platform::fixtures::TestPlatform;
use std::net::{Ipv4Addr, SocketAddrV4};

fn ip(text: &str) -> Ipv4Addr {
    text.parse().unwrap()
}

fn interface(address: &str, prefix: u8) -> Interface {
    Interface {
        address: ip(address),
        prefix,
    }
}

fn expected_packet() -> Vec<u8> {
    let mut packet = vec![0xff; 6];
    for _ in 0..16 {
        packet.extend_from_slice(&[0x02, 0, 0, 0, 0, 0x01]);
    }
    packet
}

#[test]
fn three_supported_mac_forms_produce_the_exact_packet() {
    for source in ["020000000001", "02-00-00-00-00-01", " 02:00:00:00:00:01 "] {
        assert_eq!(
            magic_packet(source).unwrap().to_vec(),
            expected_packet(),
            "{source}"
        );
    }
}

#[test]
fn invalid_macs_are_refused() {
    for source in ["00:00:00:00:00:00", "01:00:5E:00:00:01", "private-invalid"] {
        assert!(magic_packet(source).is_err(), "{source}");
    }
}

#[test]
fn directed_broadcast_prefers_the_target_subnet() {
    let interfaces = [
        interface("192.0.0.5", 8),
        interface("192.168.0.5", 16),
        interface("192.168.12.5", 24),
    ];
    assert_eq!(
        select_directed_broadcast(ip("192.168.12.42"), &interfaces),
        Ok((ip("192.168.12.5"), ip("192.168.12.255")))
    );
    // - Wi-Fi and USB Ethernet on different subnets: the one on the PC's subnet
    //   wins.
    let device = [interface("192.168.1.20", 24), interface("10.0.0.7", 24)];
    assert_eq!(
        select_directed_broadcast(ip("10.0.0.40"), &device),
        Ok((ip("10.0.0.7"), ip("10.0.0.255")))
    );
}

#[test]
fn duplicate_interfaces_collapse_and_ties_share_one_broadcast() {
    let same = [interface("10.2.3.1", 24), interface("10.2.3.1", 24)];
    assert_eq!(
        select_directed_broadcast(ip("10.2.3.4"), &same),
        Ok((ip("10.2.3.1"), ip("10.2.3.255")))
    );
    // - A docked Deck has Ethernet and Wi-Fi on the PC's subnet with the same
    //   broadcast address, so either one can wake it.
    let tie = [interface("192.168.1.3", 24), interface("192.168.1.2", 24)];
    assert_eq!(
        select_directed_broadcast(ip("192.168.1.20"), &tie),
        Ok((ip("192.168.1.2"), ip("192.168.1.255")))
    );
    let elsewhere = [interface("10.0.0.2", 24)];
    assert_eq!(
        select_directed_broadcast(ip("192.168.1.20"), &elsewhere),
        Err(WakeError::NoInterface)
    );
}

#[test]
fn network_and_broadcast_targets_are_not_hosts() {
    let interfaces = [interface("192.168.1.2", 24)];
    for target in [
        "192.168.1.0",
        "192.168.1.255",
        "0.0.0.0",
        "224.0.0.1",
        "255.255.255.255",
    ] {
        assert_eq!(
            select_directed_broadcast(ip(target), &interfaces),
            Err(WakeError::NoInterface),
            "{target}"
        );
    }
}

#[test]
fn only_unicast_hosts_with_usable_prefixes_qualify() {
    assert_eq!(
        interface_from(ip("10.2.3.5"), ip("255.255.255.0")),
        Some(interface("10.2.3.5", 24))
    );
    assert_eq!(
        interface_from(ip("10.0.0.2"), ip("255.0.0.0")),
        Some(interface("10.0.0.2", 8))
    );
    for (address, netmask) in [
        ("0.0.0.0", "255.255.255.0"),
        ("224.0.0.1", "255.255.255.0"),
        ("10.0.0.2", "255.0.255.0"),
        ("10.0.0.2", "0.0.0.0"),
        ("10.0.0.2", "255.255.255.254"),
        ("10.0.0.2", "255.255.255.255"),
        ("10.0.0.0", "255.255.255.0"),
        ("10.0.0.255", "255.255.255.0"),
    ] {
        assert_eq!(
            interface_from(ip(address), ip(netmask)),
            None,
            "{address}/{netmask}"
        );
    }
}

#[tokio::test]
async fn one_magic_packet_reaches_the_destination() {
    let receiver = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let port = receiver.local_addr().unwrap().port();
    let packet = magic_packet("02:00:00:00:00:01").unwrap();
    send_packet(
        &packet,
        Ipv4Addr::LOCALHOST,
        SocketAddrV4::new(Ipv4Addr::LOCALHOST, port),
    )
    .await
    .unwrap();
    let mut buffer = [0u8; 256];
    let (length, sender) = receiver.recv_from(&mut buffer).await.unwrap();
    assert_eq!(buffer[..length].to_vec(), expected_packet());
    assert_eq!(sender.ip(), std::net::IpAddr::V4(Ipv4Addr::LOCALHOST));
}

#[test]
fn the_platform_interfaces_are_deduplicated_in_order() {
    let platform = TestPlatform {
        interfaces: vec![
            (ip("192.168.1.20"), ip("255.255.255.0")),
            (ip("10.0.0.7"), ip("255.255.255.0")),
            (ip("192.168.1.20"), ip("255.255.255.0")),
            (ip("10.0.0.0"), ip("255.255.255.0")),
        ],
        ..TestPlatform::default()
    };
    assert_eq!(
        broadcast_interfaces(&platform).unwrap(),
        [interface("10.0.0.7", 24), interface("192.168.1.20", 24)]
    );
}
