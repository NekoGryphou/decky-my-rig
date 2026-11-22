use crate::{host_client::HostError, store::Authentication};

/// The credential a completed pairing saved on the PC, and the PC's name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Paired {
    pub authentication: Authentication,
    pub computer_name: String,
}

/// Why an exchange stopped, before it's turned into a public message.
pub(super) enum Failure {
    Host(HostError),
    Public(&'static str),
}
