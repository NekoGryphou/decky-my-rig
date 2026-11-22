//! Typed requests and replies for local management clients (CLI and notifier).
mod consts;
mod types;
mod utils;

pub use types::{
    ControlRequest, DeviceEntry, ListReply, OpenPairingReply, PortReply, RemoveReply, RenameReply,
    Reply, ReplyError, StatusReply, TestModeReply, WaitReply,
};
pub use utils::{encode, exchange, parse};

#[cfg(test)]
mod tests;
