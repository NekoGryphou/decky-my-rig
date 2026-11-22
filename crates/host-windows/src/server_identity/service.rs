use super::{
    errors::IdentityError,
    native::{check_pid, running_pid, verify_server},
    types::ServerIdentity,
};
use crate::control_client::connect_client;
use std::{io, os::windows::io::AsRawHandle};
use tokio::net::windows::named_pipe::NamedPipeClient;
use windows::Win32::Foundation::HANDLE;

impl ServerIdentity {
    pub fn recheck(&self, pipe: &NamedPipeClient) -> Result<(), IdentityError> {
        if running_pid()? != self.pid {
            return Err(IdentityError::WrongProcess);
        }
        check_pid(HANDLE(pipe.as_raw_handle()), self.pid)
    }
}

/// Connects to the control pipe `name` and checks that its server is the
/// running My Rig service before sending anything. A server that isn't gives an
/// `Other` error, never `PermissionDenied`, which means the caller was refused.
pub fn connect_verified(name: &str) -> io::Result<(NamedPipeClient, ServerIdentity)> {
    let pipe = connect_client(name)?;
    let identity = verify_server(&pipe).map_err(|_| io::Error::other("not the My Rig service"))?;
    Ok((pipe, identity))
}
