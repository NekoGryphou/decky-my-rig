//! Local control connections, with fixed limits, where the caller is authorized
//! before anything runs.
mod binding;
mod consts;
mod native;
mod service;
mod types;
mod utils;

pub use types::ControlPipe;

#[cfg(test)]
mod tests;
