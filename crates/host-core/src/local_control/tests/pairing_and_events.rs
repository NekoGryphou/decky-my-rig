use super::super::*;
use super::fixtures::*;
use myrig_protocol::local_control::client;
use serde_json::json;
use std::time::Duration;

#[test]
fn notifier_event_client_uses_shared_wire_contract() {
    let request = client::encode(&client::ControlRequest::WaitEvents {
        after: 0,
        epoch: None,
    });
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/local-control/wait-events-request.json"
    ))
    .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&request).unwrap(),
        expected
    );
    let batch = client::parse::<client::WaitReply>(
        include_str!("../../../../../tests/fixtures/local-control/wait-events-reply.json")
            .as_bytes(),
    )
    .unwrap();
    assert_eq!(batch.baseline_sequence, 1);
    assert_eq!(batch.events[0].device_name, "Device");
    assert_eq!(batch.events[0].sequence, 1);
    assert!(
        client::parse::<client::WaitReply>(
            br#"{"version":1,"ok":true,"host_id":"bad","events":[]}"#
        )
        .is_err()
    );
}

#[tokio::test]
async fn shared_local_control_fixtures_match_dispatcher() {
    let host = host();
    assert_eq!(
        reply(&host, fixture("list-request").as_bytes()),
        fixture_json("list-reply")
    );
    assert_eq!(
        reply(&host, fixture("invalid-request-duplicate").as_bytes()),
        fixture_json("error-reply")
    );
    let response = execute_secure(
        &host,
        fixture("open-pairing-request").as_bytes(),
        host.pairing_approval(),
        |_| panic!("approval must not persist"),
        |name, _approval| async move {
            assert_eq!(name, "Device");
            Ok(zeroize::Zeroizing::new("00123456".to_string()))
        },
    )
    .await;
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&response).unwrap(),
        fixture_json("open-pairing-reply")
    );
    let removed = execute(&host, fixture("remove-request").as_bytes(), |_| Ok(()));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&removed).unwrap(),
        fixture_json("remove-reply")
    );
}

#[tokio::test]
async fn secure_approval_checks_host_and_strict_inputs_before_callback() {
    let host = host();
    let request = json!({"version":1,"operation":"open_pairing","host_id":HOST,"name":"Device"});
    let bytes = serde_json::to_vec(&request).unwrap();
    for (bytes, expected) in [
        (serde_json::to_vec(&json!({"version":1,"operation":"open_pairing","host_id":DEVICE,"name":"Device"})).unwrap(), "wrong_host"),
        (serde_json::to_vec(&json!({"version":1,"operation":"open_pairing","host_id":HOST,"name":" Device"})).unwrap(), "invalid_request"),
        (serde_json::to_vec(&json!({"version":1,"operation":"open_pairing","host_id":HOST,"name":"Device","nonce":"chosen"})).unwrap(), "invalid_request"),
        (br#"{"version":1,"operation":"open_pairing","host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","name":"Device","name":"Other"}"#.to_vec(), "invalid_request"),
    ] {
        let reply = execute_secure(&host, &bytes, host.pairing_approval(), |_| panic!("must not persist"), |_, _| async { panic!("must not approve") }).await;
        assert_eq!(serde_json::from_slice::<serde_json::Value>(&reply).unwrap()["error"], expected);
    }
    // * The reply holds the pairing code, so its buffer is wiped when dropped.
    let reply: zeroize::Zeroizing<Vec<u8>> = execute_secure(
        &host,
        &bytes,
        host.pairing_approval(),
        |_| panic!("approval does not issue a credential yet"),
        |name, _approval| async move {
            assert_eq!(name, "Device");
            Ok(zeroize::Zeroizing::new("12345678".to_string()))
        },
    )
    .await;
    let value: serde_json::Value = serde_json::from_slice(&reply).unwrap();
    assert_eq!(value["code"], "12345678");
    assert_eq!(value["expires_in_seconds"], 120);
    assert!(!value.to_string().contains("token"));
    for result in [
        Err("private error"),
        Ok(zeroize::Zeroizing::new("bad".to_string())),
    ] {
        let reply = execute_secure(
            &host,
            &bytes,
            host.pairing_approval(),
            |_| panic!("must not persist"),
            |_, _| async { result },
        )
        .await;
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&reply).unwrap()["error"],
            "unavailable"
        );
        assert!(
            !String::from_utf8(reply.to_vec())
                .unwrap()
                .contains("private")
        );
    }
}

#[tokio::test]
async fn wait_events_returns_names_and_rejects_wrong_type() {
    let host = host();
    host.events().record(
        myrig_protocol::events::EventKind::TestShutdown,
        "Device".into(),
    );
    let response = execute_secure(
        &host,
        include_str!("../../../../../tests/fixtures/local-control/wait-events-request.json")
            .as_bytes(),
        host.pairing_approval(),
        |_| panic!("wait must not persist"),
        |_, _| async { panic!("wait must not approve") },
    )
    .await;
    let response: serde_json::Value = serde_json::from_slice(&response).unwrap();
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/local-control/wait-events-reply.json"
    ))
    .unwrap();
    assert_eq!(response["host_id"], expected["host_id"]);
    assert_eq!(response["baseline_sequence"], 1);
    assert_eq!(
        response["events"][0]["device_name"],
        expected["events"][0]["device_name"]
    );
    assert!(response["events"][0]["time_unix"].as_u64().unwrap() > 0);
    let bad = include_str!(
        "../../../../../tests/fixtures/local-control/invalid-wait-events-request.json"
    );
    assert_eq!(reply(&host, bad.as_bytes())["error"], "invalid_request");
}

#[tokio::test]
async fn event_cursor_ahead_of_restarted_host_returns_without_long_polling() {
    for populated in [false, true] {
        let host = host();
        if populated {
            host.events().record(
                myrig_protocol::events::EventKind::TestShutdown,
                "Device".into(),
            );
        }
        let request = br#"{"version":1,"operation":"wait_events","after":10}"#;
        let result = tokio::time::timeout(
            Duration::from_secs(1),
            execute_secure(
                &host,
                request,
                host.pairing_approval(),
                |_| panic!("wait must not save"),
                |_, _| async { panic!("wait must not open pairing") },
            ),
        )
        .await;
        host.events().close(); // Release a failed waiter before the runtime joins it.
        let reply: serde_json::Value =
            serde_json::from_slice(&result.expect("reset must return immediately")).unwrap();
        assert_eq!(reply["baseline_sequence"], if populated { 1 } else { 0 });
        assert_eq!(
            reply["events"].as_array().unwrap().len(),
            usize::from(populated)
        );
        if populated {
            assert_eq!(reply["events"][0]["sequence"], 1);
        }
    }
}

#[tokio::test]
async fn close_invalidates_an_earlier_approval_queued_before_host_admission() {
    use sha2::{Digest, Sha256};
    use std::sync::Arc;
    let host = Arc::new(host());
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (resume_tx, resume_rx) = tokio::sync::oneshot::channel();
    let opening_host = host.clone();
    let opening = tokio::spawn(async move {
        let request = br#"{"version":1,"operation":"open_pairing","host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","name":"Device"}"#;
        execute_secure(
            &opening_host,
            request,
            opening_host.pairing_approval(),
            |_| panic!("open must not save"),
            |name, approval| {
                let target = opening_host.clone();
                async move {
                    entered_tx.send(()).unwrap();
                    resume_rx.await.unwrap();
                    target
                        .open_secure_pairing(
                            approval,
                            &name,
                            "12345678",
                            [7; 32],
                            b"cert",
                            &[1; 32],
                            uuid::Uuid::from_u128(3),
                            Arc::new(|_| Ok(())),
                        )
                        .await
                        .map_err(|_| "unavailable")?;
                    Ok(zeroize::Zeroizing::new("12345678".to_owned()))
                }
            },
        )
        .await
    });
    entered_rx.await.unwrap();
    let close = br#"{"version":1,"operation":"close_pairing","host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6"}"#;
    assert_eq!(reply(&host, close)["ok"], true);
    resume_tx.send(()).unwrap();
    let opened = opening.await.unwrap();
    assert!(
        host.secure_pairing_info(Sha256::digest(b"cert").into())
            .is_err(),
        "an earlier request must not restore a code after close acknowledged"
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&opened).unwrap()["error"],
        "unavailable"
    );
}

#[tokio::test]
async fn wait_from_another_service_process_returns_every_event() {
    // ! The cursor (3) is below the newest sequence (6), so only the epoch tells the host the
    // ! caller last heard from another process. Without it, events 1-3 are left out.
    let host = host();
    for index in 0..6 {
        host.events().record(
            myrig_protocol::events::EventKind::TestShutdown,
            format!("Device {index}"),
        );
    }
    let request =
        br#"{"version":1,"operation":"wait_events","after":3,"epoch":"an-older-process"}"#;
    let response = execute_secure(
        &host,
        request,
        host.pairing_approval(),
        |_| panic!("wait must not save"),
        |_, _| async { panic!("wait must not open pairing") },
    )
    .await;
    let reply: serde_json::Value = serde_json::from_slice(&response).unwrap();
    assert_eq!(reply["events"].as_array().unwrap().len(), 6);
}

#[test]
fn event_wait_capacity_classification_requires_the_strict_v1_request() {
    assert!(super::super::utils::is_event_wait_request(
        br#"{"version":1,"operation":"wait_events","after":0}"#
    ));
    for request in [
        br#"{"version":2,"operation":"wait_events","after":0}"#.as_slice(),
        br#"{"version":1,"operation":"wait_events","after":-1}"#,
        br#"{"version":1,"operation":"wait_events","after":0,"extra":1}"#,
        br#"{"version":1,"operation":"wait_events","after":0,"after":0}"#,
        br#"{"version":1,"operation":"status"}"#,
        br#"{"version":1,"operation":"wait_events"}"#,
        b"not json",
    ] {
        assert!(!super::super::utils::is_event_wait_request(request));
    }
    let oversized = format!(
        "{}{}",
        " ".repeat(1024),
        r#"{"version":1,"operation":"wait_events","after":0}"#
    );
    assert!(!super::super::utils::is_event_wait_request(
        oversized.as_bytes()
    ));
}

#[tokio::test]
async fn wait_events_reports_a_closed_event_log() {
    let host = host();
    host.events().close();
    let reply = execute_secure(
        &host,
        br#"{"version":1,"operation":"wait_events","after":0}"#,
        host.pairing_approval(),
        |_| panic!("wait must not save"),
        |_, _| async { panic!("wait must not open pairing") },
    )
    .await;
    let reply: serde_json::Value = serde_json::from_slice(&reply).unwrap();
    assert_eq!(reply["closed"], true);
    let epoch = reply["epoch"].as_str().unwrap();
    assert_eq!(epoch.len(), 32);
    assert!(
        epoch
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    );
}
