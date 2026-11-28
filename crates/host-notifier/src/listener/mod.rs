//! Listens for local events and reconnects with a capped backoff.
mod consts;
mod messages;
mod service;
mod types;
mod utils;

pub use messages::{UNAUTHORIZED, UNAUTHORIZED_TITLE};
pub use service::run;
pub use types::Notifier;

#[cfg(test)]
mod tests;
