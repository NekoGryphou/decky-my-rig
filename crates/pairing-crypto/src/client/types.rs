use crate::Suite;
use opaque_ke::ClientLogin;
use zeroize::Zeroizing;

/// One started exchange. `finish` consumes it, so a failed or late exchange
/// can never produce a second confirmation.
pub struct ClientExchange {
    pub(super) state: ClientLogin<Suite>,
    pub(super) code: Zeroizing<Vec<u8>>,
    pub(super) context: Vec<u8>,
}
