use crate::{RESPONSE_LEN, Suite};
use opaque_ke::{ServerLogin, ServerRegistration, ServerSetup};
use std::time::Instant;
use subtle::{Choice, ConstantTimeEq};

/// An ID the server generated, not a credential. Its Debug output shows no
/// secret.
#[derive(Clone, Copy)]
pub struct AttemptId(pub(super) [u8; 32]);
// ! Compared in constant time, so response timing says nothing about how many leading bytes of a
// ! guessed ID were right.
impl ConstantTimeEq for AttemptId {
    fn ct_eq(&self, other: &Self) -> Choice {
        self.0.ct_eq(&other.0)
    }
}
pub struct Challenge {
    pub attempt: AttemptId,
    pub response: [u8; RESPONSE_LEN],
}
/// One-time proof of success. It holds no session key or credential.
pub struct Authenticated {
    pub(super) _private: (),
}

pub(super) struct Exchange {
    pub(super) id: AttemptId,
    pub(super) until: Instant,
    pub(super) login: ServerLogin<Suite>,
}
pub struct ApprovedSession {
    pub(super) setup: Option<ServerSetup<Suite>>,
    pub(super) record: Option<ServerRegistration<Suite>>,
    pub(super) context: Vec<u8>,
    pub(super) until: Instant,
    pub(super) starts: u8,
    pub(super) exchange: Option<Exchange>,
}
