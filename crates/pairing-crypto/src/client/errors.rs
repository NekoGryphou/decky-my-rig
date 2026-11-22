/// Why an exchange couldn't continue. Carries no protocol bytes or secrets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientError {
    /// Wrong code shape, certificate size or message length.
    InvalidInput,
    /// The host's response doesn't prove it knows the same code, for the same
    /// certificate and session.
    AuthenticationFailed,
    /// The OPAQUE library produced a message we didn't expect.
    Internal,
}
