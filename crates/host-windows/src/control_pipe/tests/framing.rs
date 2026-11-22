use super::super::{
    consts::{INVALID_RESPONSE, REVERT_FAILURE, UNAUTHORIZED},
    native::create_pipe,
    types::Authorize,
};
use super::fixtures::*;
use myrig_protocol::local_control::{REQUEST_LIMIT, RESPONSE_LIMIT};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::windows::named_pipe::ClientOptions,
};
use tokio::{sync::watch, time::timeout};

#[tokio::test]
async fn pipe_serves_concurrent_connections() {
    let (name, pipe) = fixture(|_, _| Ok(true));
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let entered = std::sync::Mutex::new(Some(entered_tx));
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let release = std::sync::Mutex::new(release_rx);
    let (stop, receiver) = watch::channel(false);
    let server = tokio::spawn(pipe.serve(
        move |bytes, _context| {
            if bytes == b"first" {
                entered.lock().unwrap().take().unwrap().send(()).unwrap();
                release.lock().unwrap().recv().unwrap();
            }
            bytes.to_vec()
        },
        || (),
        receiver,
    ));
    let mut first = ClientOptions::new().open(&name).unwrap();
    let first_reply = tokio::spawn(async move { request(&mut first, b"first").await });
    entered_rx.await.unwrap();
    let second_reply = match ClientOptions::new().open(&name) {
        Ok(mut second) => timeout(Duration::from_millis(500), request(&mut second, b"second"))
            .await
            .ok(),
        Err(_) => None,
    };
    release_tx.send(()).unwrap();
    assert_eq!(first_reply.await.unwrap(), b"first");
    stop.send(true).unwrap();
    server.await.unwrap().unwrap();
    assert_eq!(second_reply, Some(b"second".to_vec()));
}

#[tokio::test]
async fn private_fixture_first_instance_and_authorized_framing() {
    let (name, pipe) = fixture(|_, _| Ok(true));
    assert!(create_pipe(&name, &format!("D:P(A;;FA;;;{})", current_sid()), true).is_err());
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&calls);
    let (stop, receiver) = watch::channel(false);
    let server = tokio::spawn(pipe.serve(
        move |bytes, _context| {
            observed.fetch_add(1, Ordering::SeqCst);
            bytes.to_vec()
        },
        || (),
        receiver,
    ));
    let mut client = ClientOptions::new().open(&name).unwrap();
    assert_eq!(
        request(&mut client, b"fixture request").await,
        b"fixture request"
    );
    drop(client);
    stop.send(true).unwrap();
    server.await.unwrap().unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn private_fixture_denial_and_fatal_revert_never_dispatch() {
    let deny: Authorize = |_, _| Ok(false);
    let revert_failure: Authorize = |_, _| Err(REVERT_FAILURE);
    for (authorize, denies) in [(deny, true), (revert_failure, false)] {
        let (name, pipe) = fixture(authorize);
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let (stop, receiver) = watch::channel(false);
        let server = tokio::spawn(pipe.serve(
            move |_, _context| {
                observed.fetch_add(1, Ordering::SeqCst);
                b"unexpected".to_vec()
            },
            || (),
            receiver,
        ));
        let mut client = ClientOptions::new().open(&name).unwrap();
        if denies {
            assert_eq!(request(&mut client, b"denied").await, UNAUTHORIZED);
            stop.send(true).unwrap();
            drop(client);
            server.await.unwrap().unwrap();
        } else {
            client.write_all(&1u32.to_le_bytes()).await.unwrap();
            client.write_all(b"x").await.unwrap();
            assert_eq!(server.await.unwrap(), Err(REVERT_FAILURE));
        }
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn private_fixture_caps_response_and_joins_started_callback_on_stop() {
    let (name, pipe) = fixture(|_, _| Ok(true));
    let (stop, receiver) = watch::channel(false);
    let server = tokio::spawn(pipe.serve(
        move |_, _context| vec![b'x'; RESPONSE_LIMIT + 1],
        || (),
        receiver,
    ));
    let mut client = ClientOptions::new().open(&name).unwrap();
    assert_eq!(
        request(&mut client, b"bounded response").await,
        INVALID_RESPONSE
    );
    drop(client);
    stop.send(true).unwrap();
    server.await.unwrap().unwrap();

    let (name, pipe) = fixture(|_, _| Ok(true));
    let (started, mut started_receiver) = watch::channel(false);
    let (release, released) = std::sync::mpsc::channel::<()>();
    let released = std::sync::Mutex::new(released);
    let complete = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&complete);
    let (stop, receiver) = watch::channel(false);
    let server = tokio::spawn(pipe.serve(
        move |_, _context| {
            started.send(true).unwrap();
            // - Still running when the client leaves and the pipe stops.
            released.lock().unwrap().recv().unwrap();
            observed.fetch_add(1, Ordering::SeqCst);
            b"completed".to_vec()
        },
        || (),
        receiver,
    ));
    let mut client = ClientOptions::new().open(&name).unwrap();
    client.write_all(&1u32.to_le_bytes()).await.unwrap();
    client.write_all(b"x").await.unwrap();
    started_receiver.changed().await.unwrap();
    drop(client);
    stop.send(true).unwrap();
    assert!(!server.is_finished());
    release.send(()).unwrap();
    server.await.unwrap().unwrap();
    assert_eq!(complete.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn private_fixture_rejects_bad_lengths_and_bounds_partial_frame() {
    for length in [0u32, REQUEST_LIMIT as u32 + 1, 4] {
        let (name, pipe) = fixture(|_, _| Ok(true));
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = Arc::clone(&calls);
        let (stop, receiver) = watch::channel(false);
        let server = tokio::spawn(pipe.serve(
            move |_, _context| {
                observed.fetch_add(1, Ordering::SeqCst);
                b"unexpected".to_vec()
            },
            || (),
            receiver,
        ));
        let mut client = ClientOptions::new().open(&name).unwrap();
        client.write_all(&length.to_le_bytes()).await.unwrap();
        // - The length=4 case never sends its body. The server should disconnect
        //   at the frame deadline without running anything.
        let mut response = [0u8; 1];
        let read = timeout(Duration::from_secs(3), client.read(&mut response))
            .await
            .unwrap();
        assert!(matches!(read, Ok(0)) || read.is_err());
        drop(client);
        stop.send(true).unwrap();
        server.await.unwrap().unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
}

#[tokio::test]
async fn private_fixture_future_cancellation_joins_callback() {
    let (name, pipe) = fixture(|_, _| Ok(true));
    let (started, mut started_receiver) = watch::channel(false);
    let (release, released) = std::sync::mpsc::channel::<()>();
    let released = std::sync::Mutex::new(released);
    let complete = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&complete);
    let (_stop, receiver) = watch::channel(false);
    let server = tokio::spawn(pipe.serve(
        move |_, _context| {
            started.send(true).unwrap();
            // - Still running when the serving future is canceled.
            released.lock().unwrap().recv().unwrap();
            observed.fetch_add(1, Ordering::SeqCst);
            b"completed".to_vec()
        },
        || (),
        receiver,
    ));
    let mut client = ClientOptions::new().open(&name).unwrap();
    client.write_all(&1u32.to_le_bytes()).await.unwrap();
    client.write_all(b"x").await.unwrap();
    started_receiver.changed().await.unwrap();
    server.abort();
    // - Dropping the canceled future joins the callback on this thread, so
    //   another thread has to release it.
    std::thread::spawn(move || release.send(()).unwrap());
    assert!(server.await.unwrap_err().is_cancelled());
    assert_eq!(complete.load(Ordering::SeqCst), 1);
    drop(client);
}

#[tokio::test]
async fn private_fixture_reuses_instance_and_dispatches_once_per_connection() {
    let (name, pipe) = fixture(|_, _| Ok(true));
    let calls = Arc::new(AtomicUsize::new(0));
    let observed = calls.clone();
    let (stop, receiver) = watch::channel(false);
    let server = tokio::spawn(pipe.serve(
        move |bytes, _context| {
            observed.fetch_add(1, Ordering::SeqCst);
            bytes.to_vec()
        },
        || (),
        receiver,
    ));
    for index in 0..3 {
        let mut client = timeout(Duration::from_secs(3), async {
            loop {
                match ClientOptions::new().open(&name) {
                    Ok(client) => break client,
                    Err(error) if error.raw_os_error() == Some(231) => {
                        tokio::time::sleep(Duration::from_millis(5)).await
                    }
                    Err(error) => panic!("fixture connect failed: {error}"),
                }
            }
        })
        .await
        .unwrap();
        assert_eq!(request(&mut client, b"one request").await, b"one request");
        if index == 0 {
            let mut extra = 1u32.to_le_bytes().to_vec();
            extra.push(b'x');
            let _ = client.write_all(&extra).await;
        }
        drop(client);
    }
    stop.send(true).unwrap();
    server.await.unwrap().unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 3);
}
