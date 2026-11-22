pub struct InstallDirectory;

#[derive(Clone, Copy)]
pub(super) enum PathRole {
    Ancestor,
    CreationParent,
    Destination,
    File,
}

pub(super) struct AccessRule {
    pub principal: String,
    pub rights: u32,
    pub inherit_only: bool,
    pub inheritable: bool,
    pub allow: bool,
}

pub(super) struct Security {
    pub owner: String,
    pub rules: Vec<AccessRule>,
}
