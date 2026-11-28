use myrig_pairing_crypto::session::AttemptId;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use super::super::fixtures::empty_registry_host;
use super::fixtures::*;
use crate::host::*;

/// binding.
async fn open_code(host: &Arc<Host>) -> [u8; 32] {
    host.open_secure_pairing(
        host.pairing_approval(),
        "Device",
        "12345678",
        [7; 32],
        b"cert",
        &[1; 32],
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

#[tokio::test]
async fn a_code_whose_guesses_are_used_up_is_replaced() {
    // - Anyone on the LAN can spend a code's guesses with malformed starts. Once
    //   they are gone, the PC user's new code must replace it rather than wait out
    //   the code's lifetime.
    let host = empty_registry_host();
    let now = std::time::Instant::now();
    assert_eq!(host.request_pairing("Device", now), Ok(()));
    let used_up = open_code(&host).await;
    for _ in 0..3 {
        assert!(host.secure_pairing_start(&[0; 96], used_up).await.is_err());
    }
    // ! Three failed starts double the request interval to 20 seconds. The code lives for 120, so
    // ! waiting 30 keeps it open.
    let later = now + std::time::Duration::from_secs(30);
    assert_eq!(host.request_pairing("Device", later), Ok(()));
    let replacement = open_code(&host).await;
    assert!(host.secure_pairing_info(replacement).is_ok());
}

#[test]
fn the_protocol_states_the_code_lifetime_the_session_enforces() {
    // ? Local-control clients check expires_in_seconds against the protocol's copy of the value, so
    // ? they don't need the pairing crypto.
    assert_eq!(
        myrig_protocol::local_control::PAIRING_CODE_LIFETIME,
        myrig_pairing_crypto::session::LIFETIME
    );
}
