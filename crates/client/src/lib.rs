//! The `myrig` client: saved PCs, pinned TLS to hosts, pairing, Wake-on-LAN and
//! power control. It runs as the `myrig` command and as the Decky plugin's
//! agent.
pub mod agent;
pub mod cli;
pub mod drafts;
pub mod host_client;
pub mod mac_detection;
pub mod network;
pub mod pairing;
pub mod platform;
pub mod power;
pub mod shared;
pub mod snapshots;
pub mod store;
// ! Test support only: a failed setup should panic, so expect is the point here.
#[cfg(any(test, feature = "testing"))]
#[allow(clippy::expect_used)]
pub mod testing;
