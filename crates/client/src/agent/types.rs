use crate::{
    drafts::Drafts,
    power::{Controller, LiveEnvironment},
};
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Deserialize)]
pub(super) struct Request {
    pub(super) id: u64,
    pub(super) method: String,
    #[serde(default)]
    pub(super) params: Value,
}

/// State shared by all request handlers.
pub(super) struct Agent {
    pub(super) controller: Arc<Controller<LiveEnvironment>>,
    pub(super) drafts: Drafts,
    /// Wakes the poller early after a change.
    pub(super) changed: Notify,
    /// Set while a plugin view is on screen, so status stays fresh.
    pub(super) attended: std::sync::atomic::AtomicBool,
}
