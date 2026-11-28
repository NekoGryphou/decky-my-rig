//! A portable fake platform: plain files, a lock file, and network and neighbor
//! answers set by the test.
use super::InputEcho;
use super::{Platform, Settings};
use std::{
    fs::{self, File},
    io::{self, Read},
    net::Ipv4Addr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const DOCUMENT: &str = "pcs.json";

#[derive(Default)]
pub(crate) struct TestPlatform {
    pub(crate) interfaces: Vec<(Ipv4Addr, Ipv4Addr)>,
    pub(crate) neighbors: Vec<(Ipv4Addr, String)>,
    /// How many times input was asked to hide its echo.
    pub(crate) hidden: Arc<std::sync::atomic::AtomicUsize>,
}

pub(crate) fn platform() -> Arc<dyn Platform> {
    Arc::new(TestPlatform::default())
}

struct TestSettings {
    directory: PathBuf,
    lock: Mutex<Option<File>>,
}

impl Platform for TestPlatform {
    fn hide_input(&self) -> Option<Box<dyn InputEcho>> {
        self.hidden
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Some(Box::new(()))
    }
    fn settings_directory(&self) -> Result<PathBuf, &'static str> {
        Err("tests pass the settings directory")
    }
    fn open_settings(&self, directory: &Path, create: bool) -> io::Result<Box<dyn Settings>> {
        if create {
            fs::create_dir_all(directory)?;
        } else if !directory.is_dir() {
            return Err(io::ErrorKind::NotFound.into());
        }
        Ok(Box::new(TestSettings {
            directory: directory.to_owned(),
            lock: Mutex::new(None),
        }))
    }
    fn broadcast_interfaces(&self) -> io::Result<Vec<(Ipv4Addr, Ipv4Addr)>> {
        Ok(self.interfaces.clone())
    }
    fn neighbor_macs(&self, host: Ipv4Addr) -> io::Result<Vec<String>> {
        Ok(self
            .neighbors
            .iter()
            .filter(|(address, _)| *address == host)
            .map(|(_, mac)| mac.clone())
            .collect())
    }
    fn hostname(&self) -> Option<String> {
        Some("test-device".to_owned())
    }
}

impl Settings for TestSettings {
    fn lock(&self) -> io::Result<()> {
        // ? Next to the directory, so tests listing its entries only see the document.
        let mut name = self.directory.clone().into_os_string();
        name.push(".lock");
        let file = File::create(PathBuf::from(name))?;
        file.lock()?;
        *self.lock.lock().unwrap() = Some(file);
        Ok(())
    }
    fn read(&self, limit: usize) -> io::Result<Option<Vec<u8>>> {
        let file = match File::open(self.directory.join(DOCUMENT)) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            other => other?,
        };
        let mut raw = Vec::new();
        file.take(limit as u64 + 1).read_to_end(&mut raw)?;
        if raw.len() > limit {
            return Err(io::ErrorKind::InvalidData.into());
        }
        Ok(Some(raw))
    }
    fn replace(&self, contents: &[u8]) -> io::Result<()> {
        let temporary = self.directory.join(".pcs.tmp");
        fs::write(&temporary, contents)?;
        fs::rename(temporary, self.directory.join(DOCUMENT))
    }
}

/// Takes the directory lock like another process would; dropping the file
/// releases it.
pub(crate) fn lock_held_elsewhere(directory: &Path) -> File {
    let mut name = directory.to_owned().into_os_string();
    name.push(".lock");
    let file = File::create(PathBuf::from(name)).unwrap();
    file.lock().unwrap();
    file
}
