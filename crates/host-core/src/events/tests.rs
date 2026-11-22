use super::*;
use myrig_protocol::events::{EVENT_CAPACITY, EventKind};
use std::time::Duration;

#[test]
fn latest_sequence_is_available_before_the_first_wait() {
    let events = HostEvents::new();
    assert_eq!(events.latest_sequence(), 0);
    events.record(EventKind::TestShutdown, "Device".into());
    assert_eq!(events.latest_sequence(), 1);
}

#[test]
fn ring_keeps_last_sixteen_and_order() {
    let events = HostEvents::new();
    for index in 1..=18 {
        events.record(EventKind::TestShutdown, format!("Device {index}"));
        assert_eq!(events.latest_sequence(), index);
    }
    let latest = events.events_after(0);
    assert_eq!(latest.len(), EVENT_CAPACITY);
    assert_eq!(latest[0].sequence, 3);
    assert_eq!(latest[0].device_name, "Device 3");
    assert_eq!(latest[15].sequence, 18);
    assert_eq!(latest[15].device_name, "Device 18");
    assert!(latest.iter().all(|event| event.time_unix > 0));
}

#[tokio::test]
async fn wait_returns_immediately_when_newer_exists() {
    let events = HostEvents::new();
    events.record(EventKind::TestShutdown, "Device".into());
    assert_eq!(events.wait_after(0, Duration::from_secs(1)).await.len(), 1);
    assert!(events.events_after(1).is_empty());
}

#[tokio::test(start_paused = true)]
async fn wait_times_out_empty() {
    let events = HostEvents::new();
    assert!(
        events
            .wait_after(0, Duration::from_secs(25))
            .await
            .is_empty()
    );
}

#[tokio::test]
async fn wait_wakes_on_record_and_close() {
    // - Waiting doesn't tie up a thread, so the recorder can run while the waiter
    //   waits.
    let events = HostEvents::new();
    let waiting = events.clone();
    let task = tokio::spawn(async move { waiting.wait_after(0, Duration::from_secs(25)).await });
    tokio::task::yield_now().await;
    events.record(EventKind::TestShutdown, "Device".into());
    assert_eq!(task.await.unwrap()[0].device_name, "Device");

    let waiting = events.clone();
    let task = tokio::spawn(async move { waiting.wait_after(1, Duration::from_secs(25)).await });
    tokio::task::yield_now().await;
    events.close();
    assert!(task.await.unwrap().is_empty());
}

#[test]
fn events_carry_their_kind_on_the_wire() {
    let events = HostEvents::new();
    events.record(EventKind::TestShutdown, "Device".into());
    events.record(EventKind::PairingRequest, "steamdeck".into());
    let recorded = events.events_after(0);
    assert_eq!(recorded[0].kind, EventKind::TestShutdown);
    assert_eq!(recorded[1].kind, EventKind::PairingRequest);
    let wire = serde_json::to_value(&recorded[1]).unwrap();
    assert_eq!(wire["kind"], "pairing_request");
    assert_eq!(wire["device_name"], "steamdeck");
}
