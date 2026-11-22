//! Trusts a certificate by its exact SHA-256 pin, while still checking
//! signatures for real.
mod consts;
mod service;
mod types;

pub(super) use types::PinVerifier;
