use super::{
    utils::{self, run_with},
    *,
};
use crate::{
    host::Host,
    paired_devices::RegistryConfig,
    shutdown::{Core, Power, PowerError},
    tls::FixedIdentity,
};
use std::{io, sync::Arc, time::Duration};
use tokio::{net::TcpListener, sync::watch};

struct NoPower;
impl Power for NoPower {
    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        panic!("listener test must not schedule power")
    }
}

fn fixture_host(port: u16) -> Arc<Host> {
    let config = format!(
        "{{\"schema_version\":3,\"host_id\":\"03c85317-9d93-4fb3-a04b-91a0e16668c6\",\"port\":{port},\"test_mode\":true,\"paired_devices\":[]}}"
    );
    Arc::new(Host::new(
        RegistryConfig::from_json(config.as_bytes()).unwrap(),
        Core::new(Box::new(NoPower)),
        "Fixture PC".into(),
        std::sync::Arc::new(|_| Ok(())),
    ))
}

fn fixture_identity() -> FixedIdentity {
    let generated = rcgen::generate_simple_self_signed(vec!["myrig.local".to_owned()]).unwrap();
    FixedIdentity::from_der(
        vec![generated.cert.der().to_vec()],
        generated.signing_key.serialize_der(),
    )
    .unwrap()
}

/// A listener started on a port another socket holds, after its first AddrInUse
/// report.
async fn start_on_occupied_port() -> (
    TcpListener,
    Arc<Host>,
    watch::Sender<bool>,
    tokio::sync::mpsc::UnboundedReceiver<io::ErrorKind>,
    tokio::task::JoinHandle<()>,
) {
    let occupied = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let host = fixture_host(occupied.local_addr().unwrap().port());
    let (stop_tx, stop_rx) = watch::channel(false);
    let (errors_tx, mut errors_rx) = tokio::sync::mpsc::unbounded_channel();
    let task = tokio::spawn(run(
        host.clone(),
        fixture_identity(),
        utils::bind_any,
        stop_rx,
        move |_, error| {
            let _ = errors_tx.send(error.kind());
        },
    ));
    let kind = tokio::time::timeout(Duration::from_secs(2), errors_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(kind, io::ErrorKind::AddrInUse);
    (occupied, host, stop_tx, errors_rx, task)
}

#[tokio::test]
async fn a_port_freed_later_is_bound_again_without_a_port_change() {
    let (occupied, host, stop_tx, mut errors_rx, task) = start_on_occupied_port().await;
    drop(occupied);
    tokio::time::timeout(Duration::from_secs(5), async {
        while host.listener_state() != "listening" {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the listener must retry the bind on its own");
    // - One report per failure streak, not one per retry.
    assert!(errors_rx.try_recv().is_err());
    stop_tx.send(true).unwrap();
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
}

#[tokio::test]
async fn occupied_port_keeps_control_status_available_and_set_port_rebinds() {
    let (_occupied, host, stop_tx, _errors, task) = start_on_occupied_port().await;
    let status: serde_json::Value = serde_json::from_slice(&crate::local_control::execute(
        &host,
        br#"{"version":1,"operation":"status"}"#,
        |_| Ok(()),
    ))
    .unwrap();
    assert_eq!(status["listener_state"], "unavailable");
    assert!(!task.is_finished());

    let free = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let next_port = free.local_addr().unwrap().port();
    drop(free);
    let request = format!(
        "{{\"version\":1,\"operation\":\"set_port\",\"host_id\":\"03c85317-9d93-4fb3-a04b-91a0e16668c6\",\"port\":{next_port}}}"
    );
    let reply: serde_json::Value = serde_json::from_slice(&crate::local_control::execute(
        &host,
        request.as_bytes(),
        |_| Ok(()),
    ))
    .unwrap();
    assert_eq!(reply["port"], next_port);
    tokio::time::timeout(Duration::from_secs(2), async {
        while host.listener_state() != "listening" {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        TcpListener::bind((std::net::Ipv4Addr::UNSPECIFIED, next_port))
            .await
            .is_err()
    );
    stop_tx.send(true).unwrap();
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(host.listener_state(), "unavailable");
}

#[tokio::test]
async fn accept_failures_rebind_and_report_once_per_streak() {
    let free = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let host = fixture_host(free.local_addr().unwrap().port());
    drop(free);
    let (stop_tx, stop_rx) = watch::channel(false);
    let reports = Arc::new(std::sync::Mutex::new(Vec::new()));
    let serves = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (recorded, counted) = (reports.clone(), serves.clone());
    let task = tokio::spawn(run_with(
        host.clone(),
        utils::bind_any,
        stop_rx,
        move |_, error: &io::Error| recorded.lock().unwrap().push(error.to_string()),
        move |listener, _port, stop: super::types::ListenerStop| {
            let call = counted.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            async move {
                // - A bound listener whose accept loop ends right away, like with
                //   EMFILE.
                drop(listener);
                if call < 2 {
                    return Err(io::Error::other("accept failed"));
                }
                stop.await;
                Ok(())
            }
        },
    ));
    tokio::time::timeout(Duration::from_secs(10), async {
        while serves.load(std::sync::atomic::Ordering::SeqCst) < 3 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("the listener must rebind after accept failures");
    assert_eq!(*reports.lock().unwrap(), ["accept failed"]);
    assert_eq!(host.listener_state(), "listening");
    stop_tx.send(true).unwrap();
    tokio::time::timeout(Duration::from_secs(2), task)
        .await
        .unwrap()
        .unwrap();
}
