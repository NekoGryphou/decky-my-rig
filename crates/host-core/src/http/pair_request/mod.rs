//! The unauthenticated route a device uses to ask this PC to show a pairing
//! code.
mod utils;

pub(super) use utils::handle;

#[cfg(test)]
mod tests;
