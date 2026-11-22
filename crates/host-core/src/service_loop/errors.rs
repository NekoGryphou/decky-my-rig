#[derive(Debug, PartialEq, Eq)]
pub enum ServiceLoopError {
    Http,
    Control,
    UnexpectedExit,
    Status,
}
