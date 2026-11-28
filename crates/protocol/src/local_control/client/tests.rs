use super::{ControlRequest, OpenPairingReply, ReplyError, WaitReply, encode, parse};

const HOST: &str = "03c85317-9d93-4fb3-a04b-91a0e16668c6";

#[test]
fn unauthorized_reply_preserves_the_authorization_failure() {
    let reply = br#"{"version":1,"ok":false,"error":"unauthorized"}"#;
    assert_eq!(
        parse::<WaitReply>(reply).err(),
        Some(ReplyError::Unauthorized)
    );
}

#[test]
fn malformed_errors_do_not_claim_authorization_failure() {
    for bytes in [
        br#"{"version":2,"ok":false,"error":"unauthorized"}"#.as_slice(),
        br#"{"version":1,"ok":true,"error":"unauthorized"}"#.as_slice(),
        br#"{"version":1,"ok":false,"error":"unauthorized","extra":1}"#.as_slice(),
        br#"{"version":1,"version":1,"ok":false,"error":"unauthorized"}"#.as_slice(),
        br#"{"version":1,"ok":false,"error":"other"}"#.as_slice(),
    ] {
        assert_eq!(parse::<WaitReply>(bytes).err(), Some(ReplyError::Invalid));
    }
}

#[test]
fn known_service_errors_keep_their_code() {
    let reply = br#"{"version":1,"ok":false,"error":"storage_failed"}"#;
    assert_eq!(
        parse::<WaitReply>(reply).err(),
        Some(ReplyError::Host("storage_failed"))
    );
}

#[test]
fn open_pairing_request_names_the_host_and_device() {
    let request = encode(&ControlRequest::OpenPairing {
        host_id: HOST,
        name: "steamdeck",
    });
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&request).unwrap(),
        serde_json::json!({"version":1,"operation":"open_pairing","host_id":HOST,"name":"steamdeck"})
    );
}

#[test]
fn open_pairing_reply_yields_only_an_eight_digit_code() {
    let ok = br#"{"version":1,"ok":true,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","code":"48210937","expires_in_seconds":120}"#;
    assert_eq!(parse::<OpenPairingReply>(ok).unwrap().code, "48210937");
    for bytes in [
        br#"{"version":1,"ok":true,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","code":"4821093","expires_in_seconds":120}"#.as_slice(),
        br#"{"version":1,"ok":true,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","code":"4821093x","expires_in_seconds":120}"#,
        br#"{"version":1,"ok":true,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","code":"48210937","expires_in_seconds":0}"#,
        br#"{"version":1,"ok":true,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","code":"48210937","expires_in_seconds":121}"#,
        br#"{"version":1,"ok":true,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","code":"48210937","expires_in_seconds":120,"extra":1}"#,
        br#"{"version":1,"ok":false,"error":"unavailable"}"#,
        b"",
    ] {
        assert!(parse::<OpenPairingReply>(bytes).is_err());
    }
}

#[test]
fn event_batch_keeps_the_host_id() {
    let reply = br#"{"version":1,"ok":true,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","epoch":"test-epoch","baseline_sequence":0,"closed":false,"events":[{"sequence":1,"kind":"pairing_request","device_name":"steamdeck","time_unix":1}]}"#;
    let batch = parse::<WaitReply>(reply).unwrap();
    assert_eq!(batch.host_id, HOST);
    assert_eq!(batch.baseline_sequence, 0);
    assert_eq!(
        batch.events[0].kind,
        crate::events::EventKind::PairingRequest
    );
}

#[test]
fn unordered_events_are_refused() {
    let reply = br#"{"version":1,"ok":true,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","epoch":"test-epoch","baseline_sequence":0,"closed":false,"events":[{"sequence":2,"kind":"pairing_request","device_name":"a","time_unix":1},{"sequence":2,"kind":"pairing_request","device_name":"b","time_unix":1}]}"#;
    assert_eq!(parse::<WaitReply>(reply).err(), Some(ReplyError::Invalid));
}

#[tokio::test]
async fn exchanges_one_bounded_request_and_reply() {
    let (mut client, mut server) = tokio::io::duplex(1024);
    let service = tokio::spawn(async move {
        let request = crate::local_control::framing::read_frame(&mut server, 1024)
            .await
            .unwrap();
        assert_eq!(request, br#"{"version":1,"operation":"status"}"#);
        crate::local_control::framing::write_frame(
            &mut server,
            br#"{"version":1,"ok":false,"error":"unavailable"}"#,
            1024,
        )
        .await
        .unwrap();
    });
    let reply = super::exchange(&mut client, br#"{"version":1,"operation":"status"}"#)
        .await
        .unwrap();
    assert_eq!(reply, br#"{"version":1,"ok":false,"error":"unavailable"}"#);
    service.await.unwrap();
}

#[tokio::test]
async fn rejects_oversized_request_before_writing() {
    let (mut client, mut server) = tokio::io::duplex(16384);
    assert!(super::exchange(&mut client, &[b'x'; 1025]).await.is_err());
    let mut prefix = [0; 1];
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_millis(10),
            tokio::io::AsyncReadExt::read_exact(&mut server, &mut prefix)
        )
        .await
        .is_err()
    );
}
