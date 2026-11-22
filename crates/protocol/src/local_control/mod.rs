//! Local-control wire format: framing, limits and the typed client.
pub mod client;
mod consts;
pub mod framing;

pub use consts::{
    CONTROL_SOCKET_NAME, MIN_SETTABLE_PORT, PAIRING_CODE_LIFETIME, REQUEST_LIMIT, RESPONSE_LIMIT,
};
