//! Data folders that only ever exist with their final, protected permissions.
mod consts;
mod native;
mod service;
#[cfg(test)]
mod tests;

pub use service::create_protected_directories;
