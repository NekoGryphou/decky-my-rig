use super::{
    HostState, PROTOCOL_MAJOR, PROTOCOL_MINOR, ProtocolVersion, ShutdownReply, ShutdownRequest,
    StatusReply, Versioned,
};
use serde::Serialize;

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!(
            "../../../../tests/fixtures/lan-api/",
            $name,
            ".json"
        ))
        .trim_end()
    };
}

fn versioned<T: Serialize>(body: T) -> String {
    serde_json::to_string(&Versioned {
        protocol_version: ProtocolVersion {
            major: PROTOCOL_MAJOR,
            minor: PROTOCOL_MINOR,
        },
        body,
    })
    .unwrap()
}

#[test]
fn host_replies_match_the_shared_fixtures_byte_for_byte() {
    let status = StatusReply {
        host_id: "03c85317-9d93-4fb3-a04b-91a0e16668c6".into(),
        app_version: "1.3.0".into(),
        state: HostState::Online,
    };
    assert_eq!(versioned(&status), fixture!("status-online"));
    let accepted = ShutdownReply {
        request_id: "d1d5303f-bfd4-4861-aaaf-e992ec82ee1b".into(),
        state: HostState::ShuttingDown,
        test: None,
    };
    assert_eq!(versioned(&accepted), fixture!("shutdown-accepted"));
    let test = ShutdownReply {
        request_id: "aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa".into(),
        state: HostState::ShuttingDown,
        test: Some(true),
    };
    assert_eq!(versioned(&test), fixture!("shutdown-test-accepted"));
}

#[test]
fn fixtures_parse_back_into_the_shared_types() {
    let status: StatusReply = serde_json::from_str(fixture!("status-online")).unwrap();
    assert_eq!(status.state, HostState::Online);
    let test: ShutdownReply = serde_json::from_str(fixture!("shutdown-test-accepted")).unwrap();
    assert_eq!(test.test, Some(true));
    let request: ShutdownRequest = serde_json::from_str(fixture!("shutdown-request")).unwrap();
    assert_eq!(
        serde_json::to_string(&request).unwrap(),
        fixture!("shutdown-request")
    );
}

macro_rules! pairing_fixture {
    ($name:literal) => {
        include_str!(concat!(
            "../../../../tests/fixtures/pairing/",
            $name,
            ".json"
        ))
        .trim_end()
    };
}

#[test]
fn pairing_requests_match_the_shared_fixtures_byte_for_byte() {
    use super::{PairFinishRequest, PairStartRequest};
    for raw in [
        pairing_fixture!("finish-request"),
        pairing_fixture!("repair-finish-request"),
    ] {
        let request: PairFinishRequest = serde_json::from_str(raw).unwrap();
        assert_eq!(serde_json::to_string(&request).unwrap(), raw);
    }
    let start: PairStartRequest = serde_json::from_str(pairing_fixture!("start-request")).unwrap();
    assert_eq!(
        serde_json::to_string(&start).unwrap(),
        pairing_fixture!("start-request")
    );
}

#[test]
fn pairing_replies_carry_the_shared_fixture_fields() {
    use super::{PairFinishReply, PairInfoReply, PairStartReply};
    use serde_json::Value;
    fn same<T: Serialize + serde::de::DeserializeOwned>(raw: &str) {
        let parsed: T = serde_json::from_str(raw).unwrap();
        let expected: Value = serde_json::from_str(raw).unwrap();
        let actual: Value = serde_json::from_str(&versioned(&parsed)).unwrap();
        assert_eq!(actual, expected);
    }
    same::<PairInfoReply>(pairing_fixture!("info-reply"));
    same::<PairStartReply>(pairing_fixture!("start-reply"));
    same::<PairFinishReply>(pairing_fixture!("finish-reply"));
}
