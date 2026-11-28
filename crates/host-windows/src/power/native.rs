// ! Win32 calls behind a shutdown request. They stay apart from the `Power` impl so another
// ! platform's backend can sit beside them.
use windows::{
    Win32::{
        Foundation::{CloseHandle, ERROR_NOT_ALL_ASSIGNED, GetLastError, HANDLE},
        Security::{
            AdjustTokenPrivileges, LUID_AND_ATTRIBUTES, LookupPrivilegeValueW,
            SE_PRIVILEGE_ENABLED, SE_SHUTDOWN_NAME, TOKEN_ADJUST_PRIVILEGES, TOKEN_PRIVILEGES,
            TOKEN_QUERY,
        },
        System::{
            Shutdown::{AbortSystemShutdownW, InitiateSystemShutdownW},
            Threading::{GetCurrentProcess, OpenProcessToken},
        },
    },
    core::{HSTRING, PCWSTR, Result},
};

/// Asks Windows for a graceful shutdown after `grace_secs`, with `message`
/// shown to the user.
pub(super) fn initiate_shutdown(message: &str, grace_secs: u32) -> Result<()> {
    enable_shutdown_privilege()?;
    let message = HSTRING::from(message);
    // ! SAFETY: a null machine name means this machine; the message is a valid wide string that
    // ! lives until the call returns.
    unsafe { InitiateSystemShutdownW(PCWSTR::null(), &message, grace_secs, false, false) }
}

/// Withdraws a shutdown scheduled by `initiate_shutdown`. Windows reports an
/// error when nothing is scheduled, which the caller treats as already done.
pub(super) fn abort_shutdown() -> Result<()> {
    enable_shutdown_privilege()?;
    // ! SAFETY: a null machine name means this machine; the call takes no buffers.
    unsafe { AbortSystemShutdownW(PCWSTR::null()) }
}

/// `InitiateSystemShutdownW` fails without SeShutdownPrivilege, which the
/// process token has but leaves disabled.
fn enable_shutdown_privilege() -> Result<()> {
    let mut token = HANDLE::default();
    // ! SAFETY: the current process pseudo-handle is always valid; `token` is written on success
    // ! and closed below.
    unsafe {
        OpenProcessToken(
            GetCurrentProcess(),
            TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY,
            &mut token,
        )?;
    }
    let adjusted = adjust_shutdown_privilege(token);
    // ! SAFETY: `token` came from a successful OpenProcessToken and is closed once.
    let closed = unsafe { CloseHandle(token) };
    adjusted.and(closed)
}

fn adjust_shutdown_privilege(token: HANDLE) -> Result<()> {
    let mut privileges = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES::default()],
    };
    // ! SAFETY: `privileges` is a single-entry TOKEN_PRIVILEGES, the layout the API expects, and
    // ! `token` is a live handle opened with ADJUST_PRIVILEGES.
    unsafe {
        LookupPrivilegeValueW(
            PCWSTR::null(),
            SE_SHUTDOWN_NAME,
            &mut privileges.Privileges[0].Luid,
        )?;
        privileges.Privileges[0].Attributes = SE_PRIVILEGE_ENABLED;
        AdjustTokenPrivileges(token, false, Some(&privileges as *const _), 0, None, None)?;
        // ! Win32 reports success even when the privilege was not assigned; only the last error
        // ! says so.
        if GetLastError() == ERROR_NOT_ALL_ASSIGNED {
            return Err(ERROR_NOT_ALL_ASSIGNED.to_hresult().into());
        }
    }
    Ok(())
}
