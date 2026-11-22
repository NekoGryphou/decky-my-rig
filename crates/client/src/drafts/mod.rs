//! One pairing at a time: reserve a slot, pair with the PC, then save what the
//! user confirms. A re-pair keeps its new credential right away.
mod messages;
mod service;
mod types;

pub use types::{Drafts, Reservation};

#[cfg(test)]
mod tests;
