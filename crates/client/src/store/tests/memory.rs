use super::fixtures::{PIN, TOKEN_A, authentication, fixture, input};
use crate::platform::fixtures::platform;
use crate::store::Store;
use std::sync::Arc;
use std::{fs, thread};

#[test]
fn the_encoded_document_is_wiped_when_dropped() {
    use crate::store::utils::encode;
    let pc = fixture()
        .store
        .add(input("PC"), authentication(TOKEN_A, PIN))
        .unwrap();
    // - The bytes hold every token, so they come in a wiping buffer.
    let encoded: zeroize::Zeroizing<Vec<u8>> = encode(&[pc]).unwrap();
    assert!(String::from_utf8_lossy(&encoded).contains(TOKEN_A));
}

#[test]
fn the_list_can_be_inspected_without_copying_it() {
    let fixture = fixture();
    let other = Store::open(&fixture.directory, platform()).unwrap();
    let pc = other
        .add(input("PC"), authentication(TOKEN_A, PIN))
        .unwrap();
    let seen = fixture.store.with_configs(|pcs| pcs.to_vec());
    assert_eq!(seen, vec![pc]);
}

#[test]
fn a_reader_inside_the_platform_does_not_block_a_save_or_undo_it() {
    use crate::store::fixtures::gated;
    use std::{sync::mpsc, time::Duration};
    let fixture = fixture();
    let (gate, platform, reached, release) = gated();
    let reader = Arc::new(Store::open(&fixture.directory, platform).unwrap());
    gate.arm();
    let slow = {
        let reader = reader.clone();
        thread::spawn(move || reader.configs())
    };
    reached.recv().unwrap();
    // - The reader holds the empty list it read, inside the platform.
    let writer = {
        let reader = reader.clone();
        let (done, finished) = mpsc::channel();
        let save = thread::spawn(move || {
            let pc = reader
                .add(input("PC"), authentication(TOKEN_A, PIN))
                .unwrap();
            done.send(()).unwrap();
            pc
        });
        (save, finished)
    };
    let saved_meanwhile = writer.1.recv_timeout(Duration::from_secs(5)).is_ok();
    release.send(()).unwrap();
    slow.join().unwrap();
    let pc = writer.0.join().unwrap();
    assert!(saved_meanwhile, "the save waited for the reader");
    // ! The late reader's stale answer must not replace the saved list: with the file unreadable,
    // ! the last good list is what's left.
    fs::write(fixture.file(), "{").unwrap();
    assert_eq!(reader.configs(), vec![pc]);
}
