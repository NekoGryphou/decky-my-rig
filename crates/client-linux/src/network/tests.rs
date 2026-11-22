use super::{service::broadcast_interfaces, utils::can_broadcast};

const UP_BROADCAST: u32 = (libc::IFF_UP | libc::IFF_BROADCAST) as u32;

#[test]
fn only_up_broadcast_interfaces_that_are_not_loopback_or_point_to_point_qualify() {
    assert!(can_broadcast(UP_BROADCAST));
    for flags in [
        libc::IFF_BROADCAST as u32,
        libc::IFF_UP as u32,
        UP_BROADCAST | libc::IFF_LOOPBACK as u32,
        UP_BROADCAST | libc::IFF_POINTOPOINT as u32,
    ] {
        assert!(!can_broadcast(flags), "{flags:#x}");
    }
}

#[test]
fn this_machine_lists_its_interfaces() {
    for (address, _) in broadcast_interfaces().unwrap() {
        assert!(!address.is_loopback(), "{address}");
    }
}
