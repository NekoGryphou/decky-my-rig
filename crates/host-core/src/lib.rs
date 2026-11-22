//! Host behavior that doesn't depend on the OS, used by the Windows service.
pub mod events;
pub mod file_storage;
pub mod host;
pub mod http;
pub mod listener;
pub mod local_control;
pub mod paired_devices;
pub mod pairing_entropy;
pub mod provisioning;
pub mod security_bundle;
pub mod service_loop;
pub mod shutdown;
pub mod tls;
