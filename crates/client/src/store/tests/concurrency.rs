use super::fixtures::{PIN, TOKEN_A, TOKEN_B, authentication, fixture, input, rename};
use crate::platform::fixtures::platform;
use crate::store::{PcChanges, PcConfig, Store};
use std::net::Ipv4Addr;
use std::sync::Arc;
use std::thread;

#[test]
fn serialized_adds_have_no_lost_updates() {
    let fixture = fixture();
    let store = Arc::new(fixture.store);
    let workers: Vec<_> = (0..24)
        .map(|index| {
            let store = Arc::clone(&store);
            thread::spawn(move || {
                store
                    .add(input(&index.to_string()), authentication(TOKEN_A, PIN))
                    .unwrap()
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let persisted = Store::open(&fixture.directory, platform())
        .unwrap()
        .configs();
    assert_eq!(persisted.len(), 24);
    let mut names: Vec<_> = persisted
        .iter()
        .map(|pc| pc.name.parse::<u32>().unwrap())
        .collect();
    names.sort_unstable();
    assert_eq!(names, (0..24).collect::<Vec<_>>());
}

#[test]
fn concurrent_partial_edits_merge_against_the_current_pc() {
    let fixture = fixture();
    let pc = fixture
        .store
        .add(input("original"), authentication(TOKEN_A, PIN))
        .unwrap();
    let store = Arc::new(fixture.store);
    let edits = [
        PcChanges {
            name: Some("renamed".to_owned()),
            ..PcChanges::default()
        },
        PcChanges {
            host: Some("192.168.1.8".to_owned()),
            ..PcChanges::default()
        },
        PcChanges {
            port: Some("50001".to_owned()),
            ..PcChanges::default()
        },
        PcChanges {
            mac: Some("02:11:22:33:44:55".to_owned()),
            ..PcChanges::default()
        },
    ];
    let workers: Vec<_> = edits
        .into_iter()
        .map(|edit| {
            let store = Arc::clone(&store);
            thread::spawn(move || store.update(pc.id, edit).unwrap())
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let last: PcConfig = Store::open(&fixture.directory, platform())
        .unwrap()
        .configs()
        .remove(0);
    assert_eq!(
        (
            last.id,
            last.name.as_str(),
            last.host,
            last.port,
            last.mac.as_str()
        ),
        (
            pc.id,
            "renamed",
            Ipv4Addr::new(192, 168, 1, 8),
            50001,
            "02:11:22:33:44:55"
        )
    );
    assert_eq!(last.authentication, pc.authentication);
}

#[test]
fn a_removed_pc_stays_removed_across_processes() {
    let fixture = fixture();
    let pc = fixture
        .store
        .add(input("PC"), authentication(TOKEN_A, PIN))
        .unwrap();
    let other = Store::open(&fixture.directory, platform()).unwrap();
    fixture.store.delete(pc.id).unwrap();
    // - The second store still has the PC in memory, but its writes start from
    //   disk.
    assert_eq!(
        other.update(pc.id, rename("stale")).unwrap_err().0,
        "PC was not found."
    );
    let added = other
        .add(input("New"), authentication(TOKEN_B, PIN))
        .unwrap();
    assert_eq!(
        Store::open(&fixture.directory, platform())
            .unwrap()
            .configs(),
        vec![added.clone()]
    );
    assert_eq!(other.configs(), vec![added]);
}
