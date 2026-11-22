use std::time::Duration;

pub(super) const MAX_CONNECTIONS: usize = 16;
/// So one machine on the LAN can't take every slot and lock the device out.
pub(super) const MAX_CONNECTIONS_PER_PEER: usize = 4;
pub(super) const HEADER_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) const BODY_TIMEOUT: Duration = Duration::from_secs(2);
pub(super) const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
pub(super) const BODY_LIMIT: usize = 4096;
