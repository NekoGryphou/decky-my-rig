//! The HTTP/1 accept loop and a dispatcher with fixed limits.
mod consts;
mod pair_request;
mod pairing;
mod server;
mod shutdown;
mod types;
mod utils;

pub use server::serve_secure_tls;

#[cfg(test)]
mod fixtures;
#[cfg(test)]
mod tests;
