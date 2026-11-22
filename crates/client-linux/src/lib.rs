//! The Linux parts of the `myrig` client: the settings directory, network
//! interfaces, the neighbor table and the hostname. Empty on other systems.
#[cfg(target_os = "linux")]
mod neighbors;
#[cfg(target_os = "linux")]
mod network;
#[cfg(target_os = "linux")]
mod platform;
#[cfg(target_os = "linux")]
mod settings;

#[cfg(target_os = "linux")]
pub use platform::Linux;
