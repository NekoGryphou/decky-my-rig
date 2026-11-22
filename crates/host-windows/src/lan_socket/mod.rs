//! The LAN listener's socket, bound on every interface with exclusive use.
mod native;
#[cfg(test)]
mod tests;

pub use native::bind_exclusive;
