use crate::paired_devices::RegistryConfig;
use std::sync::{Arc, Mutex};

use super::fixtures::*;
use crate::host::*;

#[test]
fn local_removal_is_host_scoped_and_published_only_after_persistence() {
    let host = registry_host(1);
    let other_host = registry_host(3);
    assert_ne!(host.host_id, other_host.host_id);
    let device = uuid::Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap();
    assert_eq!(
        host.remove_device_checked(device, |_| Err(PersistError::Unchanged)),
        Err(RegistryError::PersistenceFailed)
    );
    assert!(host.authenticated_device(&"01".repeat(32)).is_some());
    assert_eq!(host.paired_devices().unwrap().len(), 2);
    let mut saved = Vec::new();
    assert_eq!(
        host.remove_device_checked(device, |bytes| {
            // - The old credential still works while the save is running.
            assert!(host.authenticated_device(&"01".repeat(32)).is_some());
            saved.extend_from_slice(bytes);
            Ok(())
        }),
        Ok(true)
    );
    assert!(host.authenticated_device(&"01".repeat(32)).is_none());
    assert!(host.authenticated_device(&"02".repeat(32)).is_some());
    assert!(other_host.authenticated_device(&"03".repeat(32)).is_some());
    assert_eq!(other_host.paired_devices().unwrap().len(), 2);
    let restored = RegistryConfig::from_json(&saved).unwrap();
    assert!(restored.authenticate_device(&"01".repeat(32)).is_none());
    assert!(restored.authenticate_device(&"02".repeat(32)).is_some());
    assert!(restored.test_mode());
    assert_eq!(restored.host_id(), host.host_id);
    assert_eq!(
        host.remove_device_checked(device, |_| panic!("unknown device must not write")),
        Ok(false)
    );
}

#[test]
fn removed_credentials_stay_removed_under_concurrent_removal() {
    let host = Arc::new(registry_host(1));
    let saved = Arc::new(Mutex::new(Vec::new()));
    let mut threads = Vec::new();
    for id in [
        "11111111-1111-4111-8111-111111111111",
        "22222222-2222-4222-8222-222222222222",
    ] {
        let host = host.clone();
        let saved = saved.clone();
        threads.push(std::thread::spawn(move || {
            host.remove_device_checked(uuid::Uuid::parse_str(id).unwrap(), |bytes| {
                *saved.lock().unwrap() = bytes.to_vec();
                Ok(())
            })
        }));
    }
    for thread in threads {
        assert_eq!(thread.join().unwrap(), Ok(true));
    }
    assert!(host.paired_devices().unwrap().is_empty());
    let restored = RegistryConfig::from_json(&saved.lock().unwrap()).unwrap();
    assert!(restored.authenticate_device(&"01".repeat(32)).is_none());
    assert!(restored.authenticate_device(&"02".repeat(32)).is_none());
}

#[test]
fn uncertain_storage_or_conflict_fails_closed_for_all_credentials() {
    for failure in [PersistError::RecoveryRequired, PersistError::Conflict] {
        let host = registry_host(1);
        assert!(host.authenticated_device(&"01".repeat(32)).is_some());
        assert!(host.authenticated_device(&"02".repeat(32)).is_some());
        assert!(
            host.remove_device_checked(
                uuid::Uuid::parse_str("11111111-1111-4111-8111-111111111111").unwrap(),
                |_| Err(failure)
            )
            .is_err()
        );
        assert!(host.authenticated_device(&"01".repeat(32)).is_none());
        assert!(host.authenticated_device(&"02".repeat(32)).is_none());
        assert_eq!(host.paired_devices(), Err(RegistryError::RecoveryRequired));
        assert_eq!(
            host.remove_device_checked(uuid::Uuid::nil(), |_| panic!(
                "recovery must block persistence"
            )),
            Err(RegistryError::RecoveryRequired)
        );
    }
}
