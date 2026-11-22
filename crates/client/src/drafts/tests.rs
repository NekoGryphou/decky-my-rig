use super::Drafts;
use crate::{
    host_client::{
        HostClient, HostError,
        fixtures::{RealHost, real_host_with, token},
    },
    shared::PublicError,
    store::{Authentication, PcChanges, PcConfig, PcInput, Store},
};
use myrig_protocol::api::PairingDraftResult;
use std::{net::Ipv4Addr, sync::Arc, time::Duration};
use tempfile::TempDir;
use uuid::Uuid;

const CODE: &str = "12345678";

fn found(_: Ipv4Addr) -> Result<String, PublicError> {
    Ok("02:00:00:00:00:09".to_owned())
}

fn missing(_: Ipv4Addr) -> Result<String, PublicError> {
    Err(PublicError(
        "MAC detection failed. Enter the MAC address manually.",
    ))
}

struct Fixture {
    _directory: TempDir,
    store: Arc<Store>,
    drafts: Drafts,
    host: RealHost,
}

async fn fixture(code: Option<&str>) -> Fixture {
    let directory = TempDir::new().unwrap();
    let store =
        Arc::new(Store::open(directory.path(), crate::platform::fixtures::platform()).unwrap());
    Fixture {
        drafts: Drafts::new(store.clone(), "test-device".to_owned()),
        store,
        _directory: directory,
        host: real_host_with(true, code).await,
    }
}

impl Fixture {
    fn port(&self) -> String {
        self.host.address.port().to_string()
    }
    /// A PC saved with the host's pin and the registry's token 1.
    fn saved(&self) -> PcConfig {
        self.store
            .add(
                PcInput {
                    name: "Gaming PC".into(),
                    host: "127.0.0.1".into(),
                    port: Some(self.port()),
                    mac: "02:00:00:00:00:01".into(),
                },
                Authentication {
                    host_id: Uuid::parse_str(crate::host_client::fixtures::HOST_ID).unwrap(),
                    token: token(1).into(),
                    tls_pin: self.host.pin,
                },
            )
            .unwrap()
    }
    async fn begin(
        &self,
        draft: Uuid,
        code: &str,
        original: Option<&PcConfig>,
    ) -> Result<PairingDraftResult, &'static str> {
        let original = original.map(|pc| pc.id.to_string());
        self.drafts
            .begin(
                &draft.to_string(),
                "127.0.0.1",
                &self.port(),
                code,
                original.as_deref(),
                found,
            )
            .await
            .map_err(|error| error.0)
    }
}

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
            .is_err()
    );
    assert!(fixture.store.configs().is_empty());
}

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
    // ! It's saved before the user confirms, so cancelling can't lock the device
    //   out.
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
            .unwrap_err()
            .0,
        "PC changed during pairing. Windows may retain this pairing."
    );
    assert_eq!(fixture.store.configs()[0].name, "Edited elsewhere");
}

#[tokio::test]
async fn one_pairing_at_a_time_and_close_ends_them() {
    let fixture = fixture(Some(CODE)).await;
    let draft = Uuid::new_v4().to_string();
    let reservation = fixture
        .drafts
        .reserve(&draft, "127.0.0.1", &fixture.port(), None)
        .unwrap();
    assert_eq!(
        fixture
            .drafts
            .reserve(
                &Uuid::new_v4().to_string(),
                "127.0.0.1",
                &fixture.port(),
                None
            )
            .unwrap_err()
            .0,
        "Pairing is already in progress."
    );
    fixture.drafts.cancel(&draft).await.unwrap();
    assert_eq!(
        fixture
            .drafts
            .exchange(reservation, CODE, found)
            .await
            .unwrap_err()
            .0,
        "Pairing was cancelled. Windows may retain this pairing."
    );
    fixture.drafts.close();
    assert_eq!(
        fixture
            .drafts
            .reserve(&draft, "127.0.0.1", &fixture.port(), None)
            .unwrap_err()
            .0,
        "Pairing is unavailable."
    );
}

#[tokio::test]
async fn a_dropped_exchange_frees_the_slot() {
    let fixture = fixture(Some(CODE)).await;
    let draft = Uuid::new_v4().to_string();
    let reservation = fixture
        .drafts
        .reserve(&draft, "127.0.0.1", &fixture.port(), None)
        .unwrap();
    drop(fixture.drafts.exchange(reservation, CODE, found));
    let exchange = fixture.drafts.exchange(reservation, CODE, found);
    let _ = tokio::time::timeout(Duration::from_millis(1), exchange).await;
    fixture
        .drafts
        .reserve(
            &Uuid::new_v4().to_string(),
            "127.0.0.1",
            &fixture.port(),
            None,
        )
        .unwrap();
}

#[tokio::test]
async fn cancelling_a_new_pairing_revokes_its_credential_on_the_pc() {
    // ! The PC saved this device when pairing finished, so an unsaved draft
    //   mustn't leave it there.
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
async fn a_re_pair_answered_by_another_certificate_waits_for_save() {
    // ! The old credential wasn't revoked (its token only goes to its own
    //   certificate), so the saved PC mustn't pick up the new one before Save.
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
