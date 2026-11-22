//! The host's shared state: the registry, pairing sessions and lifecycle flags.
mod errors;
mod pair_request;
mod pairing;
mod registry;
mod service;
mod types;
mod utils;

pub use errors::{OpenPairingError, PairRequestError, PairingRefusal, PersistError, RegistryError};
pub use types::{Host, PairingApproval, PairingPersistence, SecureChallenge};
#[cfg(test)]
pub(crate) use {pair_request::REQUEST_INTERVAL, types::PairingReservation};
