//! Pairing with a PC: ask it to show a code, then do one OPAQUE exchange over
//! TLS bound to the certificate it presented. There's no plaintext fallback.
mod consts;
mod messages;
mod service;
mod types;
mod utils;

pub(crate) use consts::CREDENTIAL_BUDGET;
pub(crate) use service::pair_tracked;
pub use service::{pair, request_code, revoke};
pub use types::Paired;
pub use utils::device_name;
#[cfg(test)]
pub(crate) use utils::device_name_from;

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;
