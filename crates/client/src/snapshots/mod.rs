//! The latest snapshot frame, and long-poll waiters for the next one.
mod messages;
mod service;
mod types;

pub use types::Snapshots;

#[cfg(test)]
mod tests;
