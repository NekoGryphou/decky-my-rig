pub(super) const FILE_NAME: &str = "pcs.json";
pub(super) const LOCK_NAME: &str = ".pcs.lock";
/// `FILE_FLAG_OPEN_REPARSE_POINT`: opens the link itself, never what it points
/// to.
pub(super) const OPEN_REPARSE_POINT: u32 = 0x0020_0000;
/// `FILE_FLAG_BACKUP_SEMANTICS`: needed to open a directory handle.
pub(super) const BACKUP_SEMANTICS: u32 = 0x0200_0000;
/// Read access plus the rights to change a file's owner and ACL.
pub(super) const READ_AND_SECURE: u32 = 0x8000_0000 | 0x0002_0000 | 0x0004_0000 | 0x0008_0000;
/// Read access plus the right to read a file's ACL, and nothing that changes it.
pub(super) const READ_ONLY: u32 = 0x8000_0000 | 0x0002_0000;
/// `GENERIC_WRITE` plus the rights to change a file's owner and ACL.
pub(super) const WRITE_AND_SECURE: u32 = 0x4000_0000 | 0x0002_0000 | 0x0004_0000 | 0x0008_0000;
/// Read and write sharing without delete, so the folder can't be renamed or
/// removed while it's open.
pub(super) const SHARE_NO_DELETE: u32 = 0x1 | 0x2;
/// `FILE_ATTRIBUTE_REPARSE_POINT`: a symlink, junction or other redirect.
pub(super) const REPARSE_POINT: u32 = 0x400;
