use super::{native::neighbor_macs, utils::ethernet_mac};
use std::net::Ipv4Addr;

#[test]
fn only_six_byte_addresses_are_ethernet() {
    assert_eq!(
        ethernet_mac(&[0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0x01]).as_deref(),
        Some("02:AA:BB:CC:DD:01")
    );
    assert_eq!(ethernet_mac(&[]), None);
    assert_eq!(ethernet_mac(&[0; 8]), None);
}

#[test]
fn the_neighbor_table_can_be_read() {
    assert!(
        neighbor_macs(Ipv4Addr::new(192, 0, 2, 1))
            .unwrap()
            .is_empty()
    );
}
