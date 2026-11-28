use super::{Terminal, run};
use crate::host_client::fixtures::{RealHost, real_host_with};
use std::path::Path;
use tempfile::TempDir;

/// Runs one command; returns (exit code, stdout, stderr).
async fn myrig(store: &Path, arguments: &[&str], input: &str) -> (u8, String, String) {
    let arguments: Vec<String> = arguments
        .iter()
        .map(|argument| (*argument).to_owned())
        .collect();
    let (mut output, mut errors) = (Vec::new(), Vec::new());
    let code = run(
        crate::platform::fixtures::platform(),
        &arguments,
        store,
        Terminal {
            input: &mut input.as_bytes(),
            output: &mut output,
            errors: &mut errors,
        },
    )
    .await;
    (
        code,
        String::from_utf8(output).unwrap(),
        String::from_utf8(errors).unwrap(),
    )
}

async fn paired(store: &Path, host: &RealHost) -> String {
    let port = host.address.port().to_string();
    let (code, output, errors) = myrig(
        store,
        &["pair", "127.0.0.1", "--port", &port],
        "12345678\n02:00:00:00:00:01\n",
    )
    .await;
    assert_eq!(code, 0, "{output}{errors}");
    output
}

#[tokio::test]
async fn usage_errors_exit_2() {
    let store = TempDir::new().unwrap();
    for arguments in [
        &[][..],
        &["bogus"],
        &["wake"],
        &["pair"],
        &["pair", "127.0.0.1", "--port"],
        &["pair", "127.0.0.1", "--port", "0"],
        &["rename", "PC"],
        &["stop", "PC", "--force"],
        &["list", "extra"],
        &["agent"],
    ] {
        let (code, _, errors) = myrig(store.path(), arguments, "").await;
        assert_eq!(code, 2, "{arguments:?}");
        assert!(errors.contains("Usage:"), "{arguments:?}: {errors}");
    }
}

#[tokio::test]
async fn pair_list_status_stop_rename_and_remove() {
    let host = real_host_with(true, Some("12345678")).await;
    let store = TempDir::new().unwrap();
    let output = paired(store.path(), &host).await;
    assert!(
        output.contains("Enter the pairing code shown on the PC:"),
        "{output}"
    );
    assert!(output.contains("Paired with Fixture PC."), "{output}");
    let (code, list, _) = myrig(store.path(), &["list"], "").await;
    assert_eq!(code, 0);
    assert!(
        list.contains("Fixture PC")
            && list.contains("127.0.0.1:")
            && list.contains("02:00:00:00:00:01"),
        "{list}"
    );
    let (code, status, _) = myrig(store.path(), &["status"], "").await;
    assert_eq!((code, status.trim_end()), (0, "Fixture PC: online"));
    let (code, stopped, _) = myrig(store.path(), &["stop", "Fixture PC", "--yes"], "").await;
    assert_eq!(code, 0);
    assert!(stopped.contains("Test shutdown received"), "{stopped}");
    assert_eq!(
        myrig(store.path(), &["rename", "Fixture PC", "Desk"], "")
            .await
            .0,
        0
    );
    let (code, removed, _) = myrig(store.path(), &["remove", "Desk", "--yes"], "").await;
    assert_eq!(code, 0);
    assert!(
        removed.contains("Removed Desk. The PC forgot this device."),
        "{removed}"
    );
    assert_eq!(
        myrig(store.path(), &["list"], "").await.1,
        "No saved PCs.\n"
    );
}

#[tokio::test]
async fn a_mistyped_mac_is_asked_again_and_the_pairing_is_kept() {
    let host = real_host_with(true, Some("12345678")).await;
    let store = TempDir::new().unwrap();
    let port = host.address.port().to_string();
    let (code, output, errors) = myrig(
        store.path(),
        &["pair", "127.0.0.1", "--port", &port],
        "12345678\nnot-a-mac\n02:00:00:00:00:01\n",
    )
    .await;
    assert_eq!(code, 0, "{output}{errors}");
    assert!(output.contains("MAC address"), "{output}");
    let (_, listed, _) = myrig(store.path(), &["list"], "").await;
    assert!(listed.contains("02:00:00:00:00:01"), "{listed}");
}

#[tokio::test]
async fn stop_asks_first_and_names_can_be_ambiguous() {
    let host = real_host_with(true, Some("12345678")).await;
    let store = TempDir::new().unwrap();
    paired(store.path(), &host).await;
    let (code, output, _) = myrig(store.path(), &["stop", "Fixture PC"], "n\n").await;
    assert_eq!(code, 1);
    assert!(output.contains("Shut down Fixture PC? [y/N]"), "{output}");
    let second = real_host_with(true, Some("12345678")).await;
    paired(store.path(), &second).await;
    let (code, _, errors) = myrig(store.path(), &["status", "Fixture PC"], "").await;
    assert_eq!(code, 1);
    assert!(
        errors.contains("Several PCs are named Fixture PC"),
        "{errors}"
    );
    let id = errors.lines().nth(1).unwrap().trim().to_owned();
    assert_eq!(myrig(store.path(), &["status", &id], "").await.0, 0);
}

#[tokio::test]
async fn unknown_names_and_unreachable_pcs() {
    let store = TempDir::new().unwrap();
    let (code, _, errors) = myrig(store.path(), &["wake", "Nothing"], "").await;
    assert_eq!(code, 1);
    assert!(errors.contains("No saved PC is named Nothing."), "{errors}");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port().to_string();
    drop(listener);
    let (code, _, errors) = myrig(store.path(), &["pair", "127.0.0.1", "--port", &port], "").await;
    assert_eq!(code, 3, "{errors}");
    assert!(errors.contains("PC is unreachable."), "{errors}");
}

#[tokio::test]
async fn a_pairing_left_without_a_mac_is_revoked_on_the_pc() {
    // ! The PC saved this device when pairing finished. If input ends at the MAC prompt, its
    // ! credential mustn't be left there unused.
    let host = real_host_with(true, Some("12345678")).await;
    let store = TempDir::new().unwrap();
    let port = host.address.port().to_string();
    let (code, _, _) = myrig(
        store.path(),
        &["pair", "127.0.0.1", "--port", &port],
        "12345678\n",
    )
    .await;
    assert_ne!(code, 0);
    let issued = crate::host_client::HostClient::new(
        std::net::Ipv4Addr::LOCALHOST,
        host.address.port(),
        Some(crate::host_client::fixtures::token(8)),
        Some(host.pin),
        std::time::Duration::from_secs(3),
    );
    assert_eq!(
        issued.status().await.unwrap_err(),
        crate::host_client::HostError::Unauthorized
    );
}

// ! Windows arguments are UTF-16 and can't hold these bytes this way, so the check runs on Unix
// ! only.
#[cfg(unix)]
#[test]
fn non_utf8_argument_is_rejected() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};
    let bad = OsString::from_vec(vec![0x66, 0xff]);
    assert_eq!(super::service::text_arguments([bad].into_iter()), None);
    let good = [OsString::from("wake"), OsString::from("pc")];
    assert_eq!(
        super::service::text_arguments(good.into_iter()),
        Some(vec!["wake".to_owned(), "pc".to_owned()])
    );
}

#[test]
fn a_save_stuck_behind_a_lock_cannot_hold_the_process_up() {
    use super::utils::stop_runtime;
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };
    let (release, blocked) = mpsc::channel::<()>();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    runtime.spawn_blocking(move || {
        // - Stands for a store write waiting on another process; ends on its own
        //   after the test.
        let _ = blocked.recv_timeout(Duration::from_secs(5));
    });
    let started = Instant::now();
    let error = stop_runtime(runtime, Duration::from_millis(200)).unwrap_err();
    assert!(started.elapsed() < Duration::from_secs(3));
    assert!(error.0.contains("still waiting"), "{error}");
    drop(release);
    let idle = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    assert!(stop_runtime(idle, Duration::from_millis(200)).is_ok());
}

#[tokio::test]
async fn the_pairing_code_is_read_with_echo_hidden() {
    use crate::platform::fixtures::TestPlatform;
    use std::sync::{Arc, atomic::Ordering};
    let store = TempDir::new().unwrap();
    let host = real_host_with(true, Some("12345678")).await;
    let platform = TestPlatform::default();
    let hidden = platform.hidden.clone();
    let port = host.address.port().to_string();
    let (mut output, mut errors) = (Vec::new(), Vec::new());
    let code = run(
        Arc::new(platform),
        &["pair", "127.0.0.1", "--port", &port].map(str::to_owned),
        store.path(),
        Terminal {
            input: &mut "12345678\n02:00:00:00:00:01\n".as_bytes(),
            output: &mut output,
            errors: &mut errors,
        },
    )
    .await;
    assert_eq!(code, 0, "{}", String::from_utf8_lossy(&errors));
    assert_eq!(
        hidden.load(Ordering::SeqCst),
        1,
        "the code prompt hides echo once"
    );
    assert!(!String::from_utf8_lossy(&output).contains("12345678"));
}
