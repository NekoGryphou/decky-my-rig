//! The client side of one OPAQUE pairing exchange, as a safe Rust API.
mod errors;
mod service;
mod types;

pub use errors::ClientError;
pub use types::ClientExchange;

#[cfg(test)]
mod tests;
