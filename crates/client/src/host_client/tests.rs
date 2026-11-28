use super::{
    HostClient, HostError, ShutdownResult,
    fixtures::{HOST_ID, http_reply, real_host, scripted_peer, token},
};
use hyper::Method;
use myrig_protocol::lan::{HostState, PAIR_INFO_PATH, PAIR_REQUEST_PATH, STATUS_PATH};
use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::atomic::Ordering,
    time::Duration,
};
use uuid::Uuid;

const DEADLINE: Duration = Duration::from_secs(3);

fn client(address: SocketAddr, token: Option<String>, pin: Option<[u8; 32]>) -> HostClient {
    HostClient::new(Ipv4Addr::LOCALHOST, address.port(), token, pin, DEADLINE)
}

fn status_body(major: u32) -> String {
    format!(
        r#"{{"protocol_version":{{"major":{major},"minor":0}},"host_id":"{HOST_ID}","app_version":"1","state":"online"}}"#
    )
}

#[tokio::test]
async fn status_reports_the_host_identity_and_state() {
    let host = real_host(true).await;
    let status = client(host.address, Some(token(1)), Some(host.pin))
        .status()
        .await
        .unwrap();
    assert_eq!(status.host_id, Uuid::parse_str(HOST_ID).unwrap());
    assert_eq!(status.state, HostState::Online);
}

#[tokio::test]
async fn test_mode_stop_reports_test_and_keeps_the_pc_on() {
    let host = real_host(true).await;
    let client = client(host.address, Some(token(1)), Some(host.pin));
    let result = client.shutdown(Uuid::new_v4()).await.unwrap();
    assert_eq!(result, ShutdownResult { test: true });
    assert_eq!(host.power_calls.load(Ordering::SeqCst), 0);
    assert_eq!(client.status().await.unwrap().state, HostState::Online);
}

#[tokio::test]
async fn real_stop_schedules_shutdown_once() {
    let host = real_host(false).await;
    let client = client(host.address, Some(token(1)), Some(host.pin));
    assert_eq!(
        client.shutdown(Uuid::new_v4()).await.unwrap(),
        ShutdownResult { test: false }
    );
    assert_eq!(host.power_calls.load(Ordering::SeqCst), 1);
    assert_eq!(
        client.status().await.unwrap().state,
        HostState::ShuttingDown
    );
}

#[tokio::test]
async fn unpair_revokes_only_this_credential() {
    let host = real_host(true).await;
    let first = client(host.address, Some(token(1)), Some(host.pin));
    first.unpair().await.unwrap();
    assert_eq!(first.status().await.unwrap_err(), HostError::Unauthorized);
    let second = client(host.address, Some(token(2)), Some(host.pin));
    assert!(second.status().await.is_ok());
}

#[tokio::test]
async fn wrong_token_is_unauthorized() {
    let host = real_host(true).await;
    let error = client(host.address, Some(token(9)), Some(host.pin))
        .status()
        .await
        .unwrap_err();
    assert_eq!(error, HostError::Unauthorized);
}

#[tokio::test]
async fn pin_mismatch_sends_no_request() {
    let peer = scripted_peer(Some(http_reply("200 OK", "", &status_body(1))), true).await;
    let error = client(peer.address, Some(token(1)), Some([0; 32]))
        .status()
        .await
        .unwrap_err();
    assert_eq!(error, HostError::PinMismatch);
    assert_eq!(peer.requests.load(Ordering::SeqCst), 0);
    let real = real_host(true).await;
    let error = client(real.address, Some(token(1)), Some(peer.pin))
        .status()
        .await
        .unwrap_err();
    assert_eq!(error, HostError::PinMismatch);
}

#[tokio::test]
async fn unsupported_major_is_reported() {
    let peer = scripted_peer(Some(http_reply("200 OK", "", &status_body(2))), true).await;
    let error = client(peer.address, Some(token(1)), Some(peer.pin))
        .status()
        .await
        .unwrap_err();
    assert_eq!(error, HostError::UnsupportedProtocol);
}

#[tokio::test]
async fn silent_host_hits_the_deadline() {
    let peer = scripted_peer(None, true).await;
    let client = HostClient::new(
        Ipv4Addr::LOCALHOST,
        peer.address.port(),
        Some(token(1)),
        Some(peer.pin),
        Duration::from_millis(300),
    );
    assert_eq!(client.status().await.unwrap_err(), HostError::Timeout);
}

#[tokio::test]
async fn closed_port_is_unreachable() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    drop(listener);
    let error = client(address, Some(token(1)), Some([0; 32]))
        .status()
        .await
        .unwrap_err();
    // ? Linux refuses right away; Windows retries a refused connection for about as long as the
    // ? connect timeout. Callers treat both as unreachable.
    assert!(
        matches!(error, HostError::Connection | HostError::Timeout),
        "{error:?}"
    );
}

#[tokio::test]
async fn malformed_replies_are_invalid() {
    let oversized_header = format!("x-padding: {}\r\n", "a".repeat(17_000));
    let oversized_body = format!("{{\"padding\":\"{}\"}}", "a".repeat(17_000));
    let duplicate = format!(r#"{{"state":"online",{}"#, &status_body(1)[1..]);
    let cases = [
        http_reply("200 OK", &oversized_header, &status_body(1)),
        http_reply("200 OK", "", &oversized_body),
        http_reply("200 OK", "content-type: application/json\r\n", &status_body(1)),
        http_reply("200 OK", "", &duplicate),
        http_reply("200 OK", "", "[]"),
        http_reply("200 OK", "", "{not json"),
        http_reply("200 OK", "", &status_body(1).replace("online", "sleeping")),
        http_reply("200 OK", "", &status_body(1).replace(HOST_ID, "not-a-uuid")),
        http_reply("200 OK", "", &status_body(1).replace(r#""minor":0"#, r#""minor":-1"#)),
        http_reply("202 Accepted", "", &status_body(1)),
        format!(
            "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ntransfer-encoding: chunked\r\n\r\n{:x}\r\n{}\r\n0\r\n\r\n",
            status_body(1).len(),
            status_body(1)
        )
        .into_bytes(),
        format!("HTTP/1.1 200 OK\r\ncontent-type: text/plain\r\ncontent-length: {}\r\n\r\n{}", status_body(1).len(), status_body(1)).into_bytes(),
    ];
    for (index, reply) in cases.into_iter().enumerate() {
        let peer = scripted_peer(Some(reply), true).await;
        let error = client(peer.address, Some(token(1)), Some(peer.pin))
            .status()
            .await
            .unwrap_err();
        assert_eq!(error, HostError::Invalid, "case {index}");
    }
}

#[tokio::test]
async fn missing_and_server_errors_are_connection_failures() {
    for status in [
        "404 Not Found",
        "500 Internal Server Error",
        "503 Service Unavailable",
    ] {
        let peer = scripted_peer(Some(http_reply(status, "", "{}")), true).await;
        let error = client(peer.address, Some(token(1)), Some(peer.pin))
            .status()
            .await
            .unwrap_err();
        assert_eq!(error, HostError::Connection, "{status}");
    }
}

#[tokio::test]
async fn truncated_body_is_a_connection_failure() {
    let body = status_body(1);
    let reply = format!(
        "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
        body.len() + 10,
        body
    );
    let peer = scripted_peer(Some(reply.into_bytes()), true).await;
    let error = client(peer.address, Some(token(1)), Some(peer.pin))
        .status()
        .await
        .unwrap_err();
    assert_eq!(error, HostError::Connection);
}

#[tokio::test]
async fn missing_alpn_is_invalid() {
    let peer = scripted_peer(Some(http_reply("200 OK", "", &status_body(1))), false).await;
    let error = client(peer.address, Some(token(1)), Some(peer.pin))
        .status()
        .await
        .unwrap_err();
    assert_eq!(error, HostError::Invalid);
}

#[tokio::test]
async fn shutdown_reply_must_echo_the_request() {
    let id = Uuid::new_v4();
    let body = |request_id: &str, extra: &str| {
        format!(
            r#"{{"protocol_version":{{"major":1,"minor":0}},"request_id":"{request_id}","state":"shutting_down"{extra}}}"#
        )
    };
    let other = Uuid::new_v4().to_string();
    for reply in [
        body(&other, ""),
        body(&id.to_string(), r#","test":false"#),
        body(&id.to_string(), r#","test":"yes""#),
        body(&id.to_string(), r#","test":null"#),
        body(&id.to_string(), "").replace("shutting_down", "online"),
    ] {
        let peer = scripted_peer(Some(http_reply("202 Accepted", "", &reply)), true).await;
        let error = client(peer.address, Some(token(1)), Some(peer.pin))
            .shutdown(id)
            .await
            .unwrap_err();
        assert_eq!(error, HostError::Invalid, "{reply}");
    }
}

#[tokio::test]
async fn unverified_requests_are_only_the_two_bootstrap_routes() {
    let peer = scripted_peer(Some(http_reply("200 OK", "", "{}")), true).await;
    let unverified = client(peer.address, None, None);
    for (method, path, body) in [
        (Method::GET, STATUS_PATH, None),
        (Method::POST, PAIR_INFO_PATH, Some(b"{}".to_vec())),
        (Method::GET, PAIR_REQUEST_PATH, None),
    ] {
        let error = unverified
            .request_unverified(method, path, body)
            .await
            .unwrap_err();
        assert_eq!(error, HostError::Invalid, "{path}");
    }
    let pinned = client(peer.address, None, Some(peer.pin));
    let error = pinned
        .request_unverified(Method::GET, PAIR_INFO_PATH, None)
        .await
        .unwrap_err();
    assert_eq!(error, HostError::Invalid);
    assert_eq!(unverified.status().await.unwrap_err(), HostError::Invalid);
    assert_eq!(peer.requests.load(Ordering::SeqCst), 0);
    let (reply, certificate) = unverified
        .request_unverified(Method::GET, PAIR_INFO_PATH, None)
        .await
        .unwrap();
    assert_eq!((reply.status, certificate), (200, peer.certificate.clone()));
    assert_eq!(peer.requests.load(Ordering::SeqCst), 1);
}

#[test]
fn reply_debug_output_leaves_out_the_body() {
    let reply = super::HttpReply {
        status: 200,
        content_type: None,
        cache_control: None,
        body: br#"{"token":"secret-token"}"#.to_vec(),
    };
    let debug = format!("{reply:?}");
    assert!(
        !debug.contains("body") && !debug.contains("115, 101, 99"),
        "{debug}"
    );
}

#[test]
fn only_the_answers_the_user_can_act_on_are_issues() {
    use myrig_protocol::api::PcIssue;
    assert_eq!(HostError::PinMismatch.issue(), Some(PcIssue::Identity));
    assert_eq!(
        HostError::Unauthorized.issue(),
        Some(PcIssue::Authentication)
    );
    assert_eq!(
        HostError::UnsupportedProtocol.issue(),
        Some(PcIssue::Compatibility)
    );
    for error in [
        HostError::Invalid,
        HostError::Connection,
        HostError::Timeout,
    ] {
        assert_eq!(error.issue(), None);
    }
}
