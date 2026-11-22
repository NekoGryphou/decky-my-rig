use super::Snapshots;
use myrig_protocol::api::{SnapshotFrame, TestStops};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

fn frame(revision: u64, count: u64) -> SnapshotFrame {
    SnapshotFrame {
        revision,
        refreshed: false,
        pcs: Vec::new(),
        test_stops: TestStops { count, name: None },
        closed: false,
    }
}

const SHORT: Duration = Duration::from_millis(50);

#[tokio::test]
async fn a_change_wakes_every_waiter_once() {
    let snapshots = Arc::new(Snapshots::new());
    snapshots.publish(frame(1, 0));
    let waiters: Vec<_> = (0..3)
        .map(|_| {
            let snapshots = snapshots.clone();
            tokio::spawn(async move { snapshots.watch(1).await })
        })
        .collect();
    tokio::task::yield_now().await;
    snapshots.publish(frame(2, 0));
    for waiter in waiters {
        assert_eq!(waiter.await.unwrap().unwrap().revision, 2);
    }
}

#[tokio::test]
async fn an_older_frame_never_replaces_a_newer_one() {
    let snapshots = Snapshots::new();
    snapshots.publish(frame(5, 2));
    snapshots.publish(frame(4, 1));
    assert_eq!(snapshots.watch(-1).await.unwrap(), frame(5, 2));
}

#[tokio::test]
async fn a_known_revision_waits_and_another_returns_the_latest() {
    let snapshots = Snapshots::new();
    assert!(timeout(SHORT, snapshots.watch(-1)).await.is_err());
    snapshots.publish(frame(3, 0));
    assert_eq!(snapshots.watch(-1).await.unwrap().revision, 3);
    // - A cursor from an earlier session gets the current frame right away.
    assert_eq!(snapshots.watch(9).await.unwrap().revision, 3);
    assert!(timeout(SHORT, snapshots.watch(3)).await.is_err());
}

#[tokio::test]
async fn invalid_revisions_are_refused() {
    let snapshots = Snapshots::new();
    assert_eq!(
        snapshots.watch(-2).await.unwrap_err().0,
        "Invalid snapshot revision."
    );
}

#[tokio::test]
async fn closing_releases_waiters_with_a_closed_frame() {
    let snapshots = Arc::new(Snapshots::new());
    snapshots.publish(frame(2, 1));
    let waiter = {
        let snapshots = snapshots.clone();
        tokio::spawn(async move { snapshots.watch(2).await })
    };
    tokio::task::yield_now().await;
    snapshots.close();
    let closed = waiter.await.unwrap().unwrap();
    assert!(closed.closed && !closed.refreshed);
    assert_eq!((closed.revision, closed.test_stops.count), (2, 1));
    snapshots.publish(frame(3, 0));
    assert!(snapshots.watch(2).await.unwrap().closed);
}

#[tokio::test]
async fn a_cancelled_waiter_does_not_affect_the_next() {
    let snapshots = Snapshots::new();
    snapshots.publish(frame(1, 0));
    assert!(timeout(SHORT, snapshots.watch(1)).await.is_err());
    snapshots.publish(frame(2, 0));
    assert_eq!(snapshots.watch(1).await.unwrap().revision, 2);
}
