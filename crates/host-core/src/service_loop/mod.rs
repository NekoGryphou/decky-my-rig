//! Stops the service's endpoints together and expires pairing codes on time.
mod errors;
mod utils;

pub use errors::ServiceLoopError;
pub use utils::join_endpoints;

#[cfg(test)]
mod tests;
