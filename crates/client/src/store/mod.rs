//! The saved PCs: one private `pcs.json` per settings directory.
mod consts;
mod messages;
mod service;
mod types;
mod utils;

pub use types::{Authentication, PcChanges, PcConfig, PcInput, Store};

#[cfg(test)]
mod tests;
