use super::{Authentication, PcChanges, PcConfig, PcInput, Store};
use crate::{platform::fixtures::platform, testing::StoreFixture};
use std::{fs, net::Ipv4Addr, sync::Arc, thread};
use uuid::Uuid;

// ! Fake credentials for tests, never used on a real host.
const TOKEN_A: &str = "abababababababababababababababababababababababababababababababab";
const TOKEN_B: &str = "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd";
const PIN: [u8; 32] = [0x12; 32];
const HOST_ID: &str = "03c85317-9d93-4fb3-a04b-91a0e16668c6";
const SAVED_FILE: &[u8] = include_bytes!("../../../../tests/fixtures/store/pcs.json");

fn authentication(token: &str, pin: [u8; 32]) -> Authentication {
    Authentication {
        host_id: Uuid::parse_str(HOST_ID).unwrap(),
        token: token.to_owned().into(),
        tls_pin: pin,
    }
}

fn input(name: &str) -> PcInput {
    PcInput {
        name: name.to_owned(),
        host: " 192.168.1.40 ".to_owned(),
        port: None,
        mac: "aabbccddeeff".to_owned(),
    }
}

fn rename(name: &str) -> PcChanges {
    PcChanges {
        name: Some(name.to_owned()),
        ..PcChanges::default()
    }
}

fn fixture() -> StoreFixture {
    StoreFixture::new(platform())
}

#[test]
fn input_is_normalized_and_debug_hides_credentials() {
    let fixture = fixture();
    let pc = fixture
        .store
        .add(input(" Gaming PC "), authentication(TOKEN_A, PIN))
        .unwrap();
    assert_eq!(
        (pc.name.as_str(), pc.host, pc.port, pc.mac.as_str()),
        (
            "Gaming PC",
            Ipv4Addr::new(192, 168, 1, 40),
            47991,
            "AA:BB:CC:DD:EE:FF"
        )
    );
    let debug = format!("{pc:?}");
    assert!(
        !debug.contains(TOKEN_A) && !debug.contains("1212"),
        "{debug}"
    );
}

#[test]
fn first_run_and_multi_pc_roundtrip() {
    let fixture = fixture();
    assert!(fixture.store.configs().is_empty());
    assert!(!fixture.file().exists());
    let first = fixture
        .store
        .add(input("Gaming PC"), authentication(TOKEN_A, PIN))
        .unwrap();
    let mut other = input("Gaming PC");
    other.port = Some(" 50000 ".to_owned());
    let second = fixture
        .store
        .add(other, authentication(TOKEN_B, PIN))
        .unwrap();
    assert_ne!(first.id, second.id);
    assert_eq!(second.port, 50000);
    assert_eq!(
        Store::open(&fixture.directory, platform())
            .unwrap()
            .configs(),
        vec![first, second]
    );
    assert_eq!(fixture.entries(), ["pcs.json"]);
}

#[test]
fn a_saved_file_is_read_and_rewritten_byte_for_byte() {
    let fixture = fixture();
    fs::write(fixture.file(), SAVED_FILE).unwrap();
    let store = Store::open(&fixture.directory, platform()).unwrap();
    let pcs = store.configs();
    assert_eq!(pcs.len(), 2);
    assert_eq!(pcs[0].name, "Salon – PC 🎮");
    assert_eq!(pcs[1].name, "Bureau \"x\" \\ y");
    assert_eq!(pcs[1].port, 50000);
    assert_eq!(pcs[0].authentication.tls_pin, PIN);
    // - Saving it again writes exactly the same file.
    store.update(pcs[0].id, rename("Other")).unwrap();
    assert_ne!(fs::read(fixture.file()).unwrap(), SAVED_FILE);
    store.update(pcs[0].id, rename("Salon – PC 🎮")).unwrap();
    assert_eq!(fs::read(fixture.file()).unwrap(), SAVED_FILE);
}

#[test]
fn edits_credentials_and_deletion_are_per_pc() {
    let fixture = fixture();
    let first = fixture
        .store
        .add(input("A"), authentication(TOKEN_A, PIN))
        .unwrap();
    let second = fixture
        .store
        .add(input("B"), authentication(TOKEN_B, PIN))
        .unwrap();
    let updated = fixture
        .store
        .update(
            first.id,
            PcChanges {
                name: Some("Renamed".to_owned()),
                host: Some("192.168.1.42".to_owned()),
                port: Some("50001".to_owned()),
                ..PcChanges::default()
            },
        )
        .unwrap();
    assert_eq!((updated.id, updated.port), (first.id, 50001));
    assert_eq!(*updated.authentication.token, TOKEN_A);
    let token = "ef".repeat(32);
    let updated = fixture
        .store
        .update(
            first.id,
            PcChanges {
                authentication: Some(authentication(&token, PIN)),
                ..PcChanges::default()
            },
        )
        .unwrap();
    assert_eq!(*updated.authentication.token, token);
    assert_eq!(fixture.store.configs()[1], second);
    fixture.store.delete(first.id).unwrap();
    assert_eq!(
        Store::open(&fixture.directory, platform())
            .unwrap()
            .configs(),
        vec![second]
    );
    assert!(!fs::read_to_string(fixture.file()).unwrap().contains(&token));
    assert_eq!(
        fixture
            .store
            .update(first.id, rename("deleted"))
            .unwrap_err()
            .0,
        "PC was not found."
    );
    assert_eq!(
        fixture.store.delete(first.id).unwrap_err().0,
        "PC was not found."
    );
}

#[test]
fn same_host_update_needs_the_same_host_identity() {
    let fixture = fixture();
    let observed = fixture
        .store
        .add(input("PC"), authentication(TOKEN_A, PIN))
        .unwrap();
    fixture
        .store
        .update(observed.id, rename("Renamed"))
        .unwrap();
    let change = PcChanges {
        authentication: Some(authentication(TOKEN_B, PIN)),
        ..PcChanges::default()
    };
    let updated = fixture
        .store
        .update_same_host(&observed, change.clone())
        .unwrap()
        .unwrap();
    assert_eq!(
        (updated.name.as_str(), updated.authentication.token.as_str()),
        ("Renamed", TOKEN_B)
    );
    fixture
        .store
        .update(
            observed.id,
            PcChanges {
                authentication: Some(authentication(TOKEN_A, [0x34; 32])),
                ..PcChanges::default()
            },
        )
        .unwrap();
    assert_eq!(
        fixture.store.update_same_host(&observed, change).unwrap(),
        None
    );
    assert_eq!(*fixture.store.configs()[0].authentication.token, TOKEN_A);
}

#[test]
fn update_if_current_refuses_a_changed_or_removed_record() {
    let fixture = fixture();
    let observed = fixture
        .store
        .add(input("PC"), authentication(TOKEN_A, PIN))
        .unwrap();
    let current = fixture
        .store
        .update_if_current(&observed, rename("Two"))
        .unwrap()
        .unwrap();
    assert_eq!(
        fixture
            .store
            .update_if_current(&observed, rename("Three"))
            .unwrap(),
        None
    );
    fixture.store.delete(current.id).unwrap();
    assert_eq!(
        fixture
            .store
            .update_if_current(&current, rename("Four"))
            .unwrap(),
        None
    );
    assert!(fixture.store.configs().is_empty());
}

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

#[test]
fn serialized_adds_have_no_lost_updates() {
    let fixture = fixture();
    let store = Arc::new(fixture.store);
    let workers: Vec<_> = (0..24)
        .map(|index| {
            let store = Arc::clone(&store);
            thread::spawn(move || {
                store
                    .add(input(&index.to_string()), authentication(TOKEN_A, PIN))
                    .unwrap()
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let persisted = Store::open(&fixture.directory, platform())
        .unwrap()
        .configs();
    assert_eq!(persisted.len(), 24);
    let mut names: Vec<_> = persisted
        .iter()
        .map(|pc| pc.name.parse::<u32>().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(names, (0..24).collect::<Vec<_>>());
}

#[test]
fn concurrent_partial_edits_merge_against_the_current_pc() {
    let fixture = fixture();
    let pc = fixture
        .store
        .add(input("original"), authentication(TOKEN_A, PIN))
        .unwrap();
    let store = Arc::new(fixture.store);
    let edits = [
        PcChanges {
            name: Some("renamed".to_owned()),
            ..PcChanges::default()
        },
        PcChanges {
            host: Some("192.168.1.8".to_owned()),
            ..PcChanges::default()
        },
        PcChanges {
            port: Some("50001".to_owned()),
            ..PcChanges::default()
        },
        PcChanges {
            mac: Some("02:11:22:33:44:55".to_owned()),
            ..PcChanges::default()
        },
    ];
    let workers: Vec<_> = edits
        .into_iter()
        .map(|edit| {
            let store = Arc::clone(&store);
            thread::spawn(move || store.update(pc.id, edit).unwrap())
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let last: PcConfig = Store::open(&fixture.directory, platform())
        .unwrap()
        .configs()
        .remove(0);
    assert_eq!(
        (
            last.id,
            last.name.as_str(),
            last.host,
            last.port,
            last.mac.as_str()
        ),
        (
            pc.id,
            "renamed",
            Ipv4Addr::new(192, 168, 1, 8),
            50001,
            "02:11:22:33:44:55"
        )
    );
    assert_eq!(last.authentication, pc.authentication);
}

#[test]
fn a_removed_pc_stays_removed_across_processes() {
    let fixture = fixture();
    let pc = fixture
        .store
        .add(input("PC"), authentication(TOKEN_A, PIN))
        .unwrap();
    let other = Store::open(&fixture.directory, platform()).unwrap();
    fixture.store.delete(pc.id).unwrap();
    // - The second store still has the PC in memory, but its writes start from
    //   disk.
    assert_eq!(
        other.update(pc.id, rename("stale")).unwrap_err().0,
        "PC was not found."
    );
    let added = other
        .add(input("New"), authentication(TOKEN_B, PIN))
        .unwrap();
    assert_eq!(
        Store::open(&fixture.directory, platform())
            .unwrap()
            .configs(),
        vec![added.clone()]
    );
    assert_eq!(other.configs(), vec![added]);
}
