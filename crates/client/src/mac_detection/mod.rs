//! Finds the PC's MAC address in this device's neighbor table, so the user
//! doesn't have to type it.
mod messages;
mod service;
mod utils;

pub use service::detect_mac;

#[cfg(test)]
use utils::select_mac;

#[cfg(test)]
mod tests;
