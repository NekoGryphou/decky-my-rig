//! Reads and writes the protected config and identity files.
mod consts;
mod errors;
mod service;
mod types;
mod utils;

pub use consts::SECURITY_FILE;
pub use errors::StorageError;
pub use types::FileStorage;
pub use utils::load_security;

#[cfg(test)]
mod tests;
