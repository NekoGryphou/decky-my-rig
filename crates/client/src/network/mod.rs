//! Wake-on-LAN: sends one magic packet to the directed broadcast address of the
//! interface whose subnet contains the PC.
mod consts;
mod errors;
mod service;
mod types;
mod utils;

pub use errors::WakeError;
pub use service::wake;
pub use types::Interface;

#[cfg(test)]
use {
    service::{broadcast_interfaces, send_packet},
    utils::{interface_from, magic_packet, select_directed_broadcast},
};

#[cfg(test)]
mod tests;
