use super::utils::finish_request;
use serde_json::{Map, Value};

fn object(text: &str) -> Map<String, Value> {
    serde_json::from_str(text).unwrap()
}

#[test]
fn finish_accepts_the_shared_request_shapes() {
    let plain = object(include_str!(
        "../../../../../tests/fixtures/pairing/finish-request.json"
    ));
    assert!(finish_request(&plain).unwrap().previous.is_none());
    let repair = object(include_str!(
        "../../../../../tests/fixtures/pairing/repair-finish-request.json"
    ));
    let request = finish_request(&repair).unwrap();
    assert_eq!(
        request.previous.as_deref().map(String::as_str),
        Some("05".repeat(32).as_str())
    );
}

#[test]
fn finish_refuses_malformed_or_extra_previous_tokens() {
    let base = r#""attempt_id":"0101010101010101010101010101010101010101010101010101010101010101","confirmation":"04040404040404040404040404040404040404040404040404040404040404040404040404040404040404040404040404040404040404040404040404040404""#;
    for extra in [
        r#","previous_token":"05""#.to_owned(),
        format!(r#","previous_token":"{}""#, "AB".repeat(32)),
        r#","previous_token":null"#.to_owned(),
        format!(r#","previous_token":"{}","extra":1"#, "05".repeat(32)),
        r#","extra":1"#.to_owned(),
    ] {
        assert!(
            finish_request(&object(&format!("{{{base}{extra}}}"))).is_none(),
            "{extra}"
        );
    }
}
