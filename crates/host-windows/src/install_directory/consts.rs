pub(super) const FAILURE: &str =
    "Choose a local installation folder that only administrators can modify.";
pub(super) const DATA_FAILURE: &str =
    "The My Rig data folder is not protected for administrators only, so it was not adopted.";
pub(super) const TRUSTED_OWNERS: &[&str] = &[
    "S-1-5-18",
    "S-1-5-32-544",
    // NT SERVICE\TrustedInstaller, which owns the Windows-managed program folders.
    "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464",
    // NT SERVICE\MyRigHost, which owns the data folders the service writes to.
    "S-1-5-80-1141499660-3876455147-362637617-4292076426-725672618",
];
pub(super) const CREATOR_OWNER: &str = "S-1-3-0";
// LocalService is the default owner of files the service writes. Its owner
// can change the ACL only if no OWNER RIGHTS entry limits it, so that entry is
// what makes this owner trusted.
pub(super) const LOCAL_SERVICE: &str = "S-1-5-19";
pub(super) const OWNER_RIGHTS: &str = "S-1-3-4";
// - Write, append, EA and attribute writes, delete, delete-child, DACL and owner
//   changes, and generic write.
pub(super) const WRITE_ACCESS: u32 = 0x500d_0156;
// ? Being able to add a sibling folder isn't enough to replace a child that's already protected.
pub(super) const ANCESTOR_ACCESS: u32 = WRITE_ACCESS & !0x4;
pub(super) const RESERVED_NAMES: &[&str] = &["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"];
pub(super) const VOLUME_GUID_UNITS: usize = 50;
pub(super) const VOLUME_PATH_UNITS: usize = 32768;
