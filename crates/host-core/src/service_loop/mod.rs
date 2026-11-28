//! Stops the service's endpoints together and expires pairing codes on time.
mod errors;
mod service;

pub use errors::ServiceLoopError;
pub use service::join_endpoints;

#[cfg(test)]
mod tests;
