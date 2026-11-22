#[derive(Debug)]
pub struct PowerError;

#[derive(Debug, PartialEq, Eq)]
pub enum ShutdownError {
    InProgress,
    Failed,
    Internal,
}
