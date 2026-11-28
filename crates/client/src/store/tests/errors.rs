use super::fixtures::{fixture, save_error};
use crate::platform::fixtures::platform;
use crate::store::Store;
use crate::store::fixtures::{Step, unreadable};
use std::fs;
use std::io::ErrorKind;

#[test]
fn each_failed_step_of_a_save_has_its_own_message() {
    let messages = [
        save_error(Step::Open, ErrorKind::NotFound),
        save_error(Step::Open, ErrorKind::PermissionDenied),
        save_error(Step::Lock, ErrorKind::Other),
        save_error(Step::Read, ErrorKind::Other),
        save_error(Step::Read, ErrorKind::InvalidData),
        save_error(Step::Replace, ErrorKind::Other),
    ];
    for (index, message) in messages.iter().enumerate() {
        assert!(!messages[..index].contains(message), "{index}: {message}");
    }
}

#[test]
fn opening_tells_a_permission_problem_from_an_unreadable_file() {
    let fixture = fixture();
    let open = |kind| {
        Store::open(&fixture.directory, unreadable(kind))
            .err()
            .unwrap()
            .0
    };
    let permission = open(ErrorKind::PermissionDenied);
    let unreadable = open(ErrorKind::Other);
    let oversized = open(ErrorKind::InvalidData);
    fs::write(fixture.file(), "{").unwrap();
    let corrupt = Store::open(&fixture.directory, platform()).err().unwrap().0;
    let all = [permission, unreadable, oversized, corrupt];
    for (index, message) in all.iter().enumerate() {
        assert!(!all[..index].contains(message), "{index}: {message}");
    }
    assert_eq!(oversized, "PC configuration exceeds the storage limit.");
    assert_eq!(
        corrupt,
        "Cannot load PC configuration; existing data was not reset."
    );
}
