//! One running copy per Windows session.
mod native;

pub use native::{InstanceGuard, acquire};

#[cfg(test)]
mod tests;
