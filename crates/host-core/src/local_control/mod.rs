//! Strict local control messages. Who's calling is decided by the pipe, not by
//! the message.
mod consts;
#[cfg(test)]
mod tests;
mod types;
mod utils;

pub use utils::{execute, execute_secure, is_event_wait_request};
