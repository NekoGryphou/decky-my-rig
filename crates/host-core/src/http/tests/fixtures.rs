use crate::{paired_devices::RegistryConfig, shutdown::Core};
use myrig_pairing_crypto::session::AttemptId;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::host::*;

pub(super) async fn secure_confirmation(
    host: &Arc<Host>,
    code: &[u8; 8],
    certificate: &[u8],
) -> (AttemptId, Option<[u8; 64]>) {
    let (exchange, request) =
        myrig_pairing_crypto::client::ClientExchange::start(code, certificate, &[7; 32]).unwrap();
    let correct_binding = Sha256::digest(certificate).into();
    assert_eq!(host.secure_pairing_info(correct_binding), Ok([7; 32]));
    assert!(host.secure_pairing_info([0; 32]).is_err());
    for _ in 0..6 {
        assert!(host.secure_pairing_start(&request, [0; 32]).await.is_err());
    }
    let challenge = host
        .secure_pairing_start(&request, correct_binding)
        .await
        .unwrap();
    assert_eq!(challenge.nonce, [7; 32]);
    (
        challenge.challenge.attempt,
        exchange.finish(&challenge.challenge.response).ok(),
    )
}

pub(super) fn device(index: u128) -> uuid::Uuid {
    uuid::Uuid::from_u128(index)
}

pub(super) fn registry_host(first_token_byte: u8) -> Host {
    use sha2::{Digest, Sha256};
    struct NoPower;
    impl crate::shutdown::Power for NoPower {
        fn schedule_shutdown(&self) -> Result<(), crate::shutdown::PowerError> {
            panic!("local registry operations must not call power")
        }
        fn cancel_shutdown(&self) -> Result<(), crate::shutdown::PowerError> {
            panic!("local registry operations must not call power")
        }
    }
    let digest = |byte: u8| -> String {
        Sha256::digest([byte; 32])
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    };
    let host_id = if first_token_byte == 3 {
        "03c85317-9d93-4fb3-a04b-91a0e16668c7"
    } else {
        "03c85317-9d93-4fb3-a04b-91a0e16668c6"
    };
    let value = json!({"schema_version":3,
        "host_id":host_id, "port":47991, "test_mode":true,
        "paired_devices":[
            {"device_id":"11111111-1111-4111-8111-111111111111", "name":"Device", "token_sha256":digest(first_token_byte)},
            {"device_id":"22222222-2222-4222-8222-222222222222", "name":"Device", "token_sha256":digest(2)}]});
    Host::new(
        RegistryConfig::from_json(&serde_json::to_vec(&value).unwrap()).unwrap(),
        Core::new(Box::new(NoPower)),
        "Fixture PC".into(),
        std::sync::Arc::new(|_| Ok(())),
    )
}

/// Opens a code for `device(index)` with credential entropy `[index; 32]` and
/// finishes pairing, optionally naming the token it replaces.
pub(super) async fn pair_device(
    host: &Arc<Host>,
    index: u8,
    previous: Option<&str>,
) -> Option<String> {
    host.open_secure_pairing(
        host.pairing_approval(),
        "Device",
        "12345678",
        [7; 32],
        b"cert",
        &[index; 32],
        device(index.into()),
        Arc::new(|_| Ok(())),
    )
    .await
    .unwrap();
    let (attempt, confirmation) = secure_confirmation(host, b"12345678", b"cert").await;
    host.secure_pairing_finish(
        attempt,
        &confirmation.unwrap(),
        Sha256::digest(b"cert").into(),
        previous.map(|token| zeroize::Zeroizing::new(token.to_owned())),
    )
    .await
    .ok()
    .map(|token| token.to_string())
}
