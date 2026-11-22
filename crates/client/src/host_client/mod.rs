//! One HTTP/1.1 request per connection, over TLS pinned to the certificate
//! saved at pairing.
mod consts;
mod errors;
mod pinning;
mod service;
mod types;
mod utils;

pub use errors::HostError;
pub use types::{HostClient, HttpReply, ShutdownResult, Status};
pub use utils::parse_reply;

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod tests;
