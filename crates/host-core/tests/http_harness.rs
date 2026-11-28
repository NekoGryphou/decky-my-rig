//! Only a test harness: never an application binary or a real power option.
use myrig_host_core::{
    host::{Host, PersistError, RegistryError},
    http::serve_secure_tls,
    paired_devices::RegistryConfig,
    shutdown::{Core, Power, PowerError},
};
use std::{
    io::{self, Write},
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::{net::TcpListener, sync::oneshot};

struct Recorder {
    calls: Arc<AtomicUsize>,
    refuse: bool,
}
impl Power for Recorder {
    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.refuse {
            Err(PowerError { code: 5 })
        } else {
            Ok(())
        }
    }
    fn cancel_shutdown(&self) -> Result<(), PowerError> {
        Ok(())
    }
}

/// Only for tests, not a real atomic writer.
fn file_persist(path: String) -> myrig_host_core::host::PairingPersistence {
    Arc::new(move |bytes| {
        if std::env::var_os("MY_RIG_TEST_PAIR_SAVE_FAIL").is_some() {
            return Err(PersistError::Unchanged);
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(&path)
            .map_err(|_| PersistError::RecoveryRequired)?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| PersistError::RecoveryRequired)
    })
}

#[tokio::test(flavor = "current_thread")]
#[ignore = "launched by independent Python socket tests with isolated configuration"]
async fn serve_harness() {
    let config_path = std::env::var("MY_RIG_TEST_CONFIG").expect("test config path required");
    let config = RegistryConfig::from_json(
        &std::fs::read(Path::new(&config_path)).expect("readable isolated test config"),
    )
    .expect("valid isolated test config required");
    let calls = Arc::new(AtomicUsize::new(0));
    let core = Core::new(Box::new(Recorder {
        calls: calls.clone(),
        refuse: std::env::var_os("MY_RIG_TEST_REFUSE").is_some(),
    }));
    let host = Arc::new(Host::new(
        config,
        core,
        "Fixture PC".into(),
        file_persist(config_path.clone()),
    ));
    if std::env::var_os("MY_RIG_TEST_SECURE_PAIR").is_some() {
        let certificate = std::env::var("MY_RIG_TEST_APPROVED_TLS_CERT")
            .or_else(|_| std::env::var("MY_RIG_TEST_TLS_CERT"))
            .expect("isolated approved certificate required");
        host.open_secure_pairing(
            host.pairing_approval(),
            "New Steam Deck",
            "12345678",
            [9; 32],
            &std::fs::read(certificate).unwrap(),
            &[3; 32],
            uuid::Uuid::parse_str("33333333-3333-4333-8333-333333333333").unwrap(),
            file_persist(config_path.clone()),
        )
        .await
        .expect("isolated approved secure session opens");
    }
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    println!("READY {}", listener.local_addr().unwrap().port());
    io::stdout().flush().unwrap();
    let (send, receive) = oneshot::channel();
    let control_host = host.clone();
    let input = std::thread::spawn(move || {
        loop {
            let mut line = String::new();
            if io::stdin().read_line(&mut line).unwrap_or(0) == 0 || line.trim().is_empty() {
                break;
            }
            if let Some(id) = line.trim().strip_prefix("REVOKE ") {
                let id = uuid::Uuid::parse_str(id).expect("canonical test device ID required");
                let removed = control_host
                    .remove_device_checked(id, |bytes| {
                        // ! Only for tests, not a real atomic writer.
                        let mut file = std::fs::OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .open(&config_path)
                            .map_err(|_| PersistError::Unchanged)?;
                        file.write_all(bytes)
                            .and_then(|_| file.sync_all())
                            .map_err(|_| PersistError::Unchanged)
                    })
                    .expect("test-only removal must save");
                println!("REVOKED {removed}");
                io::stdout().flush().unwrap();
            } else if let Some(id) = line.trim().strip_prefix("UNCERTAIN_REVOKE ") {
                let id = uuid::Uuid::parse_str(id).expect("test device ID required");
                assert_eq!(
                    control_host.remove_device_checked(id, |_| Err(PersistError::RecoveryRequired)),
                    Err(RegistryError::RecoveryRequired)
                );
                println!("RECOVERY_REQUIRED");
                io::stdout().flush().unwrap();
            } else if let Some(value) = line.trim().strip_prefix("TEST_MODE ") {
                let enabled = match value {
                    "on" => true,
                    "off" => false,
                    _ => panic!("invalid test mode command"),
                };
                control_host
                    .set_test_mode_checked(enabled, |bytes| {
                        std::fs::write(&config_path, bytes)
                            .map_err(|_| PersistError::RecoveryRequired)
                    })
                    .expect("isolated test mode update must save");
                println!("TEST_MODE_SET");
                io::stdout().flush().unwrap();
            } else if let Some(request) = line.trim().strip_prefix("CONTROL ") {
                let reply = myrig_host_core::local_control::execute(
                    &control_host,
                    request.as_bytes(),
                    |_| panic!("this test control command must not persist"),
                );
                println!("CONTROL {}", String::from_utf8(reply).unwrap());
                io::stdout().flush().unwrap();
            } else if line.trim() == "EVENTS" {
                let events = control_host.events().events_after(0);
                println!("EVENTS {}", serde_json::to_string(&events).unwrap());
                io::stdout().flush().unwrap();
            } else {
                panic!("unsupported test control command");
            }
        }
        let _ = send.send(());
    });
    let stop = async {
        let _ = receive.await;
    };
    let certificate =
        std::env::var("MY_RIG_TEST_TLS_CERT").expect("isolated TLS certificate required");
    let key = std::env::var("MY_RIG_TEST_TLS_KEY").expect("isolated TLS key required");
    let identity = myrig_host_core::tls::FixedIdentity::from_der(
        vec![std::fs::read(certificate).unwrap()],
        std::fs::read(key).unwrap(),
    )
    .expect("valid fixed fixture identity");
    serve_secure_tls(listener, host, identity, stop, |error| {
        eprintln!("Harness accept error: {error}")
    })
    .await
    .unwrap();
    input.join().unwrap();
    println!("CALLS {}", calls.load(Ordering::SeqCst));
}
