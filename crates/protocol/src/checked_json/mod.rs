//! JSON parsed with duplicate keys rejected at every level.
mod service;
mod types;

pub use types::CheckedJson;

#[cfg(test)]
mod tests;
