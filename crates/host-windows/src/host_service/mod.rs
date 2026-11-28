//! The service's lifetime under the SCM, its fixed protected config path, and
//! status reporting.
mod service;
mod types;
mod utils;

pub use crate::shared::consts::SERVICE_NAME;
pub use service::run;

#[cfg(test)]
mod tests;
