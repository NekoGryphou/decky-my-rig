use super::fixtures::{PIN, TOKEN_A, authentication, fixture, input};
use crate::platform::fixtures::platform;
use crate::store::Store;
use std::fs;
use uuid::Uuid;

#[test]
fn reads_see_another_handles_writes() {
    let fixture = fixture();
    let other = Store::open(&fixture.directory, platform()).unwrap();
    let added = other
        .add(input("From the CLI"), authentication(TOKEN_A, PIN))
        .unwrap();
    assert_eq!(fixture.store.configs(), vec![added.clone()]);
    other.delete(added.id).unwrap();
    assert!(fixture.store.configs().is_empty());
}

#[test]
fn an_unreadable_file_keeps_the_last_good_list() {
    let fixture = fixture();
    let pc = fixture
        .store
        .add(input("PC"), authentication(TOKEN_A, PIN))
        .unwrap();
    fs::write(fixture.file(), "{").unwrap();
    assert_eq!(fixture.store.configs(), vec![pc]);
}

#[test]
fn one_pc_is_found_by_id_from_what_is_on_disk() {
    let fixture = fixture();
    let other = Store::open(&fixture.directory, platform()).unwrap();
    let pc = other
        .add(input("PC"), authentication(TOKEN_A, PIN))
        .unwrap();
    assert_eq!(fixture.store.get(pc.id), Some(pc.clone()));
    assert_eq!(fixture.store.get(Uuid::new_v4()), None);
    other.delete(pc.id).unwrap();
    assert_eq!(fixture.store.get(pc.id), None);
}
