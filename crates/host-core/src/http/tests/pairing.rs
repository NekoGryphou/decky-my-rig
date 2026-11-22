use myrig_pairing_crypto::session::AttemptId;
use myrig_protocol::validation::{decode_hex, lower_hex};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

use myrig_protocol::checked_json::CheckedJson;

use super::super::{fixtures::empty_registry_host, server::serve_secure_tls};
use super::fixtures::*;
use crate::host::*;

#[test]
fn shared_pairing_fixtures_match_strict_hex_and_shapes() {
    let info: CheckedJson = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/pairing/info-reply.json"
    ))
    .unwrap();
    let start_request: CheckedJson = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/pairing/start-request.json"
    ))
    .unwrap();
    let start_reply: CheckedJson = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/pairing/start-reply.json"
    ))
    .unwrap();
    let finish_request: CheckedJson = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/pairing/finish-request.json"
    ))
    .unwrap();
    let finish_reply: CheckedJson = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/pairing/finish-reply.json"
    ))
    .unwrap();
    assert!(decode_hex::<32>(info.0["nonce"].as_str().unwrap()).is_some());
    assert!(decode_hex::<96>(start_request.0["request"].as_str().unwrap()).is_some());
    assert!(decode_hex::<32>(start_reply.0["attempt_id"].as_str().unwrap()).is_some());
    assert!(decode_hex::<32>(start_reply.0["session_nonce"].as_str().unwrap()).is_some());
    assert!(decode_hex::<320>(start_reply.0["response"].as_str().unwrap()).is_some());
    assert!(decode_hex::<32>(finish_request.0["attempt_id"].as_str().unwrap()).is_some());
    assert!(decode_hex::<64>(finish_request.0["confirmation"].as_str().unwrap()).is_some());
    assert!(decode_hex::<32>(finish_reply.0["token"].as_str().unwrap()).is_some());
    assert!(
        decode_hex::<32>(
            serde_json::from_str::<CheckedJson>(include_str!(
                "../../../../../tests/fixtures/pairing/invalid-info-reply.json"
            ))
            .unwrap()
            .0["nonce"]
                .as_str()
                .unwrap()
        )
        .is_none()
    );
    assert!(
        decode_hex::<96>(
            serde_json::from_str::<CheckedJson>(include_str!(
                "../../../../../tests/fixtures/pairing/invalid-start-request.json"
            ))
            .unwrap()
            .0["request"]
                .as_str()
                .unwrap()
        )
        .is_none()
    );
}

#[test]
fn shared_invalid_lan_request_is_rejected() {
    let invalid =
        include_str!("../../../../../tests/fixtures/lan-api/invalid-shutdown-request.json");
    assert!(serde_json::from_str::<CheckedJson>(invalid).is_err());
}

#[tokio::test]
async fn secure_pairing_authentication_persistence_revocation_and_failures() {
    let host = empty_registry_host();
    let saved = Arc::new(Mutex::new(Vec::new()));
    for index in 1..=2u8 {
        let capture = saved.clone();
        let current = host.clone();
        let token = format!("{index:02x}").repeat(32);
        let expected = token.clone();
        host.open_secure_pairing(
            host.pairing_approval(),
            "Device",
            "12345678",
            [7; 32],
            b"cert",
            [index; 32],
            device(index.into()),
            Arc::new(move |bytes| {
                assert!(current.authenticated_device(&expected).is_none());
                *capture.lock().unwrap() = bytes.to_vec();
                Ok(())
            }),
        )
        .await
        .unwrap();
        let (attempt, wrong) = secure_confirmation(&host, b"87654321", b"cert").await;
        assert!(wrong.is_none());
        assert!(
            host.secure_pairing_finish(attempt, &[0; 64], Sha256::digest(b"cert").into(), None)
                .await
                .is_err()
        );
        let (attempt, confirmation) = secure_confirmation(&host, b"12345678", b"cert").await;
        assert!(
            host.secure_pairing_finish(attempt, &confirmation.unwrap(), [0; 32], None)
                .await
                .is_err()
        );
        assert!(
            host.secure_pairing_finish(
                AttemptId::from_bytes([0; 32]),
                &[0; 64],
                Sha256::digest(b"cert").into(),
                None
            )
            .await
            .is_err()
        );
        assert_eq!(
            host.secure_pairing_finish(
                attempt,
                &confirmation.unwrap(),
                Sha256::digest(b"cert").into(),
                None
            )
            .await,
            Ok(zeroize::Zeroizing::new(token.clone()))
        );
        assert!(
            host.secure_pairing_finish(attempt, &[0; 64], Sha256::digest(b"cert").into(), None)
                .await
                .is_err()
        );
        assert!(host.authenticated_device(&token).is_some());
    }
    host.remove_device_checked(device(1), |_| Ok(())).unwrap();
    assert!(host.authenticated_device(&"01".repeat(32)).is_none());
    assert!(host.authenticated_device(&"02".repeat(32)).is_some());
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let entered_tx = Mutex::new(Some(entered_tx));
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let release_rx = Mutex::new(release_rx);
    host.open_secure_pairing(
        host.pairing_approval(),
        "Device",
        "12345678",
        [7; 32],
        b"cert",
        [5; 32],
        device(5),
        Arc::new(move |_| {
            entered_tx.lock().unwrap().take().unwrap().send(()).unwrap();
            release_rx.lock().unwrap().recv().unwrap();
            Ok(())
        }),
    )
    .await
    .unwrap();
    let (attempt, confirmation) = secure_confirmation(&host, b"12345678", b"cert").await;
    let finishing = host.clone();
    let waiter = tokio::spawn(async move {
        finishing
            .secure_pairing_finish(
                attempt,
                &confirmation.unwrap(),
                Sha256::digest(b"cert").into(),
                None,
            )
            .await
    });
    entered_rx.await.unwrap();
    waiter.abort();
    host.close_pairing();
    assert!(host.pairing.lock().unwrap().busy);
    assert!(host.authenticated_device(&"05".repeat(32)).is_none());
    assert_eq!(
        host.open_secure_pairing(
            host.pairing_approval(),
            "Device",
            "12345678",
            [7; 32],
            b"cert",
            [6; 32],
            device(6),
            Arc::new(|_| Ok(()))
        )
        .await,
        Err(OpenPairingError::Busy)
    );
    release_tx.send(()).unwrap();
    host.wait_pairing_idle().await;
    assert!(host.authenticated_device(&"05".repeat(32)).is_some());
    for (index, error) in [(3, PersistError::Unchanged), (4, PersistError::Conflict)] {
        host.open_secure_pairing(
            host.pairing_approval(),
            "Device",
            "12345678",
            [7; 32],
            b"cert",
            [index; 32],
            device(index.into()),
            Arc::new(move |_| {
                Err(match error {
                    PersistError::Unchanged => PersistError::Unchanged,
                    _ => PersistError::Conflict,
                })
            }),
        )
        .await
        .unwrap();
        let (attempt, confirmation) = secure_confirmation(&host, b"12345678", b"cert").await;
        assert!(
            host.secure_pairing_finish(
                attempt,
                &confirmation.unwrap(),
                Sha256::digest(b"cert").into(),
                None
            )
            .await
            .is_err()
        );
        assert!(
            host.authenticated_device(&format!("{index:02x}").repeat(32))
                .is_none()
        );
        assert_eq!(host.storage_healthy(), index == 3);
    }
}

#[tokio::test]
// ? The lock is held on purpose: the opener should refuse right away, before any
//   blocking work, rather than wait behind a save.
#[allow(clippy::await_holding_lock)]
async fn secure_pairing_admission_closure_and_cancelled_prepare_join() {
    let host = empty_registry_host();
    let mutation = host.registry_mutation.lock().unwrap();
    assert_eq!(
        host.open_secure_pairing(
            host.pairing_approval(),
            "Device",
            "12345678",
            [7; 32],
            b"cert",
            [2; 32],
            device(2),
            Arc::new(|_| Ok(()))
        )
        .await,
        Err(OpenPairingError::Busy)
    );
    drop(mutation);
    host.open_secure_pairing(
        host.pairing_approval(),
        "Device",
        "12345678",
        [7; 32],
        b"cert",
        [2; 32],
        device(2),
        Arc::new(|_| Ok(())),
    )
    .await
    .unwrap();
    host.close_pairing();
    host.wait_pairing_idle().await;
    assert!(
        host.secure_pairing_start(&[0; 96], Sha256::digest(b"cert").into())
            .await
            .is_err()
    );
    host.open_secure_pairing(
        host.pairing_approval(),
        "Device",
        "12345678",
        [7; 32],
        b"cert",
        [2; 32],
        device(2),
        Arc::new(|_| Ok(())),
    )
    .await
    .unwrap();
    let expires = host
        .pairing
        .lock()
        .unwrap()
        .session
        .as_ref()
        .unwrap()
        .crypto
        .expires_at();
    host.expire_pending_pairing_at(expires);
    assert!(host.pairing.lock().unwrap().session.is_none());
    host.open_secure_pairing(
        host.pairing_approval(),
        "Device",
        "12345678",
        [7; 32],
        b"cert",
        [2; 32],
        device(2),
        Arc::new(|_| Ok(())),
    )
    .await
    .unwrap();
    host.close_pairing();
    // ! Canceling never gives up the slot while a blocking worker owns it.
    host.pairing.lock().unwrap().busy = true;
    let reservation = PairingReservation(host.clone());
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let waiter = tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            let _reservation = reservation;
            entered_tx.send(()).unwrap();
            release_rx.recv().unwrap();
        })
        .await
        .unwrap();
    });
    entered_rx.await.unwrap();
    waiter.abort();
    host.close_pairing();
    assert!(host.pairing.lock().unwrap().busy);
    assert_eq!(
        host.open_secure_pairing(
            host.pairing_approval(),
            "Device",
            "12345678",
            [7; 32],
            b"cert",
            [3; 32],
            device(3),
            Arc::new(|_| Ok(()))
        )
        .await,
        Err(OpenPairingError::Busy)
    );
    release_tx.send(()).unwrap();
    host.wait_pairing_idle().await;
    assert!(!host.pairing.lock().unwrap().busy);
}

#[tokio::test]
async fn secure_http_stop_joins_claimed_save_and_closes_unclaimed_code() {
    let host = empty_registry_host();
    let generated = rcgen::generate_simple_self_signed(vec!["myrig.local".to_owned()]).unwrap();
    let certificate = generated.cert.der().to_vec();
    let identity = crate::tls::FixedIdentity::from_der(
        vec![certificate.clone()],
        generated.signing_key.serialize_der(),
    )
    .unwrap();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let entered = Mutex::new(Some(entered_tx));
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let release = Mutex::new(release_rx);
    host.open_secure_pairing(
        host.pairing_approval(),
        "Device",
        "12345678",
        [7; 32],
        &certificate,
        [1; 32],
        device(1),
        Arc::new(move |_| {
            entered.lock().unwrap().take().unwrap().send(()).unwrap();
            release.lock().unwrap().recv().unwrap();
            Ok(())
        }),
    )
    .await
    .unwrap();
    let (attempt, confirmation) = secure_confirmation(&host, b"12345678", &certificate).await;
    let binding = Sha256::digest(&certificate).into();
    let current = host.clone();
    let exchange = tokio::spawn(async move {
        current
            .secure_pairing_finish(attempt, &confirmation.unwrap(), binding, None)
            .await
    });
    entered_rx.await.unwrap();
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let (stopped_tx, stopped_rx) = tokio::sync::oneshot::channel();
    let (done_tx, mut done_rx) = tokio::sync::oneshot::channel();
    let current = host.clone();
    let serving_identity = identity.clone();
    let server = tokio::spawn(async move {
        let result = serve_secure_tls(
            listener,
            current,
            serving_identity,
            async {
                let _ = stop_rx.await;
                stopped_tx.send(()).unwrap();
            },
            |_| {},
        )
        .await;
        done_tx.send(result).unwrap();
    });
    stop_tx.send(()).unwrap();
    stopped_rx.await.unwrap();
    // - Let every other task run: a server that doesn't wait would be done by
    //   now.
    for _ in 0..100 {
        tokio::task::yield_now().await;
    }
    assert!(done_rx.try_recv().is_err());
    release_tx.send(()).unwrap();
    assert_eq!(
        exchange.await.unwrap(),
        Ok(zeroize::Zeroizing::new("01".repeat(32)))
    );
    done_rx.await.unwrap().unwrap();
    server.await.unwrap();
    assert!(!host.pairing.lock().unwrap().busy);
    assert!(host.authenticated_device(&"01".repeat(32)).is_some());
    host.open_secure_pairing(
        host.pairing_approval(),
        "Other",
        "87654321",
        [7; 32],
        &certificate,
        [2; 32],
        device(2),
        Arc::new(|_| panic!("unclaimed code must not persist on stop")),
    )
    .await
    .unwrap();
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .unwrap();
    serve_secure_tls(
        listener,
        host.clone(),
        identity,
        std::future::ready(()),
        |_| {},
    )
    .await
    .unwrap();
    assert!(host.secure_pairing_start(&[0; 96], binding).await.is_err());
    assert!(host.authenticated_device(&"02".repeat(32)).is_none());
}

#[test]
fn bootstrap_hex_is_exact_lowercase_and_bounded() {
    assert_eq!(decode_hex::<2>("00ff"), Some([0, 255]));
    for malformed in ["", "00f", "00fff", "00FF", "00fg", "éff", " ff"] {
        assert!(decode_hex::<2>(malformed).is_none());
    }
    assert_eq!(lower_hex(&[0, 255]), "00ff");
}

#[tokio::test]
async fn unused_pairing_code_is_replaced_and_local_close_invalidates_the_replacement() {
    let host = empty_registry_host();
    host.open_secure_pairing(
        host.pairing_approval(),
        "Device",
        "12345678",
        [7; 32],
        b"cert",
        [1; 32],
        device(1),
        Arc::new(|_| panic!("replaced approval must not save")),
    )
    .await
    .unwrap();
    host.open_secure_pairing(
        host.pairing_approval(),
        "New device",
        "87654321",
        [7; 32],
        b"cert",
        [2; 32],
        device(2),
        Arc::new(|_| Ok(())),
    )
    .await
    .unwrap();
    let (old_attempt, old_confirmation) = secure_confirmation(&host, b"12345678", b"cert").await;
    assert!(old_confirmation.is_none());
    assert!(
        host.secure_pairing_finish(old_attempt, &[0; 64], Sha256::digest(b"cert").into(), None)
            .await
            .is_err()
    );
    let (new_attempt, confirmation) = secure_confirmation(&host, b"87654321", b"cert").await;
    assert!(confirmation.is_some());
    let request =
        json!({"version":1,"operation":"close_pairing","host_id":host.host_id().to_string()})
            .to_string();
    let result = crate::local_control::execute_secure(
        &host,
        request.as_bytes(),
        host.pairing_approval(),
        |_| panic!("close must not persist"),
        |_, _| async { panic!("close must not open") },
    )
    .await;
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&result).unwrap()["ok"],
        true
    );
    assert!(
        host.secure_pairing_info(Sha256::digest(b"cert").into())
            .is_err()
    );
    assert!(
        host.secure_pairing_finish(
            new_attempt,
            &confirmation.unwrap(),
            Sha256::digest(b"cert").into(),
            None
        )
        .await
        .is_err()
    );
    assert!(host.paired_devices().unwrap().is_empty());
}

#[tokio::test]
async fn poisoned_pairing_state_is_unavailable_not_busy() {
    let host = empty_registry_host();
    let poisoner = host.clone();
    let _ = std::thread::spawn(move || {
        let _state = poisoner.pairing.lock().unwrap();
        panic!("poison the pairing state");
    })
    .join();
    assert_eq!(
        host.open_secure_pairing(
            host.pairing_approval(),
            "Device",
            "12345678",
            [7; 32],
            b"cert",
            [2; 32],
            device(2),
            Arc::new(|_| Ok(())),
        )
        .await,
        Err(OpenPairingError::Unavailable)
    );
}

#[tokio::test]
async fn re_pairing_revokes_only_the_credential_it_replaces() {
    let host = empty_registry_host();
    let first = pair_device(&host, 1, None).await.unwrap();
    let other = pair_device(&host, 2, None).await.unwrap();
    let replacement = pair_device(&host, 3, Some(&first)).await.unwrap();
    assert!(host.authenticated_device(&first).is_none());
    assert!(host.authenticated_device(&other).is_some());
    assert!(host.authenticated_device(&replacement).is_some());
    // - A token that matches no device revokes nothing, and pairing still works.
    let unknown = "0f".repeat(32);
    let fourth = pair_device(&host, 4, Some(&unknown)).await.unwrap();
    assert!(host.authenticated_device(&other).is_some());
    assert!(host.authenticated_device(&fourth).is_some());
    assert_eq!(host.paired_devices().unwrap().len(), 3);
}

/// Opens code 12345678 for certificate `cert` and returns that certificate's
/// binding.
async fn open_code(host: &Arc<Host>) -> [u8; 32] {
    host.open_secure_pairing(
        host.pairing_approval(),
        "Device",
        "12345678",
        [7; 32],
        b"cert",
        [1; 32],
        device(1),
        Arc::new(|_| Ok(())),
    )
    .await
    .unwrap();
    Sha256::digest(b"cert").into()
}

#[tokio::test]
async fn a_successful_pairing_resets_the_guess_budget() {
    // - A user who mistyped the code isn't slowed down once they pair.
    let host = empty_registry_host();
    let now = std::time::Instant::now();
    assert_eq!(host.request_pairing("Device", now), Ok(()));
    let binding = open_code(&host).await;
    for _ in 0..2 {
        let (attempt, wrong) = secure_confirmation(&host, b"87654321", b"cert").await;
        assert!(wrong.is_none());
        assert!(
            host.secure_pairing_finish(attempt, &[0; 64], binding, None)
                .await
                .is_err()
        );
    }
    let (attempt, confirmation) = secure_confirmation(&host, b"12345678", b"cert").await;
    assert!(
        host.secure_pairing_finish(attempt, &confirmation.unwrap(), binding, None)
            .await
            .is_ok()
    );
    assert_eq!(
        host.request_pairing("Device", now + REQUEST_INTERVAL),
        Ok(())
    );
}

#[tokio::test]
async fn a_held_code_is_busy_and_an_unknown_attempt_does_not_hold_it() {
    let host = empty_registry_host();
    let binding = open_code(&host).await;
    // - A finish that names no open attempt is refused before the code is
    //   claimed.
    assert!(matches!(
        host.claim_secure(binding, Some(AttemptId::from_bytes([9; 32]))),
        Err(PairingRefusal::Unavailable)
    ));
    let held = host.claim_secure(binding, None).ok().unwrap();
    assert_eq!(host.secure_pairing_info(binding), Err(PairingRefusal::Busy));
    assert!(matches!(
        host.secure_pairing_start(&[0; 96], binding).await,
        Err(PairingRefusal::Busy)
    ));
    let (session, reservation) = held;
    reservation.0.restore_secure(session).unwrap();
    drop(reservation);
    assert_eq!(host.secure_pairing_info(binding), Ok([7; 32]));
}

#[test]
fn the_protocol_states_the_code_lifetime_the_session_enforces() {
    // ? Local-control clients check expires_in_seconds against the protocol's
    //   copy of the value, so they don't need the pairing crypto.
    assert_eq!(
        myrig_protocol::local_control::PAIRING_CODE_LIFETIME,
        myrig_pairing_crypto::session::LIFETIME
    );
}
