use super::super::*;
use super::fixtures::*;
use crate::host::PersistError;
use crate::paired_devices::RegistryConfig;
use myrig_protocol::local_control::REQUEST_LIMIT;
use serde_json::json;

#[test]
fn list_and_removal_are_bounded_and_secret_free() {
    let host = host();
    let before = reply(&host, br#"{"version":1,"operation":"list"}"#);
    assert_eq!(before["paired_devices"].as_array().unwrap().len(), 2);
    assert!(!before.to_string().contains("token"));
    let mut saved = Vec::new();
    let result: serde_json::Value = serde_json::from_slice(&execute(&host, &remove(), |bytes| {
        saved.extend_from_slice(bytes);
        Ok(())
    }))
    .unwrap();
    assert_eq!(result["removed"], true);
    assert_eq!(host.paired_devices().unwrap().len(), 1);
    assert_eq!(
        RegistryConfig::from_json(&saved)
            .unwrap()
            .public_summary()
            .paired_devices[0]
            .name,
        "Other device"
    );
    assert_eq!(reply(&host, &remove())["removed"], false);
}

#[test]
fn no_identity_is_trusted_from_json() {
    let host = host();
    for bytes in [
        br#"{"version":1,"operation":"list","authorized":true}"#.as_slice(),
        br#"{"version":1,"operation":"list","caller_sid":"SYSTEM"}"#,
        br#"{"version":1,"operation":"list","token":"private"}"#,
    ] {
        assert_eq!(reply(&host, bytes)["error"], "invalid_request");
    }
}

#[test]
fn strict_schema_duplicate_keys_wrong_host_and_pairing_are_refused() {
    let host = host();
    for bytes in [
        br#"{"version":1,"operation":"list","version":1}"#.as_slice(),
        br#"{"version":1,"operation":"list","operation":"remove"}"#,
        br#"{"version":true,"operation":"list"}"#,
        br#"{"version":1.0,"operation":"list"}"#,
        br#"{"version":2,"operation":"list"}"#,
        br#"{"version":1,"operation":"open_pairing"}"#,
        br#"{"version":1,"operation":"list","path":"private"}"#,
        br#"[]"#,
        b"",
        &[b'x'; REQUEST_LIMIT + 1],
    ] {
        assert_eq!(reply(&host, bytes)["error"], "invalid_request");
    }
    let mut wrong: serde_json::Value = serde_json::from_slice(&remove()).unwrap();
    wrong["host_id"] = json!(DEVICE);
    assert_eq!(
        reply(&host, &serde_json::to_vec(&wrong).unwrap())["error"],
        "wrong_host"
    );
}

#[test]
fn persistence_failure_conflict_and_uncertain_commit_have_distinct_results() {
    for (failure, code, healthy) in [
        (PersistError::Unchanged, "storage_failed", true),
        (PersistError::Conflict, "storage_conflict", false),
        (PersistError::RecoveryRequired, "recovery_required", false),
    ] {
        let host = host();
        let response: serde_json::Value =
            serde_json::from_slice(&execute(&host, &remove(), |_| Err(failure))).unwrap();
        assert_eq!(response["error"], code);
        assert_eq!(host.storage_healthy(), healthy);
        if healthy {
            assert_eq!(host.paired_devices().unwrap().len(), 2);
        } else {
            assert_eq!(
                reply(&host, br#"{"version":1,"operation":"list"}"#)["error"],
                "recovery_required"
            );
        }
    }
}

#[test]
fn close_pairing_checks_canonical_host_and_strict_schema() {
    let host = host();
    let request = json!({"version":1,"operation":"close_pairing","host_id":HOST}).to_string();
    assert_eq!(
        reply(&host, request.as_bytes()),
        json!({"version":1,"ok":true,"host_id":HOST})
    );
    assert_eq!(
        reply(&host, request.replace(HOST, DEVICE).as_bytes())["error"],
        "wrong_host"
    );
    for invalid in [
        request.replace(HOST, &HOST.to_uppercase()),
        request.replace("\"version\":1", "\"version\":2"),
        request.replace("\"version\":1", "\"version\":1,\"version\":1"),
        request.replace("}", ",\"name\":\"Device\"}"),
    ] {
        assert_eq!(reply(&host, invalid.as_bytes())["error"], "invalid_request");
    }
}

#[test]
fn rename_saves_then_lists_the_new_name() {
    let host = host();
    let mut saved = Vec::new();
    let result: serde_json::Value =
        serde_json::from_slice(&execute(&host, &rename("Living room device"), |bytes| {
            saved.extend_from_slice(bytes);
            Ok(())
        }))
        .unwrap();
    assert_eq!(result["renamed"], true);
    assert_eq!(result["host_id"], HOST);
    let names = |devices: Vec<crate::paired_devices::PublicPairedDevice>| {
        devices
            .into_iter()
            .map(|device| device.name)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names(host.paired_devices().unwrap()),
        ["Living room device", "Other device"]
    );
    assert_eq!(
        names(
            RegistryConfig::from_json(&saved)
                .unwrap()
                .public_summary()
                .paired_devices
        ),
        ["Living room device", "Other device"]
    );
    // - Same name: nothing to save.
    assert_eq!(
        reply(&host, &rename("Living room device"))["renamed"],
        false
    );
}

#[test]
fn rename_refuses_invalid_names_unknown_devices_and_other_hosts() {
    let host = host();
    for name in ["", "Device\nName", &"x".repeat(81)] {
        assert_eq!(
            reply(&host, &rename(name))["error"],
            "invalid_request",
            "{name:?}"
        );
    }
    let unknown = json!({"version":1,"operation":"rename","host_id":HOST,
        "device_id":"33333333-3333-4333-8333-333333333333","name":"Device"});
    assert_eq!(
        reply(&host, unknown.to_string().as_bytes())["renamed"],
        false
    );
    let other = json!({"version":1,"operation":"rename",
        "host_id":"44444444-4444-4444-8444-444444444444","device_id":DEVICE,"name":"Device"});
    assert_eq!(
        reply(&host, other.to_string().as_bytes())["error"],
        "wrong_host"
    );
}
