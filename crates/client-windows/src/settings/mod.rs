//! The private settings directory: owned by this user, with a protected ACL
//! that only grants this user, and never reached through a reparse point or
//! hard link.
mod consts;
mod native;
mod service;
mod types;

pub(crate) use types::WindowsSettings;

#[cfg(test)]
mod tests;
