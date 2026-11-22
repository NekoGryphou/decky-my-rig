use super::super::*;
use super::fixtures::*;
use crate::host::PersistError;
use serde_json::json;

#[test]
fn set_port_persists_then_publishes_and_rejects_bad_input() {
    let host = host();
    let mut saved = Vec::new();
    let result: serde_json::Value = serde_json::from_slice(&execute(
        &host,
        fixture("set-port-request").as_bytes(),
        |bytes| {
            assert!(host.test_mode().unwrap());
            saved.extend_from_slice(bytes);
            Ok(())
        },
    ))
    .unwrap();
    assert_eq!(result, fixture_json("set-port-reply"));
    let stored: serde_json::Value = serde_json::from_slice(&saved).unwrap();
    assert_eq!(stored["port"], 48000);
    assert_eq!(stored["paired_devices"].as_array().unwrap().len(), 2);
    assert_eq!(host.stored_port().unwrap(), 48000);

    // - Same port again: nothing to save.
    assert_eq!(
        reply(&host, fixture("set-port-request").as_bytes())["port"],
        48000
    );
    for bad in [
        json!({"version":1,"operation":"set_port","host_id":HOST,"port":0}),
        json!({"version":1,"operation":"set_port","host_id":HOST,"port":1023}),
        json!({"version":1,"operation":"set_port","host_id":HOST,"port":65536}),
        json!({"version":1,"operation":"set_port","host_id":HOST,"port":"48000"}),
        json!({"version":1,"operation":"set_port","host_id":HOST}),
    ] {
        assert_eq!(
            reply(&host, bad.to_string().as_bytes())["error"],
            "invalid_request"
        );
    }
    let other = json!({"version":1,"operation":"set_port",
        "host_id":"22222222-2222-4222-8222-222222222222","port":48001});
    assert_eq!(
        reply(&host, other.to_string().as_bytes())["error"],
        "wrong_host"
    );
}

#[test]
fn set_port_save_failure_keeps_the_stored_port() {
    let host = host();
    let failed: serde_json::Value = serde_json::from_slice(&execute(
        &host,
        fixture("set-port-request").as_bytes(),
        |_| Err(PersistError::Unchanged),
    ))
    .unwrap();
    assert_eq!(failed["error"], "storage_failed");
    assert_eq!(host.stored_port().unwrap(), 49000);
}

#[test]
fn set_test_mode_persists_before_publish_and_rejects_wrong_type() {
    let host = host();
    let mut saved = Vec::new();
    let result: serde_json::Value = serde_json::from_slice(&execute(
        &host,
        include_str!("../../../../../tests/fixtures/local-control/set-test-mode-request.json")
            .as_bytes(),
        |bytes| {
            saved.extend_from_slice(bytes);
            Ok(())
        },
    ))
    .unwrap();
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/local-control/set-test-mode-reply.json"
    ))
    .unwrap();
    assert_eq!(result, expected);
    assert!(
        !serde_json::from_slice::<serde_json::Value>(&saved).unwrap()["test_mode"]
            .as_bool()
            .unwrap()
    );
    assert!(!host.test_mode().unwrap());
    let failing = self::host();
    let failed = execute(
        &failing,
        include_str!("../../../../../tests/fixtures/local-control/set-test-mode-request.json")
            .as_bytes(),
        |_| Err(PersistError::Unchanged),
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&failed).unwrap()["error"],
        "storage_failed"
    );
    assert!(failing.test_mode().unwrap());
    let bad = include_str!(
        "../../../../../tests/fixtures/local-control/invalid-set-test-mode-request.json"
    );
    assert_eq!(reply(&host, bad.as_bytes())["error"], "invalid_request");
}

#[test]
fn status_fixture_reports_live_management_state_without_secrets() {
    let host = host();
    let response = reply(
        &host,
        include_str!("../../../../../tests/fixtures/local-control/status-request.json").as_bytes(),
    );
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/local-control/status-reply.json"
    ))
    .unwrap();
    assert_eq!(response, expected);
    assert!(response.get("paired_devices").is_none());
}

#[test]
fn status_reports_listener_failure_and_saved_port_change_requests_rebind() {
    let host = host();
    host.set_listener_unavailable();
    let status = reply(&host, br#"{"version":1,"operation":"status"}"#);
    assert_eq!(status["listener_state"], "unavailable");
    let mut ports = host.subscribe_port();
    let request =
        serde_json::json!({"version":1,"operation":"set_port","host_id":HOST,"port":48000});
    let result = execute(&host, request.to_string().as_bytes(), |_| Ok(()));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result).unwrap()["port"],
        48000
    );
    assert!(ports.has_changed().unwrap());
    assert_eq!(*ports.borrow_and_update(), 48000);
}

#[test]
fn concurrent_port_saves_leave_notification_at_the_stored_port() {
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering},
    };
    let host = Arc::new(host());
    let barrier = Barrier::new(5);
    let finish = AtomicBool::new(false);
    let mismatch = std::thread::scope(|scope| {
        for worker in 0..4 {
            let host = &host;
            let barrier = &barrier;
            let finish = &finish;
            scope.spawn(move || {
                loop {
                    barrier.wait();
                    if finish.load(Ordering::Relaxed) {
                        break;
                    }
                    host.set_port_checked(20000 + worker, |_| Ok(())).unwrap();
                    barrier.wait();
                }
            });
        }
        let mut mismatch = None;
        for _ in 0..20_000 {
            barrier.wait();
            barrier.wait();
            let stored = host.stored_port().unwrap();
            let notified = *host.subscribe_port().borrow();
            if stored != notified {
                mismatch = Some((stored, notified));
                break;
            }
        }
        finish.store(true, Ordering::Relaxed);
        barrier.wait();
        mismatch
    });
    assert_eq!(mismatch, None);
}
