use super::super::consts::MAX_RECORDED_MANAGERS;
use crate::control_client::consts::CONTROL_CLIENT_ACCESS;

/// SYSTEM, Administrators and the service own the pipe, and each manager (the
/// group first, then each recorded user) gets client rights.
pub(super) fn control_pipe_sddl(service: &str, managers: &[String]) -> String {
    let clients: String = managers
        .iter()
        .map(|sid| format!("(A;;0x{CONTROL_CLIENT_ACCESS:08x};;;{sid})"))
        .collect();
    format!("O:{service}D:P(A;;FA;;;SY)(A;;FA;;;BA)(A;;FA;;;{service}){clients}(A;;RC;;;OW)")
}

/// The users the installer recorded as managers who are still in My Rig
/// Managers: local or domain accounts (`S-1-5-21-…`) and Entra ID accounts
/// (`S-1-12-1-…`) with four 32-bit sub-authorities, without duplicates, in
/// order. Their own SID is in their sign-in token, unlike a group they were
/// just added to, so they don't need to sign out. Taking them out of the group
/// revokes them the next time the service starts.
pub(super) fn manager_users(recorded: &[String], members: &[String]) -> Vec<String> {
    let mut users: Vec<String> = Vec::new();
    for sid in recorded {
        let parts: Vec<&str> = sid.split('-').collect();
        let account = parts.len() == 8
            && (parts[..4] == ["S", "1", "5", "21"] || parts[..4] == ["S", "1", "12", "1"])
            && parts[4..].iter().all(|part| {
                part.bytes().all(|b| b.is_ascii_digit()) && part.parse::<u32>().is_ok()
            });
        if account
            && members.contains(sid)
            && !users.contains(sid)
            && users.len() < MAX_RECORDED_MANAGERS
        {
            users.push(sid.clone());
        }
    }
    users
}
