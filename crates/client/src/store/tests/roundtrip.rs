use super::fixtures::{PIN, SAVED_FILE, TOKEN_A, TOKEN_B, authentication, fixture, input, rename};
use crate::platform::fixtures::platform;
use crate::store::Store;
use std::fs;
use std::net::Ipv4Addr;

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
