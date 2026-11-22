pub(super) const FAILURE: &str =
    "Choose a local installation folder that only administrators can modify.";
pub(super) const TRUSTED_OWNERS: &[&str] = &[
    "S-1-5-18",
    "S-1-5-32-544",
    "S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464",
];
pub(super) const CREATOR_OWNER: &str = "S-1-3-0";
// - Write, append, EA and attribute writes, delete, delete-child, DACL and owner
//   changes, and generic write.
pub(super) const WRITE_ACCESS: u32 = 0x500d_0156;
// ? Being able to add a sibling folder isn't enough to replace a child that's
//   already protected.
pub(super) const ANCESTOR_ACCESS: u32 = WRITE_ACCESS & !0x4;
pub(super) const RESERVED_NAMES: &[&str] = &["CON", "PRN", "AUX", "NUL"];
pub(super) const VOLUME_GUID_UNITS: usize = 50;
pub(super) const VOLUME_PATH_UNITS: usize = 32768;
