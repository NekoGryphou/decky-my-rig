#[derive(Debug, PartialEq, Eq)]
pub enum StorageError {
    /// The file no longer holds the bytes the caller last read.
    Conflict,
    Io,
}
