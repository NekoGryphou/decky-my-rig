use super::super::{
    native::{
        authorize, create_pipe, group_members, managers_sid, recorded_managers, service_sid_text,
        sid_words, well_known,
    },
    types::{Allowed, ControlPipe, RecordedUser},
};
use super::utils::{control_pipe_sddl, manager_users};
use crate::control_client::control_pipe_name;
use std::sync::Arc;
use windows::Win32::Security::{WinBuiltinAdministratorsSid, WinLocalSystemSid};

impl ControlPipe {
    /// Creates the service's control pipe. SYSTEM, Administrators and the
    /// service own it, and members of My Rig Managers plus the users the
    /// installer recorded get client rights. Fails if the installer hasn't
    /// created the group.
    pub fn bind() -> Result<Self, &'static str> {
        let service = service_sid_text()?;
        let (managers_text, managers_words) = managers_sid()?;
        let members = group_members().unwrap_or_default();
        let users = manager_users(&recorded_managers(), &members);
        let clients: Vec<String> = std::iter::once(managers_text)
            .chain(users.iter().cloned())
            .collect();
        let sddl = control_pipe_sddl(&service, &clients);
        let always = vec![
            well_known(WinLocalSystemSid)?,
            well_known(WinBuiltinAdministratorsSid)?,
            managers_words,
        ];
        // * An unreadable entry only costs that user access, not the whole service.
        let recorded_users = users
            .iter()
            .filter_map(|user| {
                Some(RecordedUser {
                    text: user.clone(),
                    words: sid_words(user).ok()?,
                })
            })
            .collect();
        let name = control_pipe_name();
        Ok(Self {
            pipe: create_pipe(&name, &sddl, true)?,
            name,
            sddl,
            allowed: Arc::new(Allowed {
                always,
                recorded_users,
            }),
            authorize,
        })
    }
}
