use super::super::fixtures::empty_registry_host;
use super::utils::parse_device_name;
use crate::host::{PairRequestError, REQUEST_INTERVAL};
use myrig_protocol::events::EventKind;
use sha2::{Digest, Sha256};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[test]
fn request_records_one_pairing_event_and_spaces_requests() {
    let host = empty_registry_host();
    let now = Instant::now();
    assert_eq!(host.request_pairing("steamdeck", now), Ok(()));
    let events = host.events().events_after(0);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::PairingRequest);
    assert_eq!(events[0].device_name, "steamdeck");
    assert_eq!(
        host.request_pairing(
            "steamdeck",
            now + REQUEST_INTERVAL - Duration::from_millis(1)
        ),
        Err(PairRequestError::Busy)
    );
    assert_eq!(
        host.request_pairing("steamdeck", now + REQUEST_INTERVAL),
        Ok(())
    );
    assert_eq!(host.events().latest_sequence(), 2);
}

#[test]
fn invalid_names_are_refused_without_an_event() {
    let host = empty_registry_host();
    for name in ["", "Device\nName", &"x".repeat(81)] {
        assert_eq!(
            host.request_pairing(name, Instant::now()),
            Err(PairRequestError::Invalid)
        );
    }
    assert_eq!(host.events().latest_sequence(), 0);
}

async fn open_code(host: &Arc<crate::host::Host>) {
    host.open_secure_pairing(
        host.pairing_approval(),
        "steamdeck",
        "12345678",
        [7; 32],
        b"cert",
        [1; 32],
        uuid::Uuid::from_u128(1),
        Arc::new(|_| Ok(())),
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn an_unused_code_is_replaced_but_a_tried_one_is_kept() {
    let host = empty_registry_host();
    let binding: [u8; 32] = Sha256::digest(b"cert").into();
    let now = Instant::now();
    open_code(&host).await;
    // - The notification was dismissed, so the next request shows a new code
    //   right away.
    assert_eq!(host.request_pairing("steamdeck", now), Ok(()));
    open_code(&host).await;
    // - Once someone has tried the code, it stays until it expires.
    let _ = host.secure_pairing_start(&[0; 96], binding).await;
    assert_eq!(
        host.request_pairing("steamdeck", now + REQUEST_INTERVAL),
        Err(PairRequestError::Busy)
    );
    // - Using up every attempt doesn't free it either.
    for _ in 0..6 {
        let _ = host.secure_pairing_start(&[0; 96], binding).await;
    }
    assert_eq!(
        host.request_pairing("steamdeck", now + REQUEST_INTERVAL * 2),
        Err(PairRequestError::Busy)
    );
    host.close_pairing();
    // - Seven failed guesses are two codes' worth, so the wait has doubled twice.
    assert_eq!(
        host.request_pairing("steamdeck", now + REQUEST_INTERVAL * 4),
        Ok(())
    );
}

#[test]
fn unhealthy_storage_refuses_requests() {
    let host = empty_registry_host();
    host.require_storage_recovery();
    assert_eq!(
        host.request_pairing("steamdeck", Instant::now()),
        Err(PairRequestError::Unavailable)
    );
}

#[test]
fn body_is_exactly_one_device_name() {
    assert_eq!(
        parse_device_name(br#"{"device_name":"steamdeck"}"#),
        Some("steamdeck".to_owned())
    );
    for body in [
        br#"{}"#.as_slice(),
        br#"{"device_name":1}"#,
        br#"{"device_name":"a","extra":1}"#,
        br#"{"device_name":"a","device_name":"b"}"#,
        b"not json",
        b"",
    ] {
        assert_eq!(parse_device_name(body), None, "{body:?}");
    }
}

#[tokio::test]
async fn failed_codes_slow_down_network_requests_until_a_pairing_succeeds() {
    // - Nobody's at the PC, and someone on the LAN keeps asking for new codes and
    //   using every guess. Each code's worth of failures doubles the wait.
    let host = empty_registry_host();
    let binding: [u8; 32] = Sha256::digest(b"cert").into();
    let mut now = Instant::now();
    assert_eq!(host.request_pairing("steamdeck", now), Ok(()));
    let mut waits = Vec::new();
    for _ in 0..3 {
        open_code(&host).await;
        for _ in 0..myrig_pairing_crypto::session::MAX_STARTS {
            let _ = host.secure_pairing_start(&[0; 96], binding).await;
        }
        host.close_pairing();
        let mut wait = REQUEST_INTERVAL;
        while host.request_pairing("steamdeck", now + wait) == Err(PairRequestError::Busy) {
            wait += REQUEST_INTERVAL;
        }
        now += wait;
        waits.push(wait);
    }
    assert_eq!(
        waits,
        [
            REQUEST_INTERVAL * 2,
            REQUEST_INTERVAL * 4,
            REQUEST_INTERVAL * 8
        ]
    );
}

#[tokio::test]
async fn opening_a_code_never_replaces_one_someone_has_tried() {
    // * This applies to every opener. To start over, close the code first (Cancel
    //   in the app).
    let host = empty_registry_host();
    let binding: [u8; 32] = Sha256::digest(b"cert").into();
    open_code(&host).await;
    let _ = host.secure_pairing_start(&[0; 96], binding).await;
    let reopened = host
        .open_secure_pairing(
            host.pairing_approval(),
            "steamdeck",
            "87654321",
            [7; 32],
            b"cert",
            [2; 32],
            uuid::Uuid::from_u128(2),
            Arc::new(|_| Ok(())),
        )
        .await;
    assert_eq!(reopened, Err(crate::host::OpenPairingError::Busy));
    host.close_pairing();
    open_code(&host).await;
}
