use super::{
    Paired, device_name_from,
    fixtures::{PEER_HOST_ID, PairingPeer, pairing_peer, pairing_peer_busy_once, unchanged},
    pair, request_code,
};
use crate::host_client::{
    HostClient, HostError,
    fixtures::{HOST_ID, RealHost, real_host_with, token},
};
use myrig_protocol::lan::{PAIR_FINISH_PATH, PAIR_INFO_PATH, PAIR_START_PATH};
use serde_json::{Value, json};
use std::{net::Ipv4Addr, time::Duration};

const CODE: &str = "12345678";
const FAILED: &str = "Pairing did not complete. Windows may retain this pairing.";
const INCORRECT: &str = "Pairing code is incorrect or the PC could not be verified.";
const EXPIRED: &str = "Pairing code expired or closed. Generate a new code.";
const UNREACHABLE: &str = "PC is unreachable.";
const NOT_SHOWN: &str = "PC could not show a pairing code.";

async fn pair_with(
    host: &RealHost,
    code: &str,
    previous: Option<([u8; 32], String)>,
) -> Result<Paired, &'static str> {
    let previous = previous.as_ref().map(|(pin, token)| (pin, token.as_str()));
    pair(Ipv4Addr::LOCALHOST, host.address.port(), code, previous)
        .await
        .map_err(|error| error.0)
}

async fn pair_peer(
    peer: &PairingPeer,
    code: &str,
    previous: Option<(&[u8; 32], &str)>,
) -> Result<Paired, &'static str> {
    pair(Ipv4Addr::LOCALHOST, peer.address.port(), code, previous)
        .await
        .map_err(|error| error.0)
}

fn status_client(host: &RealHost, paired: &Paired) -> HostClient {
    HostClient::new(
        Ipv4Addr::LOCALHOST,
        host.address.port(),
        Some(paired.authentication.token.to_string()),
        Some(paired.authentication.tls_pin),
        Duration::from_secs(3),
    )
}

#[tokio::test]
async fn pairs_with_the_code_shown_on_the_pc() {
    let host = real_host_with(true, Some(CODE)).await;
    let paired = pair_with(&host, CODE, None).await.unwrap();
    assert_eq!(paired.computer_name, "Fixture PC");
    assert_eq!(paired.authentication.host_id.to_string(), HOST_ID);
    assert_eq!(paired.authentication.tls_pin, host.pin);
    assert!(status_client(&host, &paired).status().await.is_ok());
}

#[tokio::test]
async fn wrong_code_is_reported_and_saves_nothing() {
    let host = real_host_with(true, Some(CODE)).await;
    assert_eq!(
        pair_with(&host, "87654321", None).await.unwrap_err(),
        INCORRECT
    );
}

#[tokio::test]
async fn closed_pairing_is_an_expired_code() {
    let host = real_host_with(true, None).await;
    assert_eq!(pair_with(&host, CODE, None).await.unwrap_err(), EXPIRED);
}

#[tokio::test]
async fn re_pairing_revokes_the_credential_it_replaces() {
    let host = real_host_with(true, Some(CODE)).await;
    let paired = pair_with(&host, CODE, Some((host.pin, token(1))))
        .await
        .unwrap();
    let old = HostClient::new(
        Ipv4Addr::LOCALHOST,
        host.address.port(),
        Some(token(1)),
        Some(host.pin),
        Duration::from_secs(3),
    );
    assert_eq!(old.status().await.unwrap_err(), HostError::Unauthorized);
    assert!(status_client(&host, &paired).status().await.is_ok());
}

#[tokio::test]
async fn previous_token_goes_only_to_the_certificate_that_issued_it() {
    let old = token(5);
    let peer = pairing_peer(CODE, unchanged).await;
    pair_peer(&peer, CODE, Some((&[0; 32], &old)))
        .await
        .unwrap();
    let finish = peer.requests.lock().unwrap().last().unwrap().1.clone();
    assert!(finish.get("previous_token").is_none());
    let peer = pairing_peer(CODE, unchanged).await;
    pair_peer(&peer, CODE, Some((&peer.pin, &old)))
        .await
        .unwrap();
    let finish = peer.requests.lock().unwrap().last().unwrap().1.clone();
    assert_eq!(finish["previous_token"], json!(old));
    assert!(!finish.to_string().contains(CODE));
}

#[tokio::test]
async fn success_returns_the_peer_credentials_after_three_requests() {
    let peer = pairing_peer(CODE, unchanged).await;
    let paired = pair_peer(&peer, CODE, None).await.unwrap();
    assert_eq!(
        peer.paths(),
        [PAIR_INFO_PATH, PAIR_START_PATH, PAIR_FINISH_PATH]
    );
    assert_eq!(paired.authentication.host_id.to_string(), PEER_HOST_ID);
    assert_eq!(*paired.authentication.token, "ab".repeat(32));
    assert_eq!(paired.authentication.tls_pin, peer.pin);
    assert_eq!(paired.computer_name, "Peer PC");
    let debug = format!("{paired:?}");
    assert!(!debug.contains(&"ab".repeat(32)));
}

#[tokio::test]
async fn invalid_info_stops_before_any_pairing_work() {
    let edits: [fn(&str, &mut Value) -> bool; 6] = [
        |path, value| {
            if path == PAIR_INFO_PATH {
                value["extra"] = json!("x");
            }
            true
        },
        |path, value| {
            if path == PAIR_INFO_PATH {
                value["bootstrap_version"] = json!(true);
            }
            true
        },
        |path, value| {
            if path == PAIR_INFO_PATH {
                value["nonce"] = json!("0".repeat(63));
            }
            true
        },
        |path, value| {
            if path == PAIR_INFO_PATH {
                value["nonce"] = json!("A".repeat(64));
            }
            true
        },
        |path, value| {
            if path == PAIR_INFO_PATH {
                value["protocol_version"]["minor"] = json!(1);
            }
            true
        },
        |path, _| path != PAIR_INFO_PATH,
    ];
    for (index, edit) in edits.into_iter().enumerate() {
        let peer = pairing_peer(CODE, edit).await;
        assert_eq!(
            pair_peer(&peer, CODE, None).await.unwrap_err(),
            FAILED,
            "case {index}"
        );
        assert_eq!(peer.paths(), [PAIR_INFO_PATH], "case {index}");
    }
}

#[tokio::test]
async fn a_malformed_start_sends_no_confirmation() {
    let edits: [fn(&str, &mut Value) -> bool; 5] = [
        |path, value| {
            if path == PAIR_START_PATH {
                value["session_nonce"] = json!("00".repeat(32));
            }
            true
        },
        |path, value| {
            if path == PAIR_START_PATH {
                value["attempt_id"] = json!("invalid");
            }
            true
        },
        |path, value| {
            if path == PAIR_START_PATH {
                value["response"] = json!("02".repeat(319));
            }
            true
        },
        |path, value| {
            if path == PAIR_START_PATH {
                value["extra"] = json!(1);
            }
            true
        },
        |path, value| {
            if path == PAIR_START_PATH {
                value["response"] = json!("02".repeat(320));
            }
            true
        },
    ];
    let expected = [FAILED, FAILED, FAILED, FAILED, INCORRECT];
    for (index, edit) in edits.into_iter().enumerate() {
        let peer = pairing_peer(CODE, edit).await;
        assert_eq!(
            pair_peer(&peer, CODE, None).await.unwrap_err(),
            expected[index],
            "case {index}"
        );
        assert_eq!(
            peer.paths(),
            [PAIR_INFO_PATH, PAIR_START_PATH],
            "case {index}"
        );
    }
}

#[tokio::test]
async fn wrong_code_never_sends_a_confirmation() {
    let peer = pairing_peer(CODE, unchanged).await;
    assert_eq!(
        pair_peer(&peer, "87654321", None).await.unwrap_err(),
        INCORRECT
    );
    assert_eq!(peer.paths(), [PAIR_INFO_PATH, PAIR_START_PATH]);
}

#[tokio::test]
async fn finish_credentials_must_be_canonical_and_exact() {
    let edits: [fn(&str, &mut Value) -> bool; 6] = [
        |path, value| {
            if path == PAIR_FINISH_PATH {
                value["token"] = json!(format!("{} ", "ab".repeat(32)));
            }
            true
        },
        |path, value| {
            if path == PAIR_FINISH_PATH {
                value["token"] = json!("ab".repeat(32).to_uppercase());
            }
            true
        },
        |path, value| {
            if path == PAIR_FINISH_PATH {
                value["host_id"] = json!(format!(" {PEER_HOST_ID}"));
            }
            true
        },
        |path, value| {
            if path == PAIR_FINISH_PATH {
                value["extra"] = json!(1);
            }
            true
        },
        |path, value| {
            if path == PAIR_FINISH_PATH {
                value["computer_name"] = json!("bad\nname");
            }
            true
        },
        |path, value| {
            if path == PAIR_FINISH_PATH {
                value["computer_name"] = json!("x".repeat(81));
            }
            true
        },
    ];
    for (index, edit) in edits.into_iter().enumerate() {
        let peer = pairing_peer(CODE, edit).await;
        assert_eq!(
            pair_peer(&peer, CODE, None).await.err(),
            Some(FAILED),
            "case {index}"
        );
        assert_eq!(
            peer.paths(),
            [PAIR_INFO_PATH, PAIR_START_PATH, PAIR_FINISH_PATH]
        );
    }
}

#[tokio::test]
async fn invalid_codes_do_not_connect() {
    let peer = pairing_peer(CODE, unchanged).await;
    for code in ["1234567", "123456789", "abcdefgh", ""] {
        assert_eq!(pair_peer(&peer, code, None).await.unwrap_err(), FAILED);
    }
    assert!(peer.paths().is_empty());
}

#[tokio::test]
async fn unreachable_pc_is_reported() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    assert_eq!(
        pair(Ipv4Addr::LOCALHOST, port, CODE, None)
            .await
            .unwrap_err()
            .0,
        UNREACHABLE
    );
    assert_eq!(
        request_code(Ipv4Addr::LOCALHOST, port, "Steam Deck")
            .await
            .unwrap_err()
            .0,
        UNREACHABLE
    );
}

#[tokio::test]
async fn code_request_asks_the_pc_and_counts_a_shown_code() {
    let host = real_host_with(true, None).await;
    let port = host.address.port();
    request_code(Ipv4Addr::LOCALHOST, port, "steamdeck")
        .await
        .unwrap();
    // - A code is already showing (409), which still means the user can read it.
    request_code(Ipv4Addr::LOCALHOST, port, "steamdeck")
        .await
        .unwrap();
}

#[tokio::test]
async fn invalid_device_names_do_not_connect() {
    let peer = pairing_peer(CODE, unchanged).await;
    for name in ["", "Device\nName", &"x".repeat(81), " Device"] {
        let error = request_code(Ipv4Addr::LOCALHOST, peer.address.port(), name).await;
        assert_eq!(error.unwrap_err().0, NOT_SHOWN);
    }
    assert!(peer.paths().is_empty());
}

#[test]
fn an_unusable_hostname_falls_back_to_a_generic_name() {
    assert_eq!(device_name_from("steamdeck\n"), "steamdeck");
    for hostname in ["", "\n", "bad\tname", &"x".repeat(81)] {
        assert_eq!(device_name_from(hostname), "My device");
    }
}

#[tokio::test]
async fn a_busy_code_is_asked_again_before_finish() {
    // - Another request held the code briefly; that's not an expiry.
    let peer = pairing_peer_busy_once(CODE, unchanged, &[PAIR_INFO_PATH, PAIR_START_PATH]).await;
    let paired = pair_peer(&peer, CODE, None).await.unwrap();
    assert_eq!(paired.computer_name, "Peer PC");
    assert_eq!(
        peer.paths(),
        [
            PAIR_INFO_PATH,
            PAIR_INFO_PATH,
            PAIR_START_PATH,
            PAIR_START_PATH,
            PAIR_FINISH_PATH
        ]
    );
}
