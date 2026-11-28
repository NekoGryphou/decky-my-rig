use super::{remove_leftover_documents, temporary_document_name};
use std::fs;

#[test]
fn only_leftover_temporary_documents_are_removed() {
    let directory = tempfile::tempdir().unwrap();
    let leftover = directory.path().join(temporary_document_name());
    for name in [
        leftover.file_name().unwrap().to_str().unwrap(),
        ".pcs-1.tmp",
    ] {
        fs::write(directory.path().join(name), "tokens").unwrap();
    }
    for name in ["pcs.json", ".pcs.lock", "notes.tmp"] {
        fs::write(directory.path().join(name), "keep").unwrap();
    }
    remove_leftover_documents(directory.path());
    let mut left: Vec<_> = fs::read_dir(directory.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    left.sort();
    assert_eq!(left, [".pcs.lock", "notes.tmp", "pcs.json"]);
}

#[test]
fn a_missing_directory_has_no_leftovers() {
    let directory = tempfile::tempdir().unwrap();
    remove_leftover_documents(&directory.path().join("missing"));
}
