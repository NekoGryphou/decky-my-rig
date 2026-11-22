//! Live power state of the saved PCs: polling, Start (Wake-on-LAN), Stop, and
//! the issues that disable Stop.
mod consts;
mod environment;
mod messages;
mod service;
mod types;
mod utils;

pub use environment::{Environment, LiveEnvironment};
pub use types::Controller;

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod tests;
