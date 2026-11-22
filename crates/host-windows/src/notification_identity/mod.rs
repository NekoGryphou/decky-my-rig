mod consts;
mod native;
mod utils;

pub use consts::{NOTIFIER_APP_ID, NOTIFIER_DISPLAY_NAME, NOTIFIER_INSTANCE};
pub use native::{register, unregister};

#[cfg(test)]
mod tests;
