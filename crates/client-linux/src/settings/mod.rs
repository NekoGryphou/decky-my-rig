//! The private settings directory: 0700, with `pcs.json` at 0600, owned by this
//! user and never reached through a link.
mod consts;
mod native;
mod service;
mod types;

pub(crate) use types::LinuxSettings;

#[cfg(test)]
mod tests;
