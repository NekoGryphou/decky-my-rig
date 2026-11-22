//! The fixed TLS identity. There's no plaintext fallback.
mod service;
mod types;
mod utils;

pub use types::FixedIdentity;

#[cfg(test)]
mod tests;
