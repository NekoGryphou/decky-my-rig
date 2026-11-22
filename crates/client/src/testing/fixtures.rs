use crate::{
    platform::Platform,
    store::{Authentication, PcInput, Store},
};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};
use tempfile::TempDir;

/// A fake credential, never used on a real host.
pub const TOKEN: &str = "abababababababababababababababababababababababababababababababab";

pub fn authentication() -> Authentication {
    Authentication {
        host_id: uuid::Uuid::from_u128(0x03c8_5317_9d93_4fb3_a04b_91a0_e166_68c6),
        token: TOKEN.to_owned().into(),
        tls_pin: [0x12; 32],
    }
}

pub fn input(name: &str) -> PcInput {
    PcInput {
        name: name.to_owned(),
        host: " 192.168.1.40 ".to_owned(),
        port: None,
        mac: "aabbccddeeff".to_owned(),
    }
}

/// A store in a fresh `settings` directory on `platform`.
pub struct StoreFixture {
    _temporary: TempDir,
    pub directory: PathBuf,
    pub store: Store,
    platform: Arc<dyn Platform>,
}

impl StoreFixture {
    pub fn new(platform: Arc<dyn Platform>) -> Self {
        let temporary = TempDir::new().expect("a temporary directory");
        let directory = temporary.path().join("settings");
        let store = Store::open(&directory, platform.clone()).expect("a new store");
        Self {
            _temporary: temporary,
            directory,
            store,
            platform,
        }
    }
    /// Another handle on `directory`, like a second process would open.
    pub fn open(&self, directory: &Path) -> Result<Store, &'static str> {
        Store::open(directory, self.platform.clone()).map_err(|error| error.0)
    }
    pub fn file(&self) -> PathBuf {
        self.directory.join("pcs.json")
    }
    /// Checks that a save deletes the temp files left by killed saves, each of
    /// which holds every token.
    pub fn assert_saves_remove_leftovers(&self) {
        fs::write(self.directory.join(".pcs-0123abcd.tmp"), "stale").expect("a leftover");
        self.store
            .add(input("PC"), authentication())
            .expect("a save");
        let leftovers: Vec<String> = self
            .entries()
            .into_iter()
            .filter(|name| crate::platform::is_temporary_document(name))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }
    pub fn entries(&self) -> Vec<String> {
        let mut names: Vec<_> = fs::read_dir(&self.directory)
            .expect("the settings directory")
            .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
            .collect();
        names.sort();
        names
    }
}
