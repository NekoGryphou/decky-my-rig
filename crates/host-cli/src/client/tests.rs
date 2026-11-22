use crate::{command::Command, shared::CliError};
use serde_json::{Value, json};
use std::collections::VecDeque;

const HOST: &str = "03c85317-9d93-4fb3-a04b-91a0e16668c6";
const DEVICE: &str = "9fbf7536-c952-45a2-ac90-2c15076715fe";

fn status(test_mode: bool) -> Value {
    json!({"version":1,"ok":true,"host_id":HOST,"port":47991,"test_mode":test_mode,"service_version":"0.1.0","listener_state":"listening"})
}

fn success(fields: Value) -> Value {
    let mut reply = fields;
    reply["version"] = json!(1);
    reply["ok"] = json!(true);
    reply["host_id"] = json!(HOST);
    reply
}

/// Runs `command` against a service answering with `replies` in order.
async fn run(command: Command, replies: Vec<Vec<u8>>) -> (Result<String, CliError>, Vec<Value>) {
    let mut replies = VecDeque::from(replies);
    let mut requests = Vec::new();
    let result = super::execute_command(&command, |bytes| {
        requests.push(serde_json::from_slice::<Value>(&bytes).unwrap());
        std::future::ready(replies.pop_front().ok_or(CliError::Unavailable))
    })
    .await;
    (result, requests)
}

fn bytes(values: &[Value]) -> Vec<Vec<u8>> {
    values
        .iter()
        .map(|value| value.to_string().into_bytes())
        .collect()
}

#[test]
fn pipe_access_denial_gets_authorization_guidance() {
    let denied = std::io::Error::from(std::io::ErrorKind::PermissionDenied);
    assert_eq!(super::connection_error(denied), CliError::Unauthorized);
    let missing = std::io::Error::from(std::io::ErrorKind::NotFound);
    assert_eq!(super::connection_error(missing), CliError::Unavailable);
}

#[tokio::test]
async fn status_prints_service_port_listener_and_test_mode() {
    let (result, requests) = run(Command::Status, bytes(&[status(true)])).await;
    assert_eq!(
        result.unwrap(),
        "Service: running\nPort: 47991\nPort status: listening\nTest mode: on"
    );
    assert_eq!(requests, vec![json!({"version":1,"operation":"status"})]);
}

#[tokio::test]
async fn list_output_starts_with_the_device_id() {
    let devices = success(json!({"paired_devices":[{"device_id":DEVICE,"name":"My device"}]}));
    let (result, _) = run(Command::List, bytes(&[devices])).await;
    assert_eq!(result.unwrap(), format!("{DEVICE}  My device"));
    let (result, _) = run(
        Command::List,
        bytes(&[success(json!({"paired_devices":[]}))]),
    )
    .await;
    assert_eq!(result.unwrap(), "No paired devices.");
}

#[tokio::test]
async fn mutations_read_status_then_send_its_host_identity() {
    for (command, reply, expected, output) in [
        (
            Command::SetPort(47992),
            json!({"port":47992}),
            json!({"version":1,"operation":"set_port","host_id":HOST,"port":47992}),
            "Port: 47992",
        ),
        (
            Command::SetTestMode(true),
            json!({"test_mode":true}),
            json!({"version":1,"operation":"set_test_mode","host_id":HOST,"enabled":true}),
            "Test mode: on",
        ),
        (
            Command::Pair("My device".into()),
            json!({"code":"12345678","expires_in_seconds":120}),
            json!({"version":1,"operation":"open_pairing","host_id":HOST,"name":"My device"}),
            "Pairing code: 12345678\nExpires in: 120 s",
        ),
    ] {
        let (result, requests) = run(command, bytes(&[status(false), success(reply)])).await;
        assert_eq!(result.unwrap(), output);
        assert_eq!(
            requests,
            vec![json!({"version":1,"operation":"status"}), expected]
        );
    }
}

#[tokio::test]
async fn remove_reads_the_name_before_removal() {
    let replies = bytes(&[
        status(false),
        success(json!({"paired_devices":[{"device_id":DEVICE,"name":"My device"}]})),
        success(json!({"removed":true})),
    ]);
    let (result, requests) = run(Command::Remove(DEVICE.into()), replies).await;
    assert_eq!(result.unwrap(), "Removed: My device");
    assert_eq!(
        requests,
        vec![
            json!({"version":1,"operation":"status"}),
            json!({"version":1,"operation":"list"}),
            json!({"version":1,"operation":"remove","host_id":HOST,"device_id":DEVICE})
        ]
    );
}

#[tokio::test]
async fn an_unknown_device_is_reported_without_a_mutation() {
    for command in [
        Command::Remove(DEVICE.into()),
        Command::Rename(DEVICE.into(), "Living room device".into()),
    ] {
        let replies = bytes(&[status(false), success(json!({"paired_devices":[]}))]);
        let (result, requests) = run(command, replies).await;
        assert_eq!(result, Err(CliError::NotPaired));
        assert_eq!(requests.len(), 2);
    }
}

#[tokio::test]
async fn rename_reports_the_new_name() {
    let replies = bytes(&[
        status(false),
        success(json!({"paired_devices":[{"device_id":DEVICE,"name":"My device"}]})),
        success(json!({"renamed":true})),
    ]);
    let (result, _) = run(
        Command::Rename(DEVICE.into(), "Living room device".into()),
        replies,
    )
    .await;
    assert_eq!(result.unwrap(), "Renamed: Living room device");
}

#[tokio::test]
async fn host_errors_are_reported_by_code() {
    for (error, expected) in [
        ("unauthorized", CliError::Unauthorized),
        ("unavailable", CliError::Host("unavailable")),
        ("storage_failed", CliError::Host("storage_failed")),
        ("made_up", CliError::Unavailable),
    ] {
        let reply = json!({"version":1,"ok":false,"error":error});
        let (result, requests) = run(Command::SetTestMode(true), bytes(&[reply])).await;
        assert_eq!(result, Err(expected));
        assert_eq!(
            requests.len(),
            1,
            "a refused status must stop before the mutation"
        );
    }
}

#[tokio::test]
async fn duplicate_extra_or_unconfirmed_reply_fields_are_refused() {
    for reply in [
        br#"{"version":2,"version":1,"ok":true,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","port":47992}"#.to_vec(),
        br#"{"version":1,"ok":true,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","port":47992,"token":"private"}"#.to_vec(),
        br#"{"version":1,"ok":true,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","port":47993}"#.to_vec(),
        br#"{"version":1,"ok":true,"host_id":"9fbf7536-c952-45a2-ac90-2c15076715fe","port":47992}"#.to_vec(),
    ] {
        let replies = vec![status(false).to_string().into_bytes(), reply];
        let (result, _) = run(Command::SetPort(47992), replies).await;
        assert_eq!(result, Err(CliError::Unavailable));
    }
}

#[tokio::test]
async fn a_lost_mutation_reply_is_not_retried() {
    let (result, requests) = run(Command::SetTestMode(true), bytes(&[status(false)])).await;
    assert_eq!(result, Err(CliError::Unavailable));
    assert_eq!(requests.len(), 2);
}
