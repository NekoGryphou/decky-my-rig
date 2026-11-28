use super::fixtures::{PIN, TOKEN_A, authentication, fixture, input, rename};
use crate::platform::fixtures::platform;
use crate::store::{PcChanges, Store};
use std::fs;
use uuid::Uuid;

#[test]
fn failed_validation_does_not_mutate_memory_or_disk() {
    let fixture = fixture();
    let pc = fixture
        .store
        .add(input("PC"), authentication(TOKEN_A, PIN))
        .unwrap();
    let before = fs::read(fixture.file()).unwrap();
    let bad_host = PcChanges {
        host: Some("999.1.2.3".to_owned()),
        ..PcChanges::default()
    };
    assert!(fixture.store.update(pc.id, bad_host).is_err());
    assert_eq!(
        fixture.store.update(pc.id, rename("PC\n")).unwrap_err().0,
        "Enter a PC name of 1 to 80 visible characters."
    );
    assert_eq!(fixture.store.configs(), vec![pc]);
    assert_eq!(fs::read(fixture.file()).unwrap(), before);
}

#[test]
fn oversized_document_is_refused_before_writing() {
    let fixture = fixture();
    fixture
        .store
        .add(input("PC"), authentication(TOKEN_A, PIN))
        .unwrap();
    let saved = fs::read_to_string(fixture.file()).unwrap();
    let pc = &saved[saved.find("{\"id\"").unwrap()..saved.len() - 2];
    let id = &pc[7..43];
    let mut pcs = Vec::new();
    let mut size = saved.len();
    while size + pc.len() + 1 < 1024 * 1024 {
        pcs.push(pc.replace(id, &Uuid::new_v4().to_string()));
        size += pc.len() + 1;
    }
    let raw = format!("{{\"schema_version\":2,\"pcs\":[{}]}}", pcs.join(","));
    fs::write(fixture.file(), &raw).unwrap();
    let store = Store::open(&fixture.directory, platform()).unwrap();
    let before = store.configs();
    let error = store
        .add(input(&"é".repeat(80)), authentication(TOKEN_A, PIN))
        .unwrap_err();
    assert_eq!(error.0, "PC configuration exceeds the storage limit.");
    assert_eq!(store.configs(), before);
    assert_eq!(fs::read_to_string(fixture.file()).unwrap(), raw);
    assert_eq!(fixture.entries(), ["pcs.json"]);
}

#[test]
fn corrupt_storage_is_refused_not_reset() {
    let fixture = fixture();
    fixture
        .store
        .add(input("PC"), authentication(TOKEN_A, PIN))
        .unwrap();
    let good = fs::read_to_string(fixture.file()).unwrap();
    let pc_start = good.find("{\"id\"").unwrap();
    let pc = &good[pc_start..good.len() - 2];
    let mut variants = vec![
        "{".to_owned(),
        "[]".to_owned(),
        "null".to_owned(),
        good.replacen(
            "{\"schema_version\":2",
            "{\"unexpected\":1,\"schema_version\":2",
            1,
        ),
        good.replacen("\"schema_version\":2", "\"schema_version\":true", 1),
        good.replacen("\"schema_version\":2", "\"schema_version\":2.0", 1),
        good.replacen("\"schema_version\":2", "\"schema_version\":1", 1),
        good.replacen("\"schema_version\":2", "\"schema_version\":3", 1),
        good.replacen(pc, &format!("{pc},{pc}"), 1),
        "{\"schema_version\":2,\"schema_version\":2,\"pcs\":[]}".to_owned(),
        good.replacen(
            "\"token\":",
            "\"token\":\"synthetic-private\",\"token\":",
            1,
        ),
    ];
    for (from, to) in [
        ("\"host\":\"192.168.1.40\"", "\"host\":\"PC.EXAMPLE\""),
        ("\"port\":47991", "\"port\":\"47991\""),
        ("\"port\":47991", "\"port\":true"),
        ("\"port\":47991", "\"port\":0"),
        ("\"mac\":\"AA:BB:CC:DD:EE:FF\"", "\"mac\":\"AABBCCDDEEFF\""),
        ("\"id\":\"", "\"id\":\" "),
        ("\"name\":\"PC\"", "\"name\":\" PC\""),
        ("\"mac\":", "\"extra\":1,\"mac\":"),
        ("\"token\":\"abab", "\"token\":\"ABAB"),
        ("\"tls_pin\":\"1212", "\"tls_pin\":\"invalid"),
        (
            ",\"tls_pin\":\"1212121212121212121212121212121212121212121212121212121212121212\"",
            "",
        ),
        ("\"tls_pin\":", "\"extra\":1,\"tls_pin\":"),
    ] {
        assert!(good.contains(from), "{from}");
        variants.push(good.replacen(from, to, 1));
    }
    for raw in variants {
        fs::write(fixture.file(), &raw).unwrap();
        let error = Store::open(&fixture.directory, platform()).err().unwrap();
        assert_eq!(
            error.0, "Cannot load PC configuration; existing data was not reset.",
            "{raw}"
        );
        assert_eq!(fs::read_to_string(fixture.file()).unwrap(), raw);
    }
}
