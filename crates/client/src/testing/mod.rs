//! Store fixtures shared by this crate's tests and the platform crates' tests
//! (feature `testing`).
mod fixtures;

pub use fixtures::{StoreFixture, TOKEN, authentication, input};
