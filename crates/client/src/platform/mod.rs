//! What the client needs from the OS. `myrig-client-linux` and
//! `myrig-client-windows` implement it, and the `myrig` binary picks one.
mod consts;
mod types;
mod utils;

pub use consts::SETTINGS_FOLDER;
pub use types::{InputEcho, Platform, Settings};
#[cfg(any(test, feature = "testing"))]
pub(crate) use utils::is_temporary_document;
pub use utils::{remove_leftover_documents, temporary_document_name};

#[cfg(test)]
pub(crate) mod fixtures;
#[cfg(test)]
mod tests;
