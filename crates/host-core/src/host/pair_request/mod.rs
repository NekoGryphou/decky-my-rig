//! A device on the network asking this PC to show a pairing code.
mod consts;
mod service;
mod utils;

#[cfg(test)]
pub(crate) use consts::REQUEST_INTERVAL;
