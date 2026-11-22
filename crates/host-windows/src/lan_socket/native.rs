use socket2::{Domain, Protocol, Socket, Type};
use std::{
    io,
    net::{Ipv4Addr, SocketAddr, TcpListener},
    os::windows::io::AsRawSocket,
};
use windows::Win32::Networking::WinSock::{SO_EXCLUSIVEADDRUSE, SOCKET, SOL_SOCKET, setsockopt};

/// Binds `port` on every IPv4 interface so no other socket, even one bound to a
/// single address, can grab the port's traffic.
pub fn bind_exclusive(port: u16) -> io::Result<TcpListener> {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
    let enabled = 1i32.to_ne_bytes();
    // ! SAFETY: the socket is open for the call and `enabled` is a live BOOL-sized buffer.
    let result = unsafe {
        setsockopt(
            SOCKET(socket.as_raw_socket() as usize),
            SOL_SOCKET,
            SO_EXCLUSIVEADDRUSE,
            Some(&enabled),
        )
    };
    if result != 0 {
        return Err(io::Error::last_os_error());
    }
    socket.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)).into())?;
    socket.listen(1024)?;
    Ok(socket.into())
}
