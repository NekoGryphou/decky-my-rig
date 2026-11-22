#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Status,
    List,
    Pair(String),
    SetPort(u16),
    SetTestMode(bool),
    Remove(String),
    /// Device ID, new name.
    Rename(String, String),
}
