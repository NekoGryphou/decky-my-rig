//! The client end of the control pipe, used by the host CLI, the notifier and
//! the service's own tests.
pub(crate) mod consts;
mod native;
mod utils;

pub use native::connect_client;
pub use utils::control_pipe_name;
