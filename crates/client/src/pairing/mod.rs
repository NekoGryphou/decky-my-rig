//! Pairing with a PC: ask it to show a code, then do one OPAQUE exchange over
//! TLS bound to the certificate it presented. There's no plaintext fallback.
mod consts;
mod messages;
mod service;
mod types;
mod utils;

pub use service::{pair, request_code, revoke};
pub use types::Paired;
pub use utils::{device_name, device_name_from};

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;
