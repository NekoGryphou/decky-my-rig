use super::fixtures::{PIN, TOKEN_A, TOKEN_B, authentication, fixture, input, rename};
use crate::platform::fixtures::platform;
use crate::store::{PcChanges, Store};
use std::fs;

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
