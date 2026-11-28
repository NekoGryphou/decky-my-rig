use super::*;
use crate::{paired_devices::MAX_CONFIG_BYTES, security_bundle::MAX_SECURITY_BYTES};
use std::{fs, path::Path};

const HOST: &str = "03c85317-9d93-4fb3-a04b-91a0e16668c6";

fn store_with(bytes: &[u8]) -> (tempfile::TempDir, FileStorage) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("host.json");
    fs::write(&path, bytes).unwrap();
    let store = FileStorage::open(&path).unwrap();
    (dir, store)
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn read_returns_written_bytes() {
    let (_dir, store) = store_with(b"{\"a\":1}");
    assert_eq!(store.read().unwrap(), b"{\"a\":1}");
}

#[test]
fn read_refuses_oversized_file() {
    let (_dir, store) = store_with(&vec![b' '; MAX_CONFIG_BYTES + 1]);
    assert!(store.read().is_err());
}

#[test]
fn open_refuses_missing_file() {
    let dir = tempfile::tempdir().unwrap();
    assert!(FileStorage::open(&dir.path().join("host.json")).is_err());
}

#[test]
fn replace_writes_atomically_and_leaves_no_temp_files() {
    let (dir, store) = store_with(b"old");
    store.replace(b"old", b"new").unwrap();
    assert_eq!(fs::read(dir.path().join("host.json")).unwrap(), b"new");
    assert_eq!(entries(dir.path()), ["host.json"]);
}

#[test]
fn replace_refuses_changed_baseline() {
    let (dir, store) = store_with(b"old");
    fs::write(dir.path().join("host.json"), b"edited elsewhere").unwrap();
    assert_eq!(store.replace(b"old", b"new"), Err(StorageError::Conflict));
    assert_eq!(
        fs::read(dir.path().join("host.json")).unwrap(),
        b"edited elsewhere"
    );
    assert_eq!(entries(dir.path()), ["host.json"]);
}

#[test]
fn load_security_rejects_wrong_host() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(
        dir.path().join(SECURITY_FILE),
        serde_json::to_vec(&serde_json::json!({"schema_version":1,"host_id":HOST,
            "certificate_der":"0102","private_key_pkcs8":"0304"}))
        .unwrap(),
    )
    .unwrap();
    let other = uuid::Uuid::parse_str("22222222-2222-4222-8222-222222222222").unwrap();
    assert!(load_security(dir.path(), other).is_err());
}

#[test]
fn load_security_refuses_missing_and_oversized() {
    let dir = tempfile::tempdir().unwrap();
    let host = uuid::Uuid::parse_str(HOST).unwrap();
    assert!(load_security(dir.path(), host).is_err());
    fs::write(
        dir.path().join(SECURITY_FILE),
        vec![b' '; MAX_SECURITY_BYTES + 1],
    )
    .unwrap();
    assert!(load_security(dir.path(), host).is_err());
}

#[test]
fn replace_reports_unreadable_file_as_io_not_conflict() {
    // A file that cannot be read says nothing about its content, so it must not
    // look like someone else changed it.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("host.json");
    fs::create_dir(&path).unwrap();
    let store = FileStorage { path };
    assert_eq!(store.replace(b"old", b"new"), Err(StorageError::Io));
}

#[test]
fn opening_removes_a_temp_file_a_crashed_save_left_behind() {
    // ! A save that crashes after creating its temp file leaves it in the folder. Nothing else uses
    // ! that name, so opening the store removes it.
    let (dir, _store) = store_with(b"old");
    fs::write(dir.path().join(".tmpQx93Zk"), b"half written").unwrap();
    let reopened = FileStorage::open(&dir.path().join("host.json")).unwrap();
    assert_eq!(entries(dir.path()), ["host.json"]);
    assert_eq!(reopened.read().unwrap(), b"old");
}
