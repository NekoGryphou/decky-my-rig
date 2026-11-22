use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WakeError {
    /// The saved MAC address is not a unicast address.
    InvalidMac,
    /// No interface that's up and can broadcast has the PC in its subnet.
    NoInterface,
    /// The packet couldn't be sent in full.
    Send,
}

impl fmt::Display for WakeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidMac => "The MAC address is not a unicast address.",
            Self::NoInterface => "No network interface reaches the PC's subnet.",
            Self::Send => "The Wake-on-LAN packet was not sent.",
        })
    }
}

impl std::error::Error for WakeError {}
