pub(super) const HOSTNAME_PATH: &str = "/proc/sys/kernel/hostname";
/// Far above any hostname; a file past it is not one.
pub(super) const MAX_HOSTNAME_BYTES: usize = 1024;
