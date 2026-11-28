use super::fixtures::{CODE, fixture, found, missing};
use crate::host_client::HostClient;
use std::net::Ipv4Addr;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn pairing_prefills_details_and_saves_only_after_confirmation() {
    let fixture = fixture(Some(CODE)).await;
    let draft = Uuid::new_v4();
    let result = fixture.begin(draft, CODE, None).await.unwrap();
    assert_eq!(result.draft_id, draft.to_string());
    assert_eq!(
        (
            result.name.as_str(),
            result.mac.as_deref(),
            result.mac_error
        ),
        ("Fixture PC", Some("02:00:00:00:00:09"), None)
    );
    assert!(fixture.store.configs().is_empty());
    let saved = fixture
        .drafts
        .save(&draft.to_string(), " Living room ", "02-00-00-00-00-09")
        .await
        .unwrap();
    assert_eq!(
        (saved.name.as_str(), saved.host.as_str(), saved.mac.as_str()),
        ("Living room", "127.0.0.1", "02:00:00:00:00:09")
    );
    let pc = &fixture.store.configs()[0];
    let client = HostClient::new(
        Ipv4Addr::LOCALHOST,
        pc.port,
        Some(pc.authentication.token.to_string()),
        Some(pc.authentication.tls_pin),
        Duration::from_secs(3),
    );
    assert!(client.status().await.is_ok());
    assert_eq!(
        fixture
            .drafts
            .save(&draft.to_string(), "Again", "02:00:00:00:00:09")
            .await
            .unwrap_err()
            .0,
        "Pairing was cancelled. Windows may retain this pairing."
    );
}

#[tokio::test]
async fn failed_mac_detection_leaves_manual_entry() {
    let fixture = fixture(Some(CODE)).await;
    let draft = Uuid::new_v4().to_string();
    let result = fixture
        .drafts
        .begin(&draft, "127.0.0.1", &fixture.port(), CODE, None, missing)
        .await
        .unwrap();
    assert_eq!(
        (result.mac, result.mac_error.as_deref()),
        (
            None,
            Some("MAC detection failed. Enter the MAC address manually.")
        )
    );
    fixture
        .drafts
        .save(&draft, "PC", "02:00:00:00:00:05")
        .await
        .unwrap();
}

#[tokio::test]
async fn invalid_input_does_not_pair_and_frees_the_slot() {
    let fixture = fixture(Some(CODE)).await;
    let draft = Uuid::new_v4();
    assert_eq!(
        fixture.begin(draft, "1234", None).await.unwrap_err(),
        "Enter the eight-digit pairing code."
    );
    assert_eq!(
        fixture
            .drafts
            .begin("bad", "127.0.0.1", &fixture.port(), CODE, None, found)
            .await
            .unwrap_err()
            .0,
        "Invalid identifier."
    );
    assert_eq!(
        fixture
            .drafts
            .begin(
                &draft.to_string(),
                "pc.local",
                &fixture.port(),
                CODE,
                None,
                found
            )
            .await
            .unwrap_err()
            .0,
        "Enter the PC's IPv4 address."
    );
    let unknown = Uuid::new_v4().to_string();
    assert_eq!(
        fixture
            .drafts
            .begin(
                &draft.to_string(),
                "127.0.0.1",
                &fixture.port(),
                CODE,
                Some(&unknown),
                found
            )
            .await
            .unwrap_err()
            .0,
        "PC was not found."
    );
    // - The code hasn't been used yet, so a valid attempt works.
    fixture.begin(Uuid::new_v4(), CODE, None).await.unwrap();
}

#[tokio::test]
async fn a_wrong_code_reports_why_and_save_is_not_ready() {
    let fixture = fixture(Some(CODE)).await;
    let draft = Uuid::new_v4();
    assert_eq!(
        fixture.begin(draft, "87654321", None).await.unwrap_err(),
        "Pairing code is incorrect or the PC could not be verified."
    );
    assert_eq!(
        fixture
            .drafts
            .save(&draft.to_string(), "PC", "02:00:00:00:00:05")
            .await
            .unwrap_err()
            .0,
        "Pairing was cancelled. Windows may retain this pairing."
    );
}

#[tokio::test]
async fn cancel_discards_a_ready_draft() {
    let fixture = fixture(Some(CODE)).await;
    let draft = Uuid::new_v4().to_string();
    fixture
        .drafts
        .begin(&draft, "127.0.0.1", &fixture.port(), CODE, None, found)
        .await
        .unwrap();
    fixture.drafts.cancel(&draft).await.unwrap();
    assert!(
        fixture
            .drafts
            .save(&draft, "PC", "02:00:00:00:00:05")
            .await
            .is_err()
    );
    assert!(fixture.store.configs().is_empty());
}
