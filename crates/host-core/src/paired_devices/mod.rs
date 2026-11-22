//! The per-device registry (schema 3) and what gets saved to protected storage.
mod consts;
mod service;
mod types;
mod utils;

pub use consts::{MAX_CONFIG_BYTES, MAX_PAIRED_DEVICES};
pub use types::{PublicPairedDevice, PublicRegistryConfig, RegistryConfig};

#[cfg(test)]
mod tests;
