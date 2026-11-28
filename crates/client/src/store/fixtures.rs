// ! A platform whose settings fail in one chosen step, so tests can tell the causes apart.
use crate::platform::{Platform, Settings, fixtures::platform};
use std::{
    io,
    net::Ipv4Addr,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Step {
    Open,
    Lock,
    Read,
    Replace,
}

pub(super) struct Failing {
    inner: Arc<dyn Platform>,
    step: Step,
    kind: io::ErrorKind,
    /// Whether the opening load (`create` set) fails too, or only later saves.
    at_open: bool,
}

/// A platform whose `step` fails with `kind` during saves; `Store::open` works.
pub(super) fn failing(step: Step, kind: io::ErrorKind) -> Arc<dyn Platform> {
    Arc::new(Failing {
        inner: platform(),
        step,
        kind,
        at_open: false,
    })
}

/// A platform whose read fails with `kind` already when the store opens.
pub(super) fn unreadable(kind: io::ErrorKind) -> Arc<dyn Platform> {
    Arc::new(Failing {
        inner: platform(),
        step: Step::Read,
        kind,
        at_open: true,
    })
}

struct FailingSettings {
    inner: Box<dyn Settings>,
    step: Option<Step>,
    kind: io::ErrorKind,
}

impl FailingSettings {
    fn check(&self, step: Step) -> io::Result<()> {
        if self.step == Some(step) {
            return Err(self.kind.into());
        }
        Ok(())
    }
}

impl Platform for Failing {
    fn hide_input(&self) -> Option<Box<dyn crate::platform::InputEcho>> {
        None
    }
    fn settings_directory(&self) -> Result<PathBuf, &'static str> {
        self.inner.settings_directory()
    }
    fn open_settings(&self, directory: &Path, create: bool) -> io::Result<Box<dyn Settings>> {
        let inner = self.inner.open_settings(directory, create)?;
        let armed = !create || self.at_open;
        if self.step == Step::Open && armed {
            return Err(self.kind.into());
        }
        Ok(Box::new(FailingSettings {
            inner,
            step: armed.then_some(self.step),
            kind: self.kind,
        }))
    }
    fn broadcast_interfaces(&self) -> io::Result<Vec<(Ipv4Addr, Ipv4Addr)>> {
        self.inner.broadcast_interfaces()
    }
    fn neighbor_macs(&self, host: Ipv4Addr) -> io::Result<Vec<String>> {
        self.inner.neighbor_macs(host)
    }
    fn hostname(&self) -> Option<String> {
        self.inner.hostname()
    }
}

impl Settings for FailingSettings {
    fn lock(&self) -> io::Result<()> {
        self.check(Step::Lock)?;
        self.inner.lock()
    }
    fn read(&self, limit: usize) -> io::Result<Option<Vec<u8>>> {
        self.check(Step::Read)?;
        self.inner.read(limit)
    }
    fn replace(&self, contents: &[u8]) -> io::Result<()> {
        self.check(Step::Replace)?;
        self.inner.replace(contents)
    }
}

/// Holds the first read after `arm` until the test releases it, so a test can
/// run other calls while one reader is inside the platform.
pub(super) struct Gate {
    armed: std::sync::atomic::AtomicBool,
    entered: std::sync::mpsc::Sender<()>,
    release: Mutex<std::sync::mpsc::Receiver<()>>,
}

/// The gate, the platform using it, the signal that a read reached the gate and
/// the handle that releases it.
pub(super) fn gated() -> (
    Arc<Gate>,
    Arc<dyn Platform>,
    std::sync::mpsc::Receiver<()>,
    std::sync::mpsc::Sender<()>,
) {
    let (entered, reached) = std::sync::mpsc::channel();
    let (open, release) = std::sync::mpsc::channel();
    let gate = Arc::new(Gate {
        armed: std::sync::atomic::AtomicBool::new(false),
        entered,
        release: Mutex::new(release),
    });
    let platform = Arc::new(Gated {
        inner: platform(),
        gate: gate.clone(),
    });
    (gate, platform, reached, open)
}

impl Gate {
    pub(super) fn arm(&self) {
        self.armed.store(true, std::sync::atomic::Ordering::SeqCst);
    }
    fn pass(&self) {
        if self.armed.swap(false, std::sync::atomic::Ordering::SeqCst) {
            let _ = self.entered.send(());
            let _ = self.release.lock().unwrap().recv();
        }
    }
}

struct Gated {
    inner: Arc<dyn Platform>,
    gate: Arc<Gate>,
}

struct GatedSettings {
    inner: Box<dyn Settings>,
    gate: Arc<Gate>,
}

impl Platform for Gated {
    fn hide_input(&self) -> Option<Box<dyn crate::platform::InputEcho>> {
        None
    }
    fn settings_directory(&self) -> Result<PathBuf, &'static str> {
        self.inner.settings_directory()
    }
    fn open_settings(&self, directory: &Path, create: bool) -> io::Result<Box<dyn Settings>> {
        Ok(Box::new(GatedSettings {
            inner: self.inner.open_settings(directory, create)?,
            gate: self.gate.clone(),
        }))
    }
    fn broadcast_interfaces(&self) -> io::Result<Vec<(Ipv4Addr, Ipv4Addr)>> {
        self.inner.broadcast_interfaces()
    }
    fn neighbor_macs(&self, host: Ipv4Addr) -> io::Result<Vec<String>> {
        self.inner.neighbor_macs(host)
    }
    fn hostname(&self) -> Option<String> {
        self.inner.hostname()
    }
}

impl Settings for GatedSettings {
    fn lock(&self) -> io::Result<()> {
        self.inner.lock()
    }
    fn read(&self, limit: usize) -> io::Result<Option<Vec<u8>>> {
        let raw = self.inner.read(limit);
        // - The gate sits after the read, so the reader holds a stale answer.
        self.gate.pass();
        raw
    }
    fn replace(&self, contents: &[u8]) -> io::Result<()> {
        self.inner.replace(contents)
    }
}
