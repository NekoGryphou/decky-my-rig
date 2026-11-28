use super::super::{
    consts::UNAUTHORIZED,
    native::{authorize, authorize_with, create_pipe, local_account_sid, well_known},
    types::{Allowed, Authorize, RecordedUser},
};
use super::fixtures::*;
use crate::control_client::connect_client;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{io, sync::Arc};
use tokio::sync::watch;
use windows::Win32::Security::{
    CheckTokenMembership, PSID, WinBuiltinAdministratorsSid, WinLocalSystemSid,
};
use windows::Win32::Security::{SidTypeAlias, SidTypeUser};

#[tokio::test]
async fn denied_pipe_connection_preserves_the_windows_access_error() {
    let name = format!(
        r"\\.\pipe\MyRigHost.Control.Denied.{}.{}",
        std::process::id(),
        NEXT_PIPE.fetch_add(1, Ordering::Relaxed)
    );
    let sddl = format!(
        "D:P(D;;0x0012019b;;;{})(A;;FA;;;SY)(A;;RC;;;OW)",
        current_sid()
    );
    let _server = create_pipe(&name, &sddl, true).unwrap();
    let error = connect_client(&name).expect_err("the fixture ACL denies client access");
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
    assert_eq!(error.raw_os_error(), Some(5));
}

#[tokio::test]
async fn client_only_ace_allows_notifier_connection() {
    let name = format!(
        r"\\.\pipe\MyRigHost.Control.NotifierTest.{}.{}",
        std::process::id(),
        NEXT_PIPE.fetch_add(1, Ordering::Relaxed)
    );
    let sddl = format!("D:P(A;;0x12019b;;;{})(A;;RC;;;OW)", current_sid());
    let _server = create_pipe(&name, &sddl, true).unwrap();
    let client = connect_client(&name).unwrap();
    drop(client);
}

#[tokio::test]
async fn actual_token_requires_administrator_or_system() {
    let admins = well_known(WinBuiltinAdministratorsSid).unwrap();
    let system = well_known(WinLocalSystemSid).unwrap();
    let member = |sid: &[u32]| {
        let mut result = false.into();
        // ! SAFETY: `sid` points to a complete SID buffer that outlives the call.
        unsafe { CheckTokenMembership(None, PSID(sid.as_ptr().cast_mut().cast()), &mut result) }
            .unwrap();
        result.as_bool()
    };
    let expected = member(&admins) || member(&system);
    let (name, pipe) = fixture(authorize);
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let (stop, receiver) = watch::channel(false);
    let server = tokio::spawn(pipe.serve(
        move |_, _context| {
            observed.fetch_add(1, Ordering::SeqCst);
            b"authorized".to_vec()
        },
        || (),
        receiver,
    ));
    let mut client = narrow_client(&name);
    assert_eq!(
        request(&mut client, b"administrator authorization").await,
        if expected {
            &b"authorized"[..]
        } else {
            UNAUTHORIZED
        }
    );
    drop(client);
    stop.send(true).unwrap();
    server.await.unwrap().unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), usize::from(expected));
}

#[tokio::test]
async fn pipe_refuses_unlisted_caller() {
    let (name, mut pipe) = fixture(authorize);
    let mut other = current_sid_words();
    *other.last_mut().unwrap() ^= 1;
    pipe.allowed = Arc::new(Allowed {
        always: vec![other],
        recorded_users: Vec::new(),
    });
    let (stop, receiver) = watch::channel(false);
    let server = tokio::spawn(pipe.serve(|_, _context| b"unexpected".to_vec(), || (), receiver));
    let mut client = narrow_client(&name);
    assert_eq!(request(&mut client, b"denied").await, UNAUTHORIZED);
    drop(client);
    stop.send(true).unwrap();
    server.await.unwrap().unwrap();
}

#[tokio::test]
async fn pipe_accepts_allowed_group() {
    let (name, mut pipe) = fixture(authorize);
    pipe.allowed = Arc::new(Allowed {
        always: vec![current_sid_words()],
        recorded_users: Vec::new(),
    });
    let (stop, receiver) = watch::channel(false);
    let server = tokio::spawn(pipe.serve(|_, _context| b"allowed".to_vec(), || (), receiver));
    let mut client = narrow_client(&name);
    assert_eq!(request(&mut client, b"group member").await, b"allowed");
    drop(client);
    stop.send(true).unwrap();
    server.await.unwrap().unwrap();
}

#[test]
fn local_account_lookup_resolves_this_machine_accounts() {
    let user = std::env::var("USERNAME").unwrap();
    let (text, _) = local_account_sid(&user, SidTypeUser).unwrap();
    assert_eq!(text, current_sid());
}

#[test]
fn missing_local_group_is_an_error() {
    let missing = format!("My Rig Missing Group {}", std::process::id());
    assert!(local_account_sid(&missing, SidTypeAlias).is_err());
}

/// Serves one request with the current user recorded as a manager, and the
/// managers group reported as `members` at the time of the connection.
async fn recorded_user_request(authorize: Authorize) -> Vec<u8> {
    let (name, mut pipe) = fixture(authorize);
    pipe.allowed = Arc::new(Allowed {
        always: Vec::new(),
        recorded_users: vec![RecordedUser {
            text: current_sid(),
            words: current_sid_words(),
        }],
    });
    let (stop, receiver) = watch::channel(false);
    let server = tokio::spawn(pipe.serve(|_, _context| b"allowed".to_vec(), || (), receiver));
    let mut client = narrow_client(&name);
    let reply = request(&mut client, b"recorded user").await;
    drop(client);
    stop.send(true).unwrap();
    server.await.unwrap().unwrap();
    reply
}

#[tokio::test]
async fn a_recorded_user_is_refused_once_removed_from_the_group() {
    let removed: Authorize = |pipe, allowed| authorize_with(pipe, allowed, || Ok(Vec::new()));
    assert_eq!(recorded_user_request(removed).await, UNAUTHORIZED);
    let unreadable: Authorize = |pipe, allowed| authorize_with(pipe, allowed, || Err("unreadable"));
    assert_eq!(recorded_user_request(unreadable).await, UNAUTHORIZED);
    let member: Authorize =
        |pipe, allowed| authorize_with(pipe, allowed, || Ok(vec![current_sid()]));
    assert_eq!(recorded_user_request(member).await, b"allowed");
}
