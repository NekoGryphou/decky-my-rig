use std::{future::Future, time::Duration};
use tokio::sync::watch;

pub async fn until_done<T>(
    work: impl Future<Output = T>,
    mut stopping: watch::Receiver<bool>,
    mut report: impl FnMut(u32) -> Result<(), ()>,
) -> (T, bool) {
    tokio::pin!(work);
    let mut interval = tokio::time::interval(Duration::from_secs(1));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut checkpoint = 1u32;
    let mut status_failed = false;
    loop {
        tokio::select! {
            biased;
            result = &mut work => return (result, status_failed),
            _ = stopping.changed(), if !*stopping.borrow() => {},
            _ = interval.tick(), if *stopping.borrow() => {
                checkpoint = checkpoint.saturating_add(1);
                status_failed |= report(checkpoint).is_err();
            },
        }
    }
}
