use crate::control_client::connect_client;
use crate::server_identity::{
    IdentityError,
    native::{check_owner, check_pid},
};
use std::{
    mem::size_of_val,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::atomic::{AtomicU64, Ordering},
};
use tokio::net::windows::named_pipe::{NamedPipeClient, NamedPipeServer, ServerOptions};
use windows::Win32::{
    Foundation::HANDLE,
    Security::{GetTokenInformation, TOKEN_OWNER, TOKEN_QUERY, TokenOwner},
    System::Threading::{GetCurrentProcess, OpenProcessToken},
};

static NEXT_PIPE: AtomicU64 = AtomicU64::new(0);

fn owned_pipe() -> (NamedPipeServer, NamedPipeClient, String) {
    let name = format!(
        r"\\.\pipe\MyRigHost.Identity.Test.{}.{}",
        std::process::id(),
        NEXT_PIPE.fetch_add(1, Ordering::Relaxed)
    );
    let server = ServerOptions::new()
        .first_pipe_instance(true)
        .reject_remote_clients(true)
        .create(&name)
        .unwrap();
    let client = connect_client(&name).unwrap();
    let mut token = HANDLE::default();
    // ! SAFETY: the process handle is a pseudo-handle and `token` is a valid out-pointer.
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }.unwrap();
    // ! SAFETY: OpenProcessToken succeeded, so `token` is a new handle that nothing else owns.
    let token = unsafe { OwnedHandle::from_raw_handle(token.0) };
    let mut buffer = [0u64; 128];
    let mut needed = 0;
    // ! SAFETY: `buffer` is live for the call and its size is stated.
    unsafe {
        GetTokenInformation(
            HANDLE(token.as_raw_handle()),
            TokenOwner,
            Some(buffer.as_mut_ptr().cast()),
            size_of_val(&buffer) as u32,
            &mut needed,
        )
    }
    .unwrap();
    // ! SAFETY: GetTokenInformation filled `buffer`, 8-byte aligned, with a TOKEN_OWNER.
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_OWNER>() };
    let sid = crate::shared::native::sid_text(user.Owner).unwrap();
    (server, client, sid)
}

#[tokio::test]
async fn pipe_owned_by_token_owner_passes_owner_check_for_that_user() {
    let (_server, client, owner) = owned_pipe();
    assert_eq!(check_owner(HANDLE(client.as_raw_handle()), &owner), Ok(()));
}

#[tokio::test]
async fn pipe_owned_by_token_owner_fails_for_service_sid() {
    let (_server, client, _) = owned_pipe();
    // - From `sc.exe showsid MyRigHost`; the service doesn't need to be installed
    //   for this.
    let service_sid = "S-1-5-80-1141499660-3876455147-362637617-4292076426-725672618";
    assert_eq!(
        check_owner(HANDLE(client.as_raw_handle()), service_sid),
        Err(IdentityError::WrongOwner)
    );
}

#[tokio::test]
async fn wrong_server_pid_is_refused() {
    let (_server, client, _) = owned_pipe();
    let pipe = HANDLE(client.as_raw_handle());
    assert_eq!(
        check_pid(pipe, std::process::id() + 1),
        Err(IdentityError::WrongProcess)
    );
    assert_eq!(check_pid(pipe, std::process::id()), Ok(()));
}

#[tokio::test]
async fn a_pipe_served_by_another_process_is_refused_before_any_request() {
    // - Someone squatting on the control pipe name while the service is stopped.
    let name = format!(
        r"\\.\pipe\MyRigHost.Identity.Test.{}.{}",
        std::process::id(),
        NEXT_PIPE.fetch_add(1, Ordering::Relaxed)
    );
    let _squatter = ServerOptions::new()
        .first_pipe_instance(true)
        .reject_remote_clients(true)
        .create(&name)
        .unwrap();
    let Err(refused) = crate::server_identity::connect_verified(&name) else {
        panic!("a squatter passed the server check");
    };
    // * Not an authorization refusal, which the notifier would show to the user.
    assert_ne!(refused.kind(), std::io::ErrorKind::PermissionDenied);
    assert_ne!(refused.kind(), std::io::ErrorKind::NotFound);
}
