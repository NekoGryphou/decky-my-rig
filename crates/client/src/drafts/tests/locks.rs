use super::fixtures::{CODE, fixture};
use crate::host_client::fixtures::token;
use crate::host_client::{HostClient, HostError};
use std::net::Ipv4Addr;
use std::time::Duration;
use uuid::Uuid;

#[tokio::test]
async fn a_store_lock_held_elsewhere_does_not_stall_the_runtime_during_save() {
    use crate::platform::fixtures::lock_held_elsewhere;
    use std::sync::atomic::{AtomicUsize, Ordering};
    let fixture = fixture(Some(CODE)).await;
    let draft = Uuid::new_v4();
    fixture.begin(draft, CODE, None).await.unwrap();
    let held = lock_held_elsewhere(fixture._directory.path());
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(500));
        drop(held);
    });
    let ticks = AtomicUsize::new(0);
    let ticker = async {
        loop {
            tokio::time::sleep(Duration::from_millis(20)).await;
            ticks.fetch_add(1, Ordering::SeqCst);
        }
    };
    let draft = draft.to_string();
    let save = fixture.drafts.save(&draft, "PC", "02:00:00:00:00:09");
    tokio::select! {
        saved = save => { saved.unwrap(); }
        () = ticker => {}
    }
    release.join().unwrap();
    assert!(ticks.load(Ordering::SeqCst) >= 3, "the runtime stood still");
}

#[tokio::test]
async fn a_pairing_holds_its_credential_until_the_save_ends() {
    use crate::platform::fixtures::lock_held_elsewhere;
    let fixture = fixture(Some(CODE)).await;
    let draft = Uuid::new_v4();
    assert!(!fixture.drafts.holds_credential());
    fixture.begin(draft, CODE, None).await.unwrap();
    // ! The exchange ended, but its credential is neither stored nor revoked.
    assert!(fixture.drafts.holds_credential());
    let held = lock_held_elsewhere(fixture._directory.path());
    let draft = draft.to_string();
    let mut save = std::pin::pin!(fixture.drafts.save(&draft, "PC", "02:00:00:00:00:09"));
    // - Blocked on the store lock: the credential is neither stored nor revoked.
    let blocked = tokio::time::timeout(Duration::from_millis(100), &mut save).await;
    assert!(blocked.is_err());
    assert!(fixture.drafts.holds_credential());
    drop(held);
    save.await.unwrap();
    assert!(!fixture.drafts.holds_credential());
}

#[tokio::test]
async fn a_cancelled_pairing_stops_holding_its_credential() {
    let fixture = fixture(Some(CODE)).await;
    let draft = Uuid::new_v4();
    fixture.begin(draft, CODE, None).await.unwrap();
    assert!(fixture.drafts.holds_credential());
    fixture.drafts.cancel(&draft.to_string()).await.unwrap();
    assert!(!fixture.drafts.holds_credential());
}

#[tokio::test]
async fn an_abandoned_save_still_revokes_a_credential_it_cannot_store() {
    use crate::platform::fixtures::lock_held_elsewhere;
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
    let held = lock_held_elsewhere(fixture._directory.path());
    let draft = draft.to_string();
    // - The save waits on the store lock and is dropped there, as the agent
    //   does when its drain runs out.
    let abandoned = tokio::time::timeout(
        Duration::from_millis(100),
        fixture.drafts.save(&draft, "PC", "02:00:00:00:00:05"),
    )
    .await;
    assert!(abandoned.is_err());
    std::fs::write(fixture._directory.path().join("pcs.json"), b"not a pc list").unwrap();
    drop(held);
    // ! Closing waits for the abandoned save to finish its store write or revoke.
    fixture.drafts.close().await;
    assert_eq!(issued.status().await.unwrap_err(), HostError::Unauthorized);
}
