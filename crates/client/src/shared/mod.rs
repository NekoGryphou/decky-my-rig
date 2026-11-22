//! Input rules and the public error type every client feature uses.
mod errors;
mod messages;
mod utils;

pub use errors::PublicError;
pub use utils::{
    normalize_host, normalize_mac, normalize_name, normalize_port, normalize_token, normalize_uuid,
};

#[cfg(test)]
mod tests;
