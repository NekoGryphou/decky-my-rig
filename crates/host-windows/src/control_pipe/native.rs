//! Windows pipe security and caller identity checks.
use super::consts::{
    MANAGER_USERS_VALUE, MANAGERS_GROUP, MANAGERS_KEY, MAX_ACTIVE, REVERT_FAILURE,
};
use crate::shared::consts::FAILURE;
use crate::shared::native::{LocalAllocation, account_sid, wide, with_security};
pub(super) use crate::shared::native::{service_sid_text, sid_text};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
use windows::Win32::NetworkManagement::NetManagement::{
    LOCALGROUP_MEMBERS_INFO_0, MAX_PREFERRED_LENGTH, NERR_Success, NetApiBufferFree,
    NetLocalGroupGetMembers,
};
use windows::{
    Win32::{
        Foundation::{DUPLICATE_SAME_ACCESS, DuplicateHandle, HANDLE},
        Security::{
            Authorization::ConvertStringSidToSidW, CheckTokenMembership, CreateWellKnownSid,
            GetLengthSid, PSID, RevertToSelf, SID_NAME_USE, SidTypeAlias, TOKEN_QUERY,
            WELL_KNOWN_SID_TYPE,
        },
        System::{
            Pipes::ImpersonateNamedPipeClient,
            Threading::{GetCurrentProcess, GetCurrentThread, OpenThreadToken},
        },
    },
    core::PCWSTR,
};
pub(super) fn managers_sid() -> Result<(String, Vec<u32>), &'static str> {
    local_account_sid(MANAGERS_GROUP, SidTypeAlias)
}

/// User SIDs the installer recorded as managers, or none. Only administrators
/// can write the key.
pub(super) fn recorded_managers() -> Vec<String> {
    windows_registry::LOCAL_MACHINE
        .open(MANAGERS_KEY)
        .and_then(|key| key.get_multi_string(MANAGER_USERS_VALUE))
        .unwrap_or_default()
}

/// SIDs of the current members of My Rig Managers, read from the account
/// database rather than a token, so membership changes count right away.
pub(super) fn group_members() -> Result<Vec<String>, &'static str> {
    let group = wide(MANAGERS_GROUP);
    let mut buffer: *mut u8 = std::ptr::null_mut();
    let (mut read, mut total) = (0u32, 0u32);
    // ! SAFETY: `group` is NUL-terminated and live; `buffer` receives a block
    //   that NetApiBufferFree releases below; the counts are valid out-pointers.
    let status = unsafe {
        NetLocalGroupGetMembers(
            PCWSTR::null(),
            PCWSTR(group.as_ptr()),
            0,
            &mut buffer,
            MAX_PREFERRED_LENGTH,
            &mut read,
            &mut total,
            None,
        )
    };
    if status != NERR_Success {
        return Err(FAILURE);
    }
    let members = if buffer.is_null() {
        Vec::new()
    } else {
        // ! SAFETY: the call succeeded, so `buffer` holds `read` level-0 entries.
        let entries = unsafe {
            std::slice::from_raw_parts(buffer.cast::<LOCALGROUP_MEMBERS_INFO_0>(), read as usize)
        };
        entries
            .iter()
            .filter_map(|entry| sid_text(entry.lgrmi0_sid).ok())
            .collect()
    };
    if !buffer.is_null() {
        // ! SAFETY: `buffer` came from NetLocalGroupGetMembers and is freed once.
        unsafe { NetApiBufferFree(Some(buffer.cast())) };
    }
    Ok(members)
}

/// The SID string as words, the form the caller check compares.
pub(super) fn sid_words(text: &str) -> Result<Vec<u32>, &'static str> {
    let text = wide(text);
    let mut sid = PSID::default();
    // ! SAFETY: `text` is NUL-terminated and live; the SID is freed by `allocation`.
    unsafe { ConvertStringSidToSidW(PCWSTR(text.as_ptr()), &mut sid) }.map_err(|_| FAILURE)?;
    let allocation = LocalAllocation(sid.0);
    // ! SAFETY: ConvertStringSidToSidW succeeded, so `sid` is a valid SID.
    let length = unsafe { GetLengthSid(sid) } as usize;
    let mut words = vec![0u32; length.div_ceil(4)];
    // ! SAFETY: `words` holds at least `length` bytes and does not overlap the SID.
    unsafe { std::ptr::copy_nonoverlapping(sid.0.cast::<u8>(), words.as_mut_ptr().cast(), length) };
    drop(allocation);
    Ok(words)
}

/// Looks up an account in this computer's own account database. The installer
/// creates the managers group, so a missing account is an error.
pub(super) fn local_account_sid(
    name: &str,
    expected: SID_NAME_USE,
) -> Result<(String, Vec<u32>), &'static str> {
    let computer = crate::shared::native::computer_name()?;
    // ? LookupAccountNameW doesn't understand the ".\" shorthand, and using the
    //   computer name keeps a domain account with the same name from being
    //   picked.
    let (mut sid, kind) = account_sid(&format!(r"{computer}\{name}"))?;
    if kind != expected {
        return Err(FAILURE);
    }
    Ok((sid_text(PSID(sid.as_mut_ptr().cast()))?, sid))
}

pub(super) fn create_pipe(
    name: &str,
    sddl: &str,
    first: bool,
) -> Result<NamedPipeServer, &'static str> {
    with_security(sddl, |attributes| {
        // ! SAFETY: `attributes` and its descriptor stay alive for the call, which copies the security.
        unsafe {
            ServerOptions::new()
                .first_pipe_instance(first)
                .max_instances(MAX_ACTIVE + 1)
                .reject_remote_clients(true)
                .create_with_security_attributes_raw(name, attributes.cast())
        }
        .map_err(|_| FAILURE)
    })?
}

pub(super) fn well_known(kind: WELL_KNOWN_SID_TYPE) -> Result<Vec<u32>, &'static str> {
    let mut sid = vec![0u32; 32];
    let mut length = (sid.len() * 4) as u32;
    // ! SAFETY: `sid` is a 128-byte buffer, larger than SECURITY_MAX_SID_SIZE, and `length` states its size.
    unsafe { CreateWellKnownSid(kind, None, Some(PSID(sid.as_mut_ptr().cast())), &mut length) }
        .map_err(|_| FAILURE)?;
    Ok(sid)
}

pub(super) fn authorize(pipe: &OwnedHandle, allowed: &[Vec<u32>]) -> Result<bool, &'static str> {
    // ! Clients connect at Identification level, so the caller's token can only be
    //   opened as the service itself (OpenAsSelf). Impersonation lasts just long
    //   enough to take that token; membership is checked after reverting.
    // ! SAFETY: `pipe` is an open pipe handle owned by the caller for the whole call.
    if unsafe { ImpersonateNamedPipeClient(HANDLE(pipe.as_raw_handle())) }.is_err() {
        return Ok(false);
    }
    let mut token = HANDLE::default();
    // ! SAFETY: `token` is a valid out-pointer; the current thread handle is a pseudo-handle.
    let opened = unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, true, &mut token) };
    // ! SAFETY: reverts this thread's own impersonation; it has no memory preconditions.
    let reverted = unsafe { RevertToSelf() };
    let token = opened
        .is_ok()
        // ! SAFETY: OpenThreadToken succeeded, so `token` is a new handle that nothing else owns.
        .then(|| unsafe { OwnedHandle::from_raw_handle(token.0) });
    if reverted.is_err() {
        return Err(REVERT_FAILURE);
    }
    let Some(token) = token else {
        return Ok(false);
    };
    let mut authorized = false;
    for sid in allowed {
        let mut member = false.into();
        // ! SAFETY: `token` is an open token handle and `sid` points to a complete SID buffer.
        let result = unsafe {
            CheckTokenMembership(
                Some(HANDLE(token.as_raw_handle())),
                PSID(sid.as_ptr().cast_mut().cast()),
                &mut member,
            )
        };
        authorized |= result.is_ok() && member.as_bool();
    }
    Ok(authorized)
}

pub(super) fn duplicate_pipe(pipe: &NamedPipeServer) -> Result<OwnedHandle, &'static str> {
    let mut duplicate = HANDLE::default();
    // ! SAFETY: both process handles are the current-process pseudo-handle and `pipe` is open for the call.
    unsafe {
        DuplicateHandle(
            GetCurrentProcess(),
            HANDLE(pipe.as_raw_handle()),
            GetCurrentProcess(),
            &mut duplicate,
            0,
            false,
            DUPLICATE_SAME_ACCESS,
        )
    }
    .map_err(|_| FAILURE)?;
    // ! SAFETY: DuplicateHandle succeeded, so `duplicate` is a new handle that nothing else owns.
    Ok(unsafe { OwnedHandle::from_raw_handle(duplicate.0) })
}
