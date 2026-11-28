use super::fixtures::{CODE, fixture, found};
use uuid::Uuid;

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
    fixture.drafts.close().await;
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
    // ! One poll starts the exchange and leaves it waiting on the network. Then it's dropped, and
    // ! the slot must be free again.
    let mut exchange = Box::pin(fixture.drafts.exchange(reservation, CODE, found));
    std::future::poll_fn(|cx| {
        let _ = exchange.as_mut().poll(cx);
        std::task::Poll::Ready(())
    })
    .await;
    drop(exchange);
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
async fn a_finished_unsaved_pairing_is_not_replaced_by_a_new_one() {
    // ! The first pairing's credential exists on the PC and only this draft holds it. A second
    // ! start must not drop that draft and leave the credential behind.
    let fixture = fixture(Some(CODE)).await;
    fixture.begin(Uuid::new_v4(), CODE, None).await.unwrap();
    let second = Uuid::new_v4().to_string();
    let result = fixture
        .drafts
        .reserve(&second, "127.0.0.1", &fixture.port(), None);
    assert_eq!(result.unwrap_err().0, crate::drafts::messages::BUSY);
}
