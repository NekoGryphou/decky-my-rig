use myrig_protocol::lan::HostState;
use std::{net::SocketAddrV4, time::Duration};
use uuid::Uuid;
use zeroize::Zeroizing;

/// A client for one host. Without a pin it can only send the two bootstrap
/// requests; with one, nothing is sent unless the certificate matches.
pub struct HostClient {
    pub(super) address: SocketAddrV4,
    pub(super) token: Option<Zeroizing<String>>,
    pub(super) pin: Option<[u8; 32]>,
    pub(super) deadline: Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status {
    pub host_id: Uuid,
    pub state: HostState,
}

/// `test` is true when test mode kept the PC on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShutdownResult {
    pub test: bool,
}

/// A complete, size-checked HTTP reply. The body can contain a new token, so
/// `Debug` leaves it out.
#[derive(Clone, PartialEq, Eq)]
pub struct HttpReply {
    pub status: u16,
    pub content_type: Option<String>,
    pub cache_control: Option<String>,
    pub body: Vec<u8>,
}

impl std::fmt::Debug for HttpReply {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HttpReply")
            .field("status", &self.status)
            .field("content_type", &self.content_type)
            .field("cache_control", &self.cache_control)
            .finish_non_exhaustive()
    }
}
