use super::messages::FAILURE;
use super::*;
use myrig_protocol::validation::decode_hex_bytes;
use uuid::Uuid;
const HOST: &str = "03c85317-9d93-4fb3-a04b-91a0e16668c6";
fn document() -> serde_json::Value {
    serde_json::json!({"schema_version":1,"host_id":HOST,
        "certificate_der":"010203",
        "private_key_pkcs8":"040506"})
}
fn generated_document() -> Vec<u8> {
    let hex = |bytes: Vec<u8>| {
        bytes
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let certificate = std::fs::read(
        std::env::var_os("MY_RIG_TEST_SECURITY_CERT")
            .expect("isolated certificate fixture required"),
    )
    .unwrap();
    let key = std::fs::read(
        std::env::var_os("MY_RIG_TEST_SECURITY_KEY").expect("isolated key fixture required"),
    )
    .unwrap();
    let mut value = document();
    value["certificate_der"] = serde_json::json!(hex(certificate));
    value["private_key_pkcs8"] = serde_json::json!(hex(key));
    serde_json::to_vec(&value).unwrap()
}

#[test]
#[ignore = "generated isolated certificate/key fixture required"]
fn generated_identity_roundtrip_and_mismatch() {
    let bytes = generated_document();
    let host = Uuid::parse_str(HOST).unwrap();
    let identity = HostSecurity::from_trusted_json(&bytes, host)
        .unwrap()
        .into_identity();
    assert_eq!(
        identity.leaf_der(),
        std::fs::read(std::env::var_os("MY_RIG_TEST_SECURITY_CERT").unwrap()).unwrap()
    );
    let other = std::fs::read(
        std::env::var_os("MY_RIG_TEST_SECURITY_OTHER_CERT")
            .expect("different isolated identity required"),
    )
    .unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["certificate_der"] = serde_json::json!(
        other
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );
    refuse(&serde_json::to_vec(&value).unwrap());
}
fn refuse(bytes: &[u8]) {
    assert_eq!(
        HostSecurity::from_trusted_json(bytes, Uuid::parse_str(HOST).unwrap()).err(),
        Some(FAILURE)
    );
}
#[test]
fn strict_security_shape_host_binding_and_safe_errors() {
    refuse(b"");
    refuse(&vec![b' '; MAX_SECURITY_BYTES + 1]);
    refuse(br#"{"schema_version":1,"schema_version":1}"#);
    for (field, value) in [
        ("schema_version", serde_json::json!(true)),
        ("schema_version", serde_json::json!(2)),
        (
            "host_id",
            serde_json::json!("03c85317-9d93-4fb3-a04b-91a0e16668c7"),
        ),
        ("host_id", serde_json::json!(HOST.to_uppercase())),
        ("certificate_der", serde_json::json!("AB")),
        ("certificate_der", serde_json::json!(null)),
        (
            "private_key_pkcs8",
            serde_json::json!("dead secret fixture"),
        ),
        (
            "unknown",
            serde_json::json!("input must not appear in error"),
        ),
    ] {
        let mut value_document = document();
        value_document[field] = value;
        refuse(&serde_json::to_vec(&value_document).unwrap());
    }
    for field in ["host_id", "certificate_der", "private_key_pkcs8"] {
        let mut value = document();
        value.as_object_mut().unwrap().remove(field);
        refuse(&serde_json::to_vec(&value).unwrap());
    }
    refuse(&serde_json::to_vec(&document()).unwrap()); // Invalid actual certificate/key.
}
#[test]
fn bounded_canonical_secret_hex() {
    assert_eq!(
        &*decode_hex_bytes("00abcdef", 4).unwrap(),
        &[0, 171, 205, 239]
    );
    for value in ["", "0", "AB", "ff ", "gg", "0000000000"] {
        assert!(decode_hex_bytes(value, 4).is_none());
    }
}
