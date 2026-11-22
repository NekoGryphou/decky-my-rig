//! Which Windows session this process runs in.
mod native;

pub use native::is_console_session;
