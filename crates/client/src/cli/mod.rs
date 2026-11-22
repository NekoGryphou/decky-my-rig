//! The `myrig` command. It keeps its saved PCs in the platform's settings
//! folder, separate from the Decky plugin's.
mod consts;
mod messages;
mod service;
mod types;
mod utils;

pub use service::{main, run};
pub use types::Terminal;

#[cfg(test)]
mod tests;
