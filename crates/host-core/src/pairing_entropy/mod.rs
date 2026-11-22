//! Only OS randomness: no clock, MAC, reused token or fallback.
mod consts;
mod messages;
mod utils;

pub use utils::{generate, nonce};

#[cfg(test)]
mod tests;
