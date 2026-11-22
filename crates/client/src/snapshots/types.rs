use myrig_protocol::api::SnapshotFrame;
use tokio::sync::watch;

/// Holds the newest frame. A closed frame is final.
pub struct Snapshots {
    pub(super) latest: watch::Sender<Option<SnapshotFrame>>,
}
