//! The service's lifetime under the SCM, its fixed protected config path, and
//! status reporting.
mod service;
mod types;

pub use crate::shared::consts::SERVICE_NAME;
pub use service::run;
