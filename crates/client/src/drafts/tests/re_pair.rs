use super::fixtures::{CODE, fixture, found};
use crate::host_client::fixtures::token;
use crate::host_client::{HostClient, HostError};
use crate::store::PcChanges;
use std::net::Ipv4Addr;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn re_pairing_keeps_the_new_credential_at_once_and_revokes_the_old() {
    let fixture = fixture(Some(CODE)).await;
    let original = fixture.saved();
    let draft = Uuid::new_v4().to_string();
    fixture
        .drafts
        .begin(
            &draft,
            "127.0.0.1",
            &fixture.port(),
            CODE,
            Some(&original.id.to_string()),
            found,
        )
        .await
        .unwrap();
    // ! It's saved before the user confirms, so cancelling can't lock the device out.
    let stored = fixture.store.configs()[0].clone();
    assert_ne!(stored.authentication.token, original.authentication.token);
    assert_eq!(
        (stored.name.as_str(), stored.host),
        ("Gaming PC", Ipv4Addr::LOCALHOST)
    );
    let old = HostClient::new(
        Ipv4Addr::LOCALHOST,
        original.port,
        Some(token(1)),
        Some(fixture.host.pin),
        Duration::from_secs(3),
    );
    assert_eq!(old.status().await.unwrap_err(), HostError::Unauthorized);
    fixture.drafts.cancel(&draft).await.unwrap();
    assert_eq!(fixture.store.configs()[0], stored);
}

#[tokio::test]
async fn re_pair_save_applies_the_edited_details() {
    let fixture = fixture(Some(CODE)).await;
    let original = fixture.saved();
    let draft = Uuid::new_v4().to_string();
    fixture
        .drafts
        .begin(
            &draft,
            "127.0.0.1",
            &fixture.port(),
            CODE,
            Some(&original.id.to_string()),
            found,
        )
        .await
        .unwrap();
    let saved = fixture
        .drafts
        .save(&draft, "Renamed", "02:00:00:00:00:07")
        .await
        .unwrap();
    assert_eq!(
        (saved.id, saved.name.as_str()),
        (original.id.to_string(), "Renamed")
    );
    assert_eq!(fixture.store.configs().len(), 1);
}

#[tokio::test]
async fn an_edit_before_save_is_not_overwritten() {
    let fixture = fixture(Some(CODE)).await;
    let original = fixture.saved();
    let draft = Uuid::new_v4().to_string();
    fixture
        .drafts
        .begin(
            &draft,
            "127.0.0.1",
            &fixture.port(),
            CODE,
            Some(&original.id.to_string()),
            found,
        )
        .await
        .unwrap();
    fixture
        .store
        .update(
            original.id,
            PcChanges {
                name: Some("Edited elsewhere".into()),
                ..PcChanges::default()
            },
        )
        .unwrap();
    assert_eq!(
        fixture
            .drafts
            .save(&draft, "Mine", "02:00:00:00:00:07")
            .await
            .unwrap_err()
            .0,
        "PC changed during pairing. Windows may retain this pairing."
    );
    assert_eq!(fixture.store.configs()[0].name, "Edited elsewhere");
}
