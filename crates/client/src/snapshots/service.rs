use super::{messages::INVALID_REVISION, types::Snapshots};
use crate::shared::PublicError;
use myrig_protocol::api::{SnapshotFrame, TestStops};
use tokio::sync::watch;

impl Default for Snapshots {
    fn default() -> Self {
        Self::new()
    }
}

impl Snapshots {
    pub fn new() -> Self {
        Self {
            latest: watch::Sender::new(None),
        }
    }

    /// Stores `frame` unless a newer one or the closed frame is already there.
    pub fn publish(&self, frame: SnapshotFrame) {
        self.latest.send_if_modified(|latest| match latest {
            Some(current) if current.closed || current.revision >= frame.revision => false,
            _ => {
                *latest = Some(frame);
                true
            }
        });
    }

    /// Ends the stream: every waiter, now or later, gets the closed frame.
    pub fn close(&self) {
        self.latest.send_modify(|latest| {
            let frame = latest.take().unwrap_or(SnapshotFrame {
                revision: 0,
                refreshed: false,
                pcs: Vec::new(),
                test_stops: TestStops {
                    count: 0,
                    name: None,
                },
                closed: true,
            });
            *latest = Some(SnapshotFrame {
                closed: true,
                refreshed: false,
                ..frame
            });
        });
    }

    /// The latest frame if the caller hasn't seen it yet (`after` is the last
    /// revision it saw, or -1), otherwise waits for the next one.
    pub async fn watch(&self, after: i64) -> Result<SnapshotFrame, PublicError> {
        if after < -1 {
            return Err(PublicError(INVALID_REVISION));
        }
        let mut receiver = self.latest.subscribe();
        loop {
            if let Some(frame) = receiver.borrow_and_update().as_ref()
                && (frame.closed || i64::try_from(frame.revision).ok() != Some(after))
            {
                return Ok(frame.clone());
            }
            // ? The sender lives as long as `self`, so this can't fail while
            //   anyone can still call `watch`.
            if receiver.changed().await.is_err() {
                return Err(PublicError(INVALID_REVISION));
            }
        }
    }
}
