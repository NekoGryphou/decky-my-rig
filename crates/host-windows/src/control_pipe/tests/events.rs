use super::fixtures::*;
use std::{sync::Arc, time::Duration};
use tokio::{io::AsyncWriteExt, net::windows::named_pipe::ClientOptions};
use tokio::{sync::watch, time::timeout};

#[tokio::test]
async fn four_event_waiters_leave_status_mutation_and_rejections_responsive() {
    struct NoPower;
    impl myrig_host_core::shutdown::Power for NoPower {
        fn schedule_shutdown(&self) -> Result<(), myrig_host_core::shutdown::PowerError> {
            panic!("pipe capacity tests must never reach power");
        }
        fn cancel_shutdown(&self) -> Result<(), myrig_host_core::shutdown::PowerError> {
            panic!("pipe capacity tests must never reach power")
        }
    }
    let host = Arc::new(myrig_host_core::host::Host::new(
        myrig_host_core::paired_devices::RegistryConfig::from_json(br#"{"schema_version":3,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","port":49000,"test_mode":false,"paired_devices":[]}"#).unwrap(),
        myrig_host_core::shutdown::Core::new(Box::new(NoPower)),
        "Fixture PC".into(), std::sync::Arc::new(|_| Ok(())),
    ));
    let (name, pipe) = fixture(|_, _| Ok(true));
    let (stop, receiver) = watch::channel(false);
    let (entered, mut entries) = tokio::sync::mpsc::unbounded_channel();
    let dispatch_host = host.clone();
    let capture_host = host.clone();
    let runtime = tokio::runtime::Handle::current();
    let server = tokio::spawn(pipe.serve(
        move |bytes, approval| {
            let request: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            if request["operation"] == "wait_events" {
                entered.send(()).unwrap();
            }
            runtime.block_on(myrig_host_core::local_control::execute_secure(
                &dispatch_host,
                bytes,
                approval,
                |_| Ok(()),
                |_, _| async { panic!("capacity test must not open pairing") },
            ))
        },
        move || capture_host.pairing_approval(),
        receiver,
    ));
    let wait = br#"{"version":1,"operation":"wait_events","after":0}"#;
    let mut waiters = Vec::new();
    for _ in 0..4 {
        let mut client = ClientOptions::new().open(&name).unwrap();
        client
            .write_all(&(wait.len() as u32).to_le_bytes())
            .await
            .unwrap();
        client.write_all(wait).await.unwrap();
        timeout(Duration::from_secs(3), entries.recv())
            .await
            .unwrap()
            .unwrap();
        waiters.push(client);
    }
    let mut normal = tokio::spawn(async move {
        for (request_bytes, field, expected) in [
            (br#"{"version":1,"operation":"status"}"#.as_slice(), "port", serde_json::json!(49000)),
            (br#"{"version":1,"operation":"set_test_mode","host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","enabled":true}"#, "test_mode", serde_json::json!(true)),
            (br#"{"version":1,"operation":"wait_events","after":0,"extra":1}"#, "error", serde_json::json!("invalid_request")),
            (wait, "error", serde_json::json!("unavailable")),
            (br#"{"version":1,"operation":"status"}"#, "test_mode", serde_json::json!(true)),
        ] {
            let mut client = ClientOptions::new().open(&name).unwrap();
            let response = request(&mut client, request_bytes).await;
            let reply: serde_json::Value = serde_json::from_slice(&response).unwrap();
            assert_eq!(reply[field], expected);
        }
    });
    let result = timeout(Duration::from_secs(2), &mut normal).await;
    if result.is_err() {
        normal.abort();
        let _ = normal.await;
    }
    host.events().close();
    stop.send(true).unwrap();
    drop(waiters);
    server.await.unwrap().unwrap();
    result
        .expect("normal pipe work must finish while all four event waits remain open")
        .unwrap();
    assert!(host.test_mode().unwrap());
}

#[tokio::test]
async fn close_invalidates_an_older_pipe_worker_before_core_dispatch() {
    struct NoPower;
    impl myrig_host_core::shutdown::Power for NoPower {
        fn schedule_shutdown(&self) -> Result<(), myrig_host_core::shutdown::PowerError> {
            panic!("pairing ordering test must never reach power");
        }
        fn cancel_shutdown(&self) -> Result<(), myrig_host_core::shutdown::PowerError> {
            panic!("pairing ordering test must never reach power");
        }
    }
    let host = Arc::new(myrig_host_core::host::Host::new(
        myrig_host_core::paired_devices::RegistryConfig::from_json(br#"{"schema_version":3,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","port":49000,"test_mode":true,"paired_devices":[]}"#).unwrap(),
        myrig_host_core::shutdown::Core::new(Box::new(NoPower)),
        "Fixture PC".into(), std::sync::Arc::new(|_| Ok(())),
    ));
    let (name, pipe) = fixture(|_, _| Ok(true));
    let (stop, receiver) = watch::channel(false);
    let (entered, entry) = tokio::sync::oneshot::channel();
    let entered = std::sync::Mutex::new(Some(entered));
    let (resume, resuming) = std::sync::mpsc::channel();
    let resuming = std::sync::Mutex::new(resuming);
    let dispatch_host = host.clone();
    let capture_host = host.clone();
    let runtime = tokio::runtime::Handle::current();
    let server = tokio::spawn(pipe.serve(
        move |bytes, approval| {
            let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
            if value["operation"] == "open_pairing" {
                entered.lock().unwrap().take().unwrap().send(()).unwrap();
                resuming.lock().unwrap().recv().unwrap();
            }
            runtime.block_on(myrig_host_core::local_control::execute_secure(
                &dispatch_host,
                bytes,
                approval,
                |_| Ok(()),
                |device_name, approval| {
                    let target = dispatch_host.clone();
                    async move {
                        target
                            .open_secure_pairing(
                                approval,
                                &device_name,
                                "12345678",
                                [7; 32],
                                b"cert",
                                &[1; 32],
                                uuid::Uuid::from_u128(3),
                                Arc::new(|_| Ok(())),
                            )
                            .await
                            .map_err(|_| "unavailable")?;
                        Ok(zeroize::Zeroizing::new("12345678".to_string()))
                    }
                },
            ))
        },
        move || capture_host.pairing_approval(),
        receiver,
    ));
    let open_name = name.clone();
    let opening = tokio::spawn(async move {
        let mut client = ClientOptions::new().open(&open_name).unwrap();
        request(&mut client, br#"{"version":1,"operation":"open_pairing","host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","name":"Device"}"#).await
    });
    entry.await.unwrap();
    let mut closer = ClientOptions::new().open(&name).unwrap();
    let closed = request(&mut closer, br#"{"version":1,"operation":"close_pairing","host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6"}"#).await;
    drop(closer);
    resume.send(()).unwrap();
    let opened = opening.await.unwrap();
    host.close_pairing();
    stop.send(true).unwrap();
    server.await.unwrap().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&closed).unwrap()["ok"],
        true
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&opened).unwrap()["error"],
        "unavailable",
        "an older worker must not recreate approval after close acknowledged"
    );
}
