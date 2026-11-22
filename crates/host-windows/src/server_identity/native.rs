use super::{errors::IdentityError, types::ServerIdentity};
use crate::{
    shared::consts::SERVICE_NAME,
    shared::native::{LocalAllocation, service_sid_text, sid_text},
};
use std::os::windows::io::AsRawHandle;
use tokio::net::windows::named_pipe::NamedPipeClient;
use windows::Win32::{
    Foundation::HANDLE,
    Security::{
        Authorization::{GetSecurityInfo, SE_KERNEL_OBJECT},
        OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID,
    },
    System::Pipes::GetNamedPipeServerProcessId,
};
use windows_service::{
    service::{ServiceAccess, ServiceState, ServiceType},
    service_manager::{ServiceManager, ServiceManagerAccess},
};

pub fn verify_server(pipe: &NamedPipeClient) -> Result<ServerIdentity, IdentityError> {
    let pid = running_pid()?;
    let handle = HANDLE(pipe.as_raw_handle());
    check_pid(handle, pid)?;
    let service_sid = service_sid_text().map_err(|_| IdentityError::Native)?;
    check_owner(handle, &service_sid)?;
    Ok(ServerIdentity { pid })
}

pub(super) fn running_pid() -> Result<u32, IdentityError> {
    let manager = ServiceManager::local_computer(None::<&str>, ServiceManagerAccess::CONNECT)
        .map_err(|_| IdentityError::Native)?;
    let service = manager
        .open_service(SERVICE_NAME, ServiceAccess::QUERY_STATUS)
        .map_err(|_| IdentityError::ServiceNotRunning)?;
    let status = service.query_status().map_err(|_| IdentityError::Native)?;
    if status.service_type != ServiceType::OWN_PROCESS
        || status.current_state != ServiceState::Running
    {
        return Err(IdentityError::ServiceNotRunning);
    }
    status
        .process_id
        .filter(|pid| *pid != 0)
        .ok_or(IdentityError::ServiceNotRunning)
}

pub(super) fn check_pid(pipe: HANDLE, expected_pid: u32) -> Result<(), IdentityError> {
    let mut actual_pid = 0;
    // ! SAFETY: `pipe` is a pipe handle the caller keeps open; `actual_pid` is a valid out-pointer.
    unsafe { GetNamedPipeServerProcessId(pipe, &mut actual_pid) }
        .map_err(|_| IdentityError::Native)?;
    if actual_pid == expected_pid {
        Ok(())
    } else {
        Err(IdentityError::WrongProcess)
    }
}

pub(super) fn check_owner(pipe: HANDLE, expected_sid: &str) -> Result<(), IdentityError> {
    let mut owner = PSID::default();
    let mut descriptor = PSECURITY_DESCRIPTOR::default();
    // ! SAFETY: `pipe` is open for the call; the descriptor is freed by `allocation`.
    unsafe {
        GetSecurityInfo(
            pipe,
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION,
            Some(&mut owner),
            None,
            None,
            None,
            Some(&mut descriptor),
        )
    }
    .ok()
    .map_err(|_| IdentityError::Native)?;
    let allocation = LocalAllocation(descriptor.0);
    if owner.0.is_null() {
        return Err(IdentityError::Native);
    }
    let actual = sid_text(owner).map_err(|_| IdentityError::Native)?;
    drop(allocation);
    if actual == expected_sid {
        Ok(())
    } else {
        Err(IdentityError::WrongOwner)
    }
}
