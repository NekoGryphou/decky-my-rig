//! A fake environment: a clock you can set, host and wake results, call
//! counters, and an optional gate that holds wakes until released.
use super::{Controller, Environment};
use crate::{
    host_client::{HostError, ShutdownResult, Status},
    network::WakeError,
    shared::PublicError,
    store::{Authentication, PcConfig, PcInput, Store},
};
use myrig_protocol::lan::HostState;
use std::{
    net::Ipv4Addr,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};
use tempfile::TempDir;
use tokio::sync::Semaphore;
use uuid::Uuid;

pub(crate) const HOST_ID: &str = "03c85317-9d93-4fb3-a04b-91a0e16668c6";

pub(crate) struct FakeEnvironment {
    pub(crate) now: Mutex<Instant>,
    pub(crate) status: Mutex<Result<Status, HostError>>,
    pub(crate) status_delay: Duration,
    pub(crate) shutdown: Mutex<Result<ShutdownResult, HostError>>,
    pub(crate) unpair: Mutex<Result<(), HostError>>,
    pub(crate) wake: Mutex<Result<(), WakeError>>,
    /// When set, each wake waits for one permit.
    pub(crate) wake_gate: Option<Arc<Semaphore>>,
    pub(crate) calls: Calls,
}

#[derive(Default)]
pub(crate) struct Calls {
    pub(crate) status: AtomicUsize,
    pub(crate) shutdown: AtomicUsize,
    pub(crate) unpair: AtomicUsize,
    pub(crate) wake: AtomicUsize,
    pub(crate) active_status: AtomicUsize,
    pub(crate) peak_status: AtomicUsize,
}

pub(crate) fn online() -> Result<Status, HostError> {
    Ok(Status {
        host_id: Uuid::parse_str(HOST_ID).unwrap(),
        state: HostState::Online,
    })
}

impl Default for FakeEnvironment {
    fn default() -> Self {
        Self {
            now: Mutex::new(Instant::now()),
            status: Mutex::new(online()),
            status_delay: Duration::ZERO,
            shutdown: Mutex::new(Ok(ShutdownResult { test: false })),
            unpair: Mutex::new(Ok(())),
            wake: Mutex::new(Ok(())),
            wake_gate: None,
            calls: Calls::default(),
        }
    }
}

impl FakeEnvironment {
    pub(crate) fn advance(&self, by: Duration) {
        *self.now.lock().unwrap() += by;
    }
    pub(crate) fn count(counter: &AtomicUsize) -> usize {
        counter.load(Ordering::SeqCst)
    }
}

impl Environment for FakeEnvironment {
    fn now(&self) -> Instant {
        *self.now.lock().unwrap()
    }
    async fn status(&self, _pc: &PcConfig) -> Result<Status, HostError> {
        self.calls.status.fetch_add(1, Ordering::SeqCst);
        let active = self.calls.active_status.fetch_add(1, Ordering::SeqCst) + 1;
        self.calls.peak_status.fetch_max(active, Ordering::SeqCst);
        tokio::time::sleep(self.status_delay).await;
        self.calls.active_status.fetch_sub(1, Ordering::SeqCst);
        *self.status.lock().unwrap()
    }
    async fn shutdown(
        &self,
        _pc: &PcConfig,
        _request_id: Uuid,
    ) -> Result<ShutdownResult, HostError> {
        self.calls.shutdown.fetch_add(1, Ordering::SeqCst);
        *self.shutdown.lock().unwrap()
    }
    async fn unpair(&self, _pc: &PcConfig) -> Result<(), HostError> {
        self.calls.unpair.fetch_add(1, Ordering::SeqCst);
        *self.unpair.lock().unwrap()
    }
    async fn wake(&self, _pc: &PcConfig) -> Result<(), WakeError> {
        self.calls.wake.fetch_add(1, Ordering::SeqCst);
        if let Some(gate) = &self.wake_gate {
            gate.acquire().await.unwrap().forget();
        }
        *self.wake.lock().unwrap()
    }
    fn detect_mac(&self, _host: Ipv4Addr) -> Result<String, PublicError> {
        Ok("02:00:00:00:00:01".to_owned())
    }
}

pub(crate) struct Fixture {
    _directory: TempDir,
    pub(crate) store: Arc<Store>,
    pub(crate) controller: Arc<Controller<FakeEnvironment>>,
}

pub(crate) fn fixture(environment: FakeEnvironment) -> Fixture {
    let directory = TempDir::new().unwrap();
    let store = Arc::new(
        Store::open(
            &directory.path().join("settings"),
            crate::platform::fixtures::platform(),
        )
        .unwrap(),
    );
    let controller = Arc::new(Controller::new(store.clone(), environment));
    Fixture {
        _directory: directory,
        store,
        controller,
    }
}

impl Fixture {
    pub(crate) fn add(&self, name: &str) -> PcConfig {
        self.store
            .add(
                PcInput {
                    name: name.to_owned(),
                    host: "192.168.1.40".to_owned(),
                    port: None,
                    mac: "02:00:00:00:00:01".to_owned(),
                },
                Authentication {
                    host_id: Uuid::parse_str(HOST_ID).unwrap(),
                    token: "ab".repeat(32).into(),
                    tls_pin: [0x12; 32],
                },
            )
            .unwrap()
    }
    pub(crate) fn environment(&self) -> &FakeEnvironment {
        &self.controller.environment
    }
}
