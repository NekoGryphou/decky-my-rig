//! What the client needs from the OS. `myrig-client-linux` and
//! `myrig-client-windows` implement it, and the `myrig` binary picks one.
mod consts;
mod types;
mod utils;

pub use consts::SETTINGS_FOLDER;
pub use types::{Platform, Settings};
pub use utils::{is_temporary_document, temporary_document_name};

#[cfg(test)]
pub(crate) mod fixtures;
