use super::{native::broadcast_interfaces, utils::netmask};
use std::net::Ipv4Addr;

#[test]
fn prefixes_become_netmasks() {
    assert_eq!(netmask(24), Ipv4Addr::new(255, 255, 255, 0));
    assert_eq!(netmask(8), Ipv4Addr::new(255, 0, 0, 0));
    assert_eq!(netmask(0), Ipv4Addr::UNSPECIFIED);
    assert_eq!(netmask(32), Ipv4Addr::BROADCAST);
    assert_eq!(netmask(255), Ipv4Addr::BROADCAST);
}

#[test]
fn this_machine_lists_no_loopback_interface() {
    for (address, _) in broadcast_interfaces().unwrap() {
        assert!(!address.is_loopback(), "{address}");
    }
}
