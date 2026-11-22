//! The Windows parts of the `myrig` client: the settings directory, network
//! interfaces, the neighbor table and the hostname. Empty on other systems.
#[cfg(windows)]
mod neighbors;
#[cfg(windows)]
mod network;
#[cfg(windows)]
mod platform;
#[cfg(windows)]
mod settings;

#[cfg(windows)]
pub use platform::Windows;
