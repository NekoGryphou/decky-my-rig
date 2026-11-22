use super::utils::{digests_match, token_digest};
use super::*;
use myrig_protocol::validation::decode_hex;
use myrig_protocol::validation::lower_hex;
use sha2::{Digest, Sha256};
use uuid::Uuid;
const CONFIG_FIXTURE: &str =
    include_str!("../../../../tests/fixtures/host-config/host-config.json");
const HOST: &str = "03c85317-9d93-4fb3-a04b-91a0e16668c6";
const FIRST: &str = "11111111-1111-4111-8111-111111111111";
const SECOND: &str = "22222222-2222-4222-8222-222222222222";
const TOKEN: &str = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f";
fn fixture() -> serde_json::Value {
    serde_json::json!({"schema_version":3,"host_id":HOST,"port":47991,"test_mode":true,"paired_devices":[
        {"device_id":FIRST,"name":"Device","token_sha256":lower_hex(&Sha256::digest([1u8;32]))},
        {"device_id":SECOND,"name":"Device","token_sha256":lower_hex(&Sha256::digest([2u8;32]))}]})
}
fn parse(value: &serde_json::Value) -> Result<RegistryConfig, &'static str> {
    RegistryConfig::from_json(&serde_json::to_vec(value).unwrap())
}
#[test]
fn independent_authentication_revocation_and_restart_roundtrip() {
    let mut config = parse(&fixture()).unwrap();
    assert!(config.authenticate_device(&"01".repeat(32)).is_some());
    assert!(config.authenticate_device(&"02".repeat(32)).is_some());
    assert!(config.authenticate_device(&"03".repeat(32)).is_none());
    assert!(
        config
            .authenticate_device("private malformed token")
            .is_none()
    );
    assert!(config.revoke(Uuid::parse_str(FIRST).unwrap()));
    assert!(!config.revoke(Uuid::parse_str(FIRST).unwrap()));
    let restored = RegistryConfig::from_json(&config.to_json().unwrap()).unwrap();
    assert!(restored.authenticate_device(&"01".repeat(32)).is_none());
    assert!(restored.authenticate_device(&"02".repeat(32)).is_some());
    assert_eq!(restored.public_summary(), config.public_summary());
}
#[test]
fn test_mode_and_authenticated_device_name_follow_registry_state() {
    let mut config = parse(&fixture()).unwrap();
    assert!(config.test_mode());
    assert_eq!(
        config.authenticate_device(&"01".repeat(32)),
        Some("Device".into())
    );
    assert_eq!(config.authenticate_device(&"03".repeat(32)), None);
    assert!(config.set_test_mode(false));
    assert!(!config.test_mode());
    assert!(!config.set_test_mode(false));
}
#[test]
fn projection_excludes_credentials() {
    let summary = serde_json::to_string(&parse(&fixture()).unwrap().public_summary()).unwrap();
    assert!(!summary.contains("token"));
    assert!(
        !summary.contains(
            fixture()["paired_devices"][0]["token_sha256"]
                .as_str()
                .unwrap()
        )
    );
}
#[test]
fn accepts_the_full_device_limit_and_unicode_names() {
    let mut value = fixture();
    value["paired_devices"] = serde_json::Value::Array((1..=MAX_PAIRED_DEVICES as u8).map(|index| {
        serde_json::json!({"device_id":format!("00000000-0000-4000-8000-{index:012x}"),"name":"é".repeat(80),"token_sha256":lower_hex(&Sha256::digest([index;32]))})
    }).collect());
    let config = parse(&value).unwrap();
    assert_eq!(
        config.public_summary().paired_devices.len(),
        MAX_PAIRED_DEVICES
    );
    assert!(config.authenticate_device(&"10".repeat(32)).is_some());
    assert!(config.authenticate_device(&"11".repeat(32)).is_none());
    let bytes = config.to_json().unwrap();
    assert!(RegistryConfig::from_json(&bytes).is_ok());
    let mut at_limit = bytes.clone();
    at_limit.resize(MAX_CONFIG_BYTES, b' ');
    assert!(RegistryConfig::from_json(&at_limit).is_ok());
    at_limit.push(b' ');
    assert!(RegistryConfig::from_json(&at_limit).is_err());
}
#[test]
fn rejects_invalid_fields_and_limits_without_exposing_values() {
    for (field, bad) in [
        ("schema_version", serde_json::json!(2.0)),
        ("schema_version", serde_json::json!(true)),
        ("schema_version", serde_json::json!(1)),
        ("schema_version", serde_json::json!(2)),
        ("port", serde_json::json!(0)),
        ("port", serde_json::json!(65536)),
        ("port", serde_json::json!(1.0)),
        ("port", serde_json::json!(true)),
        ("test_mode", serde_json::json!(0)),
        ("host_id", serde_json::json!(HOST.to_uppercase())),
    ] {
        let mut value = fixture();
        value[field] = bad;
        assert!(parse(&value).is_err());
    }
    for (field, bad) in [
        ("device_id", serde_json::json!("invalid private value")),
        ("name", serde_json::json!(" Device")),
        ("name", serde_json::json!("")),
        ("name", serde_json::json!("x".repeat(81))),
        ("name", serde_json::json!("Device\n")),
        ("unknown", serde_json::json!(1)),
        ("token_sha256", serde_json::json!("F".repeat(64))),
    ] {
        let mut value = fixture();
        value["paired_devices"][0][field] = bad;
        assert!(parse(&value).is_err());
    }
    for field in ["device_id", "token_sha256"] {
        let mut value = fixture();
        value["paired_devices"][1][field] = value["paired_devices"][0][field].clone();
        assert!(parse(&value).is_err());
    }
    let mut value = fixture();
    value["paired_devices"] = serde_json::Value::Array((1u8..=17).map(|index| {
        serde_json::json!({"device_id":format!("00000000-0000-4000-8000-{index:012x}"),"name":"Device","token_sha256":lower_hex(&Sha256::digest([index;32]))})
    }).collect());
    assert_eq!(parse(&value).err(), Some("Too many paired devices."));
    let mut value = fixture();
    value["paired_devices"] = serde_json::json!([]);
    assert!(
        parse(&value)
            .unwrap()
            .authenticate_device(&"01".repeat(32))
            .is_none()
    );
    let json = fixture().to_string();
    for bad in [
        json.replace("\"port\":47991", "\"port\":47991,\"port\":47991"),
        json.replace(
            "\"name\":\"Device\"",
            "\"name\":\"Device\",\"name\":\"Device\"",
        ),
        json.replace("\"name\":\"Device\"", "\"name\":\"Device\",\"unknown\":0"),
        json.replace("\"port\":47991", "\"port\":47991,\"unknown\":0"),
        json.replace(",\"test_mode\":true", ""),
    ] {
        assert!(RegistryConfig::from_json(bad.as_bytes()).is_err());
    }
    for bad in [
        b"[]".as_slice(),
        b"null",
        b"{}",
        b"{",
        &[b' '; MAX_CONFIG_BYTES + 1],
    ] {
        assert!(RegistryConfig::from_json(bad).is_err());
    }
}

#[test]
fn token_digest_matches_an_independent_vector_and_rejects_other_forms() {
    let digest =
        decode_hex("630dcd2966c4336691125448bbb25b4ff412a49c732db2c8abc1b8581bd710dd").unwrap();
    assert!(token_digest(TOKEN).is_some_and(|actual| digests_match(&actual, &digest)));
    let actual = token_digest(TOKEN).unwrap();
    assert!(!digests_match(
        &actual,
        &Sha256::digest(TOKEN.as_bytes()).into()
    ));
    assert!(!digests_match(&actual, &[0; 32]));
    for bad in [
        "".to_owned(),
        TOKEN.to_uppercase(),
        format!(" {TOKEN}"),
        TOKEN[..62].to_owned(),
        "g".repeat(64),
    ] {
        assert!(token_digest(&bad).is_none());
    }
}

#[test]
fn accepts_the_fixture_configuration() {
    let config = RegistryConfig::from_json(CONFIG_FIXTURE.as_bytes()).unwrap();
    assert_eq!(config.port(), 47991);
    assert!(!config.test_mode());
    assert_eq!(
        config.host_id().to_string(),
        "03c85317-9d93-4fb3-a04b-91a0e16668c6"
    );
    assert!(
        RegistryConfig::from_json(
            CONFIG_FIXTURE
                .replace("\"test_mode\":false", "\"test_mode\":true")
                .as_bytes()
        )
        .unwrap()
        .test_mode()
    );
}
#[test]
fn rejects_shared_invalid_config_fixture() {
    let invalid = include_str!("../../../../tests/fixtures/host-config/invalid-extra-field.json");
    assert!(RegistryConfig::from_json(invalid.as_bytes()).is_err());
}
#[test]
fn rejects_invalid_stored_types_and_fields() {
    for (from, to) in [
        ("\"schema_version\":3", "\"schema_version\":true"),
        ("\"schema_version\":3", "\"schema_version\":1.0"),
        ("\"schema_version\":3", "\"schema_version\":1"),
        ("\"port\":47991", "\"port\":0"),
        ("\"port\":47991", "\"port\":65536"),
        ("\"port\":47991", "\"port\":\"47991\""),
        ("\"port\":47991", "\"port\":true"),
        ("\"port\":47991", "\"port\":1.0"),
        ("\"test_mode\":false", "\"test_mode\":0"),
        ("\"test_mode\":false", "\"test_mode\":\"false\""),
        ("\"test_mode\":false", ""),
        ("\"port\":47991", "\"port\":47991,\"port\":47991"),
        ("\"port\":47991,", ""),
        ("\"port\":47991", "\"port\":47991,\"token\":\"private\""),
        (
            "03c85317-9d93-4fb3-a04b-91a0e16668c6",
            "03C85317-9D93-4FB3-A04B-91A0E16668C6",
        ),
    ] {
        assert!(
            RegistryConfig::from_json(CONFIG_FIXTURE.replace(from, to).as_bytes()).is_err(),
            "case {from}"
        );
    }
    for bad in [
        b"[]".as_slice(),
        b"null",
        b"{}",
        b"{",
        &[b' '; MAX_CONFIG_BYTES + 1],
    ] {
        assert!(RegistryConfig::from_json(bad).is_err());
    }
}
