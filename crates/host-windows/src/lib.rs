//! Windows side of the host. The `service` feature adds the service itself (SCM
//! service, power, control pipe server, LAN socket). Without it you get the
//! client parts the host CLI, the notifier and setup need.
#[cfg(windows)]
pub mod control_client;
#[cfg(all(windows, feature = "service"))]
pub mod control_pipe;
#[cfg(windows)]
pub mod event_log;
#[cfg(all(windows, feature = "service"))]
pub mod host_service;
#[cfg(all(windows, feature = "service"))]
pub mod power;
#[cfg(all(windows, feature = "service"))]
mod program_data;
#[cfg(all(windows, feature = "service"))]
pub mod registry_control;
#[cfg(all(windows, feature = "service"))]
mod stop_progress;

#[cfg(windows)]
mod shared;

#[cfg(windows)]
pub mod server_identity;

#[cfg(windows)]
pub mod install_directory;

#[cfg(all(windows, feature = "service"))]
pub mod lan_socket;

#[cfg(windows)]
pub mod protected_directory;

#[cfg(windows)]
pub mod notification_identity;

#[cfg(windows)]
pub mod single_instance;

#[cfg(windows)]
pub mod session;
