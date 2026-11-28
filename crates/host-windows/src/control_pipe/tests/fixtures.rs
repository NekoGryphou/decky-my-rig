use super::super::{
    native::{create_pipe, sid_text, well_known},
    types::{Allowed, Authorize, ControlPipe},
};
use crate::control_client::connect_client;
use myrig_protocol::local_control::RESPONSE_LIMIT;
use std::sync::atomic::{AtomicU64, Ordering};
use std::{
    mem::size_of_val,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    sync::Arc,
    time::Duration,
};
use tokio::time::timeout;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::NamedPipeClient,
};
use windows::Win32::{
    Foundation::HANDLE,
    Security::{WinBuiltinAdministratorsSid, WinLocalSystemSid},
    System::Threading::GetCurrentProcess,
};
use windows::Win32::{
    Security::{GetLengthSid, GetTokenInformation, TOKEN_QUERY, TOKEN_USER, TokenUser},
    System::Threading::OpenProcessToken,
};

pub(super) static NEXT_PIPE: AtomicU64 = AtomicU64::new(0);

/// This process's TOKEN_USER in an 8-byte aligned buffer. The SID it points to
/// sits inside the buffer, so the buffer lives on the heap and doesn't move
/// when it's returned.
fn token_user() -> Vec<u64> {
    let mut handle = HANDLE::default();
    // ! SAFETY: the process handle is a pseudo-handle and `handle` is a valid out-pointer.
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut handle) }.unwrap();
    // ! SAFETY: OpenProcessToken succeeded, so `handle` is a new handle that nothing else owns.
    let token = unsafe { OwnedHandle::from_raw_handle(handle.0) };
    let mut buffer = vec![0u64; 128];
    let mut needed = 0;
    // ! SAFETY: `buffer` is live for the call and its size is stated.
    unsafe {
        GetTokenInformation(
            HANDLE(token.as_raw_handle()),
            TokenUser,
            Some(buffer.as_mut_ptr().cast()),
            size_of_val(buffer.as_slice()) as u32,
            &mut needed,
        )
    }
    .unwrap();
    buffer
}

pub(super) fn current_sid() -> String {
    let buffer = token_user();
    // ! SAFETY: `token_user` filled `buffer`, 8-byte aligned, with a TOKEN_USER.
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    sid_text(user.User.Sid).unwrap()
}

pub(super) fn current_sid_words() -> Vec<u32> {
    let buffer = token_user();
    // ! SAFETY: `token_user` filled `buffer`, 8-byte aligned, with a TOKEN_USER.
    let sid = unsafe { (*buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    // ! SAFETY: `sid` points into `buffer`, which holds a valid SID.
    let len = unsafe { GetLengthSid(sid) } as usize;
    let mut words = vec![0u32; len.div_ceil(4)];
    // ! SAFETY: `words` holds at least `len` bytes and does not overlap `buffer`.
    unsafe {
        std::ptr::copy_nonoverlapping(sid.0.cast::<u8>(), words.as_mut_ptr().cast(), len);
    }
    words
}

pub(super) fn fixture(authorize: Authorize) -> (String, ControlPipe) {
    let name = format!(
        r"\\.\pipe\MyRigHost.Control.Test.{}.{}",
        std::process::id(),
        NEXT_PIPE.fetch_add(1, Ordering::Relaxed)
    );
    let sddl = format!("D:P(A;;FA;;;{})(A;;RC;;;OW)", current_sid());
    let pipe = ControlPipe {
        pipe: create_pipe(&name, &sddl, true).unwrap(),
        name: name.clone(),
        sddl,
        allowed: Arc::new(Allowed {
            always: vec![
                well_known(WinLocalSystemSid).unwrap(),
                well_known(WinBuiltinAdministratorsSid).unwrap(),
            ],
            recorded_users: Vec::new(),
        }),
        authorize,
    };
    (name, pipe)
}

pub(super) fn narrow_client(name: &str) -> NamedPipeClient {
    connect_client(name).unwrap()
}

pub(super) async fn request(client: &mut NamedPipeClient, bytes: &[u8]) -> Vec<u8> {
    client
        .write_all(&(bytes.len() as u32).to_le_bytes())
        .await
        .unwrap();
    // - Send the input in pieces rather than as one write per frame.
    for byte in bytes {
        client.write_all(&[*byte]).await.unwrap();
    }
    let mut prefix = [0u8; 4];
    timeout(Duration::from_secs(3), client.read_exact(&mut prefix))
        .await
        .unwrap()
        .unwrap();
    let length = u32::from_le_bytes(prefix) as usize;
    assert!(length <= RESPONSE_LIMIT);
    let mut response = vec![0; length];
    timeout(Duration::from_secs(3), client.read_exact(&mut response))
        .await
        .unwrap()
        .unwrap();
    response
}
