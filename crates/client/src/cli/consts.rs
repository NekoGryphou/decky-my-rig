pub(super) const OK: u8 = 0;
pub(super) const REFUSED: u8 = 1;
pub(super) const USAGE: u8 = 2;
pub(super) const UNREACHABLE: u8 = 3;
pub(super) const DEFAULT_PORT: &str = "47991";
pub(super) const STATUS_DEADLINE: std::time::Duration = std::time::Duration::from_secs(3);
