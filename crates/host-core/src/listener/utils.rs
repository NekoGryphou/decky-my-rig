use super::types::Bind;
use std::io;
use tokio::net::TcpListener;

pub(super) fn listen(bind: Bind, port: u16) -> io::Result<TcpListener> {
    let listener = bind(port)?;
    listener.set_nonblocking(true)?;
    TcpListener::from_std(listener)
}
