use std::{future::Future, io, pin::Pin};

/// Resolves when the listener should stop: on shutdown, or when a new port is
/// saved.
pub(super) type ListenerStop = Pin<Box<dyn Future<Output = ()> + Send>>;

/// Opens the LAN listener's socket on every interface for a port. A platform
/// can set options a plain bind doesn't have.
pub type Bind = fn(u16) -> io::Result<std::net::TcpListener>;
