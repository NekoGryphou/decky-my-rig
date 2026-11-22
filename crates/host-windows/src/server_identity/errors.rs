#[derive(Debug, PartialEq, Eq)]
pub enum IdentityError {
    ServiceNotRunning,
    WrongProcess,
    WrongOwner,
    Native,
}
