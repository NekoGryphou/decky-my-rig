//! Sets up the protected files on a first install.
use myrig_host_core::provisioning::generate;
use socket2::{Domain, Protocol, Socket, Type};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    net::{Ipv4Addr, SocketAddr},
    path::Path,
};

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), &'static str> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| "Cannot create protected host file.")?;
    let result = file
        .write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| "Cannot write protected host file.");
    if result.is_err() {
        drop(file);
        let _ = fs::remove_file(path);
    }
    result
}

/// Creates the host's config and identity on a first install. Existing files
/// are refused: upgrades keep theirs and never run this.
pub fn initialize(config_path: &Path, security_path: &Path, port: u16) -> Result<(), &'static str> {
    if config_path.exists() || security_path.exists() {
        return Err("Host files already exist; refusing to replace the identity.");
    }
    let provisioned = generate(first_free_port(port)?)?;
    write_new(config_path, &provisioned.config)?;
    if let Err(error) = write_new(security_path, &provisioned.security) {
        let _ = fs::remove_file(config_path);
        return Err(error);
    }
    Ok(())
}

/// The first port from `start` that nothing else listens on, so a new install
/// never starts on a port it can't bind.
pub fn first_free_port(start: u16) -> Result<u16, &'static str> {
    (start..=u16::MAX)
        .find(|&port| probe_port(port).is_ok())
        .ok_or("No free port for the host.")
}

/// Binds `port` on every interface like the service will, without listening. A
/// setup helper that listens makes Windows Firewall prompt for it and keep an
/// inbound rule.
pub fn probe_port(port: u16) -> std::io::Result<Socket> {
    let socket = Socket::new(Domain::IPV4, Type::STREAM, Some(Protocol::TCP))?;
    socket.bind(&SocketAddr::from((Ipv4Addr::UNSPECIFIED, port)).into())?;
    Ok(socket)
}
