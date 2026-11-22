use std::fs::File;

/// An open settings directory. Every file operation is relative to it.
pub(crate) struct LinuxSettings {
    pub(super) directory: File,
}
