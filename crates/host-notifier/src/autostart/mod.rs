//! The notifier's per-user login entry.
mod messages;
mod service;
mod types;
mod utils;

pub use messages::{APP_NAME, CANNOT_CONFIGURE, CANNOT_UPDATE, NO_ACTION};
pub use service::launch_path;
pub use types::Command;

#[cfg(test)]
mod tests;
