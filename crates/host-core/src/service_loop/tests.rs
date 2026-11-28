use super::*;
use crate::{
    host::Host,
    paired_devices::RegistryConfig,
    shutdown::{Core, Power, PowerError},
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;
struct NoPower;
impl Power for NoPower {
    fn schedule_shutdown(&self) -> Result<(), PowerError> {
        panic!("service lifecycle must not schedule power")
    }
    fn cancel_shutdown(&self) -> Result<(), PowerError> {
        panic!("service lifecycle must not schedule power")
    }
}
fn host() -> Arc<Host> {
    Arc::new(Host::new(
        RegistryConfig::from_json(br#"{"schema_version":3,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","port":47991,"test_mode":false,"paired_devices":[]}"#).unwrap(),
        Core::new(Box::new(NoPower)),
        "Fixture PC".into(), std::sync::Arc::new(|_| Ok(())),
    ))
}
async fn endpoint(
    mut stop: watch::Receiver<bool>,
    joined: Arc<AtomicUsize>,
    fail: bool,
) -> Result<(), ()> {
    while !*stop.borrow_and_update() {
        if stop.changed().await.is_err() {
            break;
        }
    }
    tokio::task::yield_now().await;
    joined.fetch_add(1, Ordering::SeqCst);
    if fail { Err(()) } else { Ok(()) }
}
#[tokio::test]
async fn normal_stop_joins_both_and_reports_stopping_once() {
    let (stop, receiver) = watch::channel(false);
    let joined = Arc::new(AtomicUsize::new(0));
    let status_calls = AtomicUsize::new(0);
    assert_eq!(
        join_endpoints(
            host(),
            endpoint(receiver.clone(), joined.clone(), false),
            endpoint(receiver, joined.clone(), false),
            async {},
            stop,
            || {
                status_calls.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        )
        .await,
        Ok(())
    );
    assert_eq!(joined.load(Ordering::SeqCst), 2);
    assert_eq!(status_calls.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn a_failed_or_unexpected_endpoint_stops_and_joins_its_peer() {
    for (is_http, fail) in [(true, true), (false, true), (true, false), (false, false)] {
        let (stop, receiver) = watch::channel(false);
        let joined = Arc::new(AtomicUsize::new(0));
        let http = async {
            if is_http {
                if fail { Err(()) } else { Ok(()) }
            } else {
                endpoint(receiver.clone(), joined.clone(), false).await
            }
        };
        let control = async {
            if !is_http {
                if fail { Err(()) } else { Ok(()) }
            } else {
                endpoint(receiver.clone(), joined.clone(), false).await
            }
        };
        let expected = if !fail {
            ServiceLoopError::UnexpectedExit
        } else if is_http {
            ServiceLoopError::Http
        } else {
            ServiceLoopError::Control
        };
        assert_eq!(
            join_endpoints(host(), http, control, std::future::pending(), stop, || Ok(
                ()
            ))
            .await,
            Err(expected)
        );
        assert_eq!(joined.load(Ordering::SeqCst), 1);
    }
}
#[tokio::test]
async fn stopping_status_failure_and_peer_failure_still_join_both() {
    for fail_status in [true, false] {
        let (stop, receiver) = watch::channel(false);
        let joined = Arc::new(AtomicUsize::new(0));
        assert_eq!(
            join_endpoints(
                host(),
                endpoint(receiver.clone(), joined.clone(), true),
                endpoint(receiver, joined.clone(), true),
                async {},
                stop,
                || if fail_status { Err(()) } else { Ok(()) }
            )
            .await,
            Err(if fail_status {
                ServiceLoopError::Status
            } else {
                ServiceLoopError::Http
            })
        );
        assert_eq!(joined.load(Ordering::SeqCst), 2);
    }
}

#[tokio::test]
async fn queued_approval_cannot_reopen_after_stop_barrier() {
    let host = Arc::new(Host::new(
        RegistryConfig::from_json(br#"{"schema_version":3,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","port":47991,"test_mode":true,"paired_devices":[]}"#).unwrap(),
        Core::new(Box::new(NoPower)),
        "Fixture PC".into(), std::sync::Arc::new(|_| Ok(())),
    ));
    let (stop, mut receiver) = watch::channel(false);
    let callback_host = host.clone();
    let queued_callback = async move {
        receiver.changed().await.unwrap();
        assert_eq!(
            callback_host
                .open_secure_pairing(
                    callback_host.pairing_approval(),
                    "Device",
                    "12345678",
                    [1; 32],
                    b"certificate",
                    &[2; 32],
                    uuid::Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap(),
                    Arc::new(|_| panic!("must not save")),
                )
                .await,
            Err(crate::host::OpenPairingError::Unavailable)
        );
        Ok(())
    };
    // - The barrier also applies to callbacks queued before the endpoints
    //   stopped.
    assert_eq!(
        join_endpoints(
            host,
            async { Ok(()) },
            queued_callback,
            async {},
            stop,
            || Ok(())
        )
        .await,
        Ok(())
    );
}

#[tokio::test]
async fn stop_wakes_test_event_waiter_before_joining_endpoints() {
    let host = host();
    let events = host.events();
    let waiting = tokio::spawn(async move { events.wait_after(0, Duration::from_secs(25)).await });
    let (stop, _) = watch::channel(false);
    let result = tokio::time::timeout(
        Duration::from_millis(200),
        join_endpoints(
            host,
            async { Ok(()) },
            async {
                assert!(waiting.await.unwrap().is_empty());
                Ok(())
            },
            async {},
            stop,
            || Ok(()),
        ),
    )
    .await;
    assert_eq!(result, Ok(Ok(())));
}
