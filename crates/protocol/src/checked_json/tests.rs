use super::CheckedJson;

#[test]
fn rejects_duplicates_everywhere() {
    for input in [
        r#"{"a":1,"a":2}"#,
        r#"{"unknown":{"a":1,"a":2}}"#,
        r#"{"unknown":[{"a":1,"a":2}]}"#,
        r#"{"a":1,"\u0061":2}"#,
        r#"{"a":NaN}"#,
    ] {
        assert!(serde_json::from_str::<CheckedJson>(input).is_err());
    }
    assert!(
        serde_json::from_str::<CheckedJson>(r#"{"unknown":[null,true,1,1.5,{"x":"ok"}]}"#).is_ok()
    );
}
