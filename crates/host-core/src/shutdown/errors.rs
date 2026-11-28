/// `code` is the platform's error value (an HRESULT on Windows); it is for the
/// host's own log and never goes into an HTTP reply.
#[derive(Debug, PartialEq, Eq)]
pub struct PowerError {
    pub code: u32,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ShutdownError {
    InProgress,
    Failed(PowerError),
    Internal,
}
