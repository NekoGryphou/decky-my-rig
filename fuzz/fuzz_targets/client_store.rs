//! Any saved-PC document either opens or is refused; it never panics.
#![no_main]
use libfuzzer_sys::fuzz_target;
use myrig_client::testing::StoreFixture;
use myrig_client_linux::Linux;
use std::{fs::OpenOptions, io::Write, os::unix::fs::OpenOptionsExt, sync::Arc};

fuzz_target!(|data: &[u8]| {
    let fixture = StoreFixture::new(Arc::new(Linux));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(fixture.file())
        .unwrap();
    file.write_all(data).unwrap();
    drop(file);
    if let Ok(store) = fixture.open(&fixture.directory) {
        let _ = store.configs();
    }
});
