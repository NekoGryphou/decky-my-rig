use windows::Win32::System::{
    RemoteDesktop::{ProcessIdToSessionId, WTSGetActiveConsoleSessionId},
    Threading::GetCurrentProcessId,
};

/// True when this process runs in the session shown on the PC's own screen,
/// not a remote or switched-away one.
pub fn is_console_session() -> bool {
    let mut session = 0;
    // ! SAFETY: `session` is a valid out-pointer for the current process id.
    unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session) }.is_ok()
        // ! SAFETY: WTSGetActiveConsoleSessionId has no preconditions.
        && session == unsafe { WTSGetActiveConsoleSessionId() }
}
