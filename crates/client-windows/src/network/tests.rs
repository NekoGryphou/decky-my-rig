use super::{
    native::broadcast_interfaces,
    utils::{MAX_ADAPTER_QUERIES, fill_adapter_buffer, netmask},
};
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

#[test]
fn an_adapter_list_that_keeps_growing_is_an_error() {
    use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_INVALID_DATA};
    let mut calls = 0;
    let error = fill_adapter_buffer(|buffer, size| {
        calls += 1;
        *size = (buffer.len() * 8 * 2) as u32;
        // ? Stops an uncapped loop, which would otherwise never end.
        if calls > 100 {
            ERROR_INVALID_DATA.0
        } else {
            ERROR_BUFFER_OVERFLOW.0
        }
    })
    .unwrap_err();
    assert_eq!(error.raw_os_error(), Some(ERROR_BUFFER_OVERFLOW.0 as i32));
    assert_eq!(calls, MAX_ADAPTER_QUERIES);
}

#[test]
fn a_list_that_outgrew_its_buffer_once_is_read_with_the_larger_one() {
    use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS};
    let mut sizes = Vec::new();
    let buffer = fill_adapter_buffer(|buffer, size| {
        sizes.push(*size);
        if sizes.len() == 1 {
            *size = 100_000;
            ERROR_BUFFER_OVERFLOW.0
        } else {
            assert!(buffer.len() * 8 >= 100_000);
            ERROR_SUCCESS.0
        }
    })
    .unwrap();
    assert_eq!(sizes.len(), 2);
    assert!(buffer.len() * 8 >= 100_000);
}
