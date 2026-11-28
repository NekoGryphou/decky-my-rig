use myrig_protocol::api::PcIssue;
use std::fmt;

/// Why a host request failed. None of these include text from the host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostError {
    /// The host refused the credential (HTTP 401).
    Unauthorized,
    /// The host presented a certificate other than the pinned one.
    PinMismatch,
    /// The host speaks another protocol major version.
    UnsupportedProtocol,
    /// The reply broke the protocol, or the request is not allowed.
    Invalid,
    /// No connection, a dropped connection, or a host-side failure.
    Connection,
    /// The request deadline passed.
    Timeout,
}

impl HostError {
    /// The answers the user has to deal with, rather than something to retry.
    pub fn issue(self) -> Option<PcIssue> {
        match self {
            Self::PinMismatch => Some(PcIssue::Identity),
            Self::Unauthorized => Some(PcIssue::Authentication),
            Self::UnsupportedProtocol => Some(PcIssue::Compatibility),
            _ => None,
        }
    }
}

impl fmt::Display for HostError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Unauthorized => "Authentication failed.",
            Self::PinMismatch => "Host certificate does not match.",
            Self::UnsupportedProtocol => "Unsupported protocol major version.",
            Self::Invalid => "Invalid host response.",
            Self::Connection => "Host connection failed.",
            Self::Timeout => "Host request timed out.",
        })
    }
}

impl std::error::Error for HostError {}
