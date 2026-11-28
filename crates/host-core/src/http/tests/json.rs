use super::super::utils::{checked_value, parse_checked};
use myrig_protocol::lan::ShutdownRequest;

const ID: &str = "11111111-1111-4111-8111-111111111111";

#[test]
fn a_body_is_parsed_once_into_the_wanted_type() {
    let request: ShutdownRequest =
        parse_checked(format!(r#"{{"request_id":"{ID}"}}"#).as_bytes()).unwrap();
    assert_eq!(request.request_id, ID);
    assert!(parse_checked::<ShutdownRequest>(br#"{"request_id":1}"#).is_none());
    assert!(parse_checked::<ShutdownRequest>(b"not json").is_none());
}

#[test]
fn duplicate_keys_are_rejected_at_any_depth() {
    let twice = format!(r#"{{"request_id":"{ID}","request_id":"{ID}"}}"#);
    assert!(parse_checked::<ShutdownRequest>(twice.as_bytes()).is_none());
    assert!(checked_value(br#"{"a":1,"a":2}"#).is_none());
    assert!(checked_value(br#"{"outer":{"a":1,"a":2}}"#).is_none());
    assert!(checked_value(br#"{"a":1,"b":2}"#).is_some());
}
