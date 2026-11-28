//! Windows shutdown through the system_shutdown crate.
// ! Windows shutdown: `InitiateSystemShutdownW` after the privilege it needs. The Win32 calls live
// ! in `native.rs`; a Linux backend would add its own module beside them.
mod native;
mod service;
#[cfg(test)]
mod tests;
mod types;

pub use types::WindowsPower;
