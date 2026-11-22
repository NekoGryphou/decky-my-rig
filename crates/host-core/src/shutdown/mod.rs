//! Coordinates scheduled shutdowns. Repeating a request ID doesn't schedule a
//! second one.
mod consts;
mod errors;
mod service;
mod types;

pub use consts::{PENDING_TIMEOUT, SHUTDOWN_GRACE};
pub use errors::{PowerError, ShutdownError};
pub use types::{Core, Power};

#[cfg(test)]
mod tests;
