//! Host events as local management clients receive them.
mod consts;
mod types;

pub use consts::EVENT_CAPACITY;
pub use types::{EventKind, HostEvent};
