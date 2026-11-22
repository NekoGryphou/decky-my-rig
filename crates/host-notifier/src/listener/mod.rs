//! Listens for local events and reconnects with a capped backoff.
mod consts;
mod messages;
mod types;
mod utils;

pub use messages::{UNAUTHORIZED, UNAUTHORIZED_TITLE};
pub use types::Notifier;
pub use utils::run;

#[cfg(test)]
mod tests;
