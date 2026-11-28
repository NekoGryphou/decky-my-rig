use super::fixtures::{CODE, fixture};
use crate::host_client::fixtures::token;
use crate::host_client::{HostClient, HostError};
use crate::store::{Authentication, PcChanges};
use std::net::Ipv4Addr;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn cancelling_a_new_pairing_revokes_its_credential_on_the_pc() {
    // ! The PC saved this device when pairing finished, so an unsaved draft mustn't leave it there.
    let fixture = fixture(Some(CODE)).await;
    let draft = Uuid::new_v4();
    fixture.begin(draft, CODE, None).await.unwrap();
    let issued = HostClient::new(
        Ipv4Addr::LOCALHOST,
        fixture.host.address.port(),
        Some(token(8)),
        Some(fixture.host.pin),
        Duration::from_secs(3),
    );
    assert!(issued.status().await.is_ok());
    fixture.drafts.cancel(&draft.to_string()).await.unwrap();
    assert_eq!(issued.status().await.unwrap_err(), HostError::Unauthorized);
    assert!(fixture.store.configs().is_empty());
}

#[tokio::test]
async fn a_save_that_cannot_store_revokes_the_new_credential() {
    // ! The PC saved the device when pairing finished. If this device can't store it, the PC must
    // ! not keep a credential nobody holds. The store document is corrupted after pairing, so the
    // ! save fails the same way on every platform.
    let fixture = fixture(Some(CODE)).await;
    let draft = Uuid::new_v4();
    fixture.begin(draft, CODE, None).await.unwrap();
    let issued = HostClient::new(
        Ipv4Addr::LOCALHOST,
        fixture.host.address.port(),
        Some(token(8)),
        Some(fixture.host.pin),
        Duration::from_secs(3),
    );
    assert!(issued.status().await.is_ok());
    std::fs::write(fixture._directory.path().join("pcs.json"), b"not a pc list").unwrap();
    let saved = fixture
        .drafts
        .save(&draft.to_string(), "PC", "02:00:00:00:00:05")
        .await;
    assert!(saved.is_err());
    assert_eq!(issued.status().await.unwrap_err(), HostError::Unauthorized);
}

#[tokio::test]
async fn a_pc_changed_before_its_credential_is_kept_is_not_reported_as_stored() {
    // ! The store's record changed identity under the draft, so the new credential was not saved.
    // ! Reporting it as stored would skip the revoke and leave the PC with a device nobody holds.
    let fixture = fixture(Some(CODE)).await;
    let original = fixture.saved();
    fixture
        .store
        .update(
            original.id,
            PcChanges {
                authentication: Some(Authentication {
                    host_id: original.authentication.host_id,
                    token: token(2).into(),
                    tls_pin: [0x34; 32],
                }),
                ..PcChanges::default()
            },
        )
        .unwrap();
    let paired = crate::pairing::Paired {
        authentication: Authentication {
            host_id: original.authentication.host_id,
            token: token(8).into(),
            tls_pin: fixture.host.pin,
        },
        computer_name: "Gaming PC".into(),
    };
    let before = fixture.store.configs();
    let error = fixture
        .drafts
        .keep_credential(original, &paired)
        .await
        .unwrap_err();
    assert_eq!(error.0, crate::drafts::messages::PC_CHANGED);
    // ? The record must keep its own credential: nothing new was stored.
    assert_eq!(fixture.store.configs(), before);
}

#[tokio::test]
async fn a_re_pair_answered_by_another_certificate_waits_for_save() {
    // ! The old credential wasn't revoked (its token only goes to its own certificate), so the
    // ! saved PC mustn't pick up the new one before Save.
    let fixture = fixture(Some(CODE)).await;
    let mut original = fixture.saved();
    original = fixture
        .store
        .update(
            original.id,
            PcChanges {
                authentication: Some(Authentication {
                    tls_pin: [0; 32],
                    ..original.authentication.clone()
                }),
                ..PcChanges::default()
            },
        )
        .unwrap();
    let draft = Uuid::new_v4();
    fixture.begin(draft, CODE, Some(&original)).await.unwrap();
    assert_eq!(fixture.store.configs(), vec![original]);
}
