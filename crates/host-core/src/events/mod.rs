//! Host events kept in memory (a bounded number) for local management clients.
mod service;
mod types;

pub use types::HostEvents;

#[cfg(test)]
mod tests;
