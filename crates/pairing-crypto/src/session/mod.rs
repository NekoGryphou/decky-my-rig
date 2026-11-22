//! OPAQUE sessions that are approved locally, tied to a certificate, and used
//! once.
mod consts;
mod errors;
mod service;
mod types;

pub use consts::{LIFETIME, MAX_STARTS};
pub use errors::Error;
pub use types::{ApprovedSession, AttemptId, Authenticated, Challenge};

#[cfg(test)]
mod tests;
