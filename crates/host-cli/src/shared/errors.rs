use super::messages::{UNAUTHORIZED, UNAVAILABLE, host_error};
use myrig_protocol::local_control::client::ReplyError;

/// Why a command failed. Each one maps to one exit code and one message.
#[derive(Debug, PartialEq, Eq)]
pub enum CliError {
    /// The service couldn't be reached, verified or understood.
    Unavailable,
    Unauthorized,
    /// The device isn't paired with this host.
    NotPaired,
    /// The service refused with this error code.
    Host(&'static str),
}

impl CliError {
    pub fn exit_code(&self) -> u8 {
        match self {
            Self::Unavailable => 3,
            Self::Unauthorized => 4,
            Self::NotPaired | Self::Host(_) => 1,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Unavailable => UNAVAILABLE.into(),
            Self::Unauthorized => UNAUTHORIZED.into(),
            Self::NotPaired => host_error("not_paired"),
            Self::Host(code) => host_error(code),
        }
    }
}

impl From<ReplyError> for CliError {
    fn from(error: ReplyError) -> Self {
        match error {
            ReplyError::Unauthorized => Self::Unauthorized,
            ReplyError::Host(code) => Self::Host(code),
            ReplyError::Invalid => Self::Unavailable,
        }
    }
}
