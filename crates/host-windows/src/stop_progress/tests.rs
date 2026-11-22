use super::*;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::watch;

#[tokio::test]
async fn pending_stop_advances_checkpoints_until_work_finishes() {
    let (send, receive) = watch::channel(false);
    let (done_tx, done_rx) = tokio::sync::oneshot::channel::<()>();
    let seen = Arc::new(Mutex::new(Vec::new()));
    let reported = seen.clone();
    let task = tokio::spawn(async move {
        until_done(
            async { done_rx.await.unwrap() },
            receive,
            move |checkpoint| {
                reported.lock().unwrap().push(checkpoint);
                Ok(())
            },
        )
        .await
    });
    send.send(true).unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while seen.lock().unwrap().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        seen.lock()
            .unwrap()
            .iter()
            .all(|checkpoint| *checkpoint >= 2)
    );
    done_tx.send(()).unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap(),
        ((), false)
    );
}

#[tokio::test]
async fn stop_after_long_uptime_does_not_replay_missed_checkpoints() {
    tokio::time::pause();
    let (send, receive) = watch::channel(false);
    let (done_tx, done_rx) = tokio::sync::oneshot::channel::<()>();
    let reports = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = reports.clone();
    let task = tokio::spawn(until_done(
        async { done_rx.await.unwrap() },
        receive,
        move |_| {
            count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Ok(())
        },
    ));
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(86_400)).await;
    send.send(true).unwrap();
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }
    let observed = reports.load(std::sync::atomic::Ordering::Relaxed);
    done_tx.send(()).unwrap();
    task.await.unwrap();
    assert!(observed <= 2, "replayed {observed} missed checkpoints");
}
