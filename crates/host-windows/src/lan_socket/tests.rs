use super::bind_exclusive;
use std::net::{Ipv4Addr, TcpListener};

#[test]
fn no_other_socket_can_take_the_port_on_one_address() {
    let listener = bind_exclusive(0).unwrap();
    let port = listener.local_addr().unwrap().port();
    // ? Without exclusive use, Windows lets the more specific address win.
    assert!(TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_err());
}
