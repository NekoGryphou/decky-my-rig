use std::fmt;

/// An error with a message written for the user and safe to show. It never
/// contains input values or secrets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PublicError(pub &'static str);

impl fmt::Display for PublicError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

impl std::error::Error for PublicError {}
