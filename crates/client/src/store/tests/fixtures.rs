use crate::platform::fixtures::platform;
use crate::store::fixtures::{Step, failing};
use crate::store::{Authentication, PcChanges, PcInput, Store};
use crate::testing::StoreFixture;
use std::io::ErrorKind;
use uuid::Uuid;

pub(super) const TOKEN_A: &str = "abababababababababababababababababababababababababababababababab";
pub(super) const TOKEN_B: &str = "cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd";
pub(super) const PIN: [u8; 32] = [0x12; 32];
pub(super) const HOST_ID: &str = "03c85317-9d93-4fb3-a04b-91a0e16668c6";
pub(super) const SAVED_FILE: &[u8] = include_bytes!("../../../../../tests/fixtures/store/pcs.json");

pub(super) fn authentication(token: &str, pin: [u8; 32]) -> Authentication {
    Authentication {
        host_id: Uuid::parse_str(HOST_ID).unwrap(),
        token: token.to_owned().into(),
        tls_pin: pin,
    }
}

pub(super) fn input(name: &str) -> PcInput {
    PcInput {
        name: name.to_owned(),
        host: " 192.168.1.40 ".to_owned(),
        port: None,
        mac: "aabbccddeeff".to_owned(),
    }
}

pub(super) fn rename(name: &str) -> PcChanges {
    PcChanges {
        name: Some(name.to_owned()),
        ..PcChanges::default()
    }
}

pub(super) fn fixture() -> StoreFixture {
    StoreFixture::new(platform())
}

/// The message a save through a platform failing at `step` shows.
pub(super) fn save_error(step: Step, kind: ErrorKind) -> &'static str {
    let fixture = fixture();
    let store = Store::open(&fixture.directory, failing(step, kind)).unwrap();
    store
        .add(input("PC"), authentication(TOKEN_A, PIN))
        .unwrap_err()
        .0
}
