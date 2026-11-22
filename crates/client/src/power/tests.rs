use super::fixtures::{FakeEnvironment, Fixture, HOST_ID, fixture, online};
use crate::{
    host_client::{HostError, ShutdownResult, Status},
    network::WakeError,
    store::PcChanges,
};
use myrig_protocol::{
    api::{PcIssue, PowerState},
    lan::HostState,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::Semaphore;
use uuid::Uuid;

const TRANSITION: Duration = Duration::from_secs(120);

fn state(fixture: &Fixture, index: usize) -> (PowerState, Option<PcIssue>) {
    let row = &fixture.controller.publish(false).pcs[index];
    (row.state, row.issue)
}

async fn online_pc(fixture: &Fixture) -> String {
    let id = fixture.add("PC").id.to_string();
    fixture.controller.poll(Uuid::parse_str(&id).unwrap()).await;
    assert_eq!(state(fixture, 0), (PowerState::Online, None));
    id
}

async fn offline_pc(fixture: &Fixture) -> String {
    *fixture.environment().status.lock().unwrap() = Err(HostError::Connection);
    let id = fixture.add("PC").id;
    fixture.controller.poll(id).await;
    assert_eq!(state(fixture, 0).0, PowerState::Checking);
    fixture.controller.poll(id).await;
    assert_eq!(state(fixture, 0).0, PowerState::Offline);
    id.to_string()
}

#[tokio::test]
async fn new_pcs_start_checking_and_snapshots_are_ordered_and_secret_free() {
    let fixture = fixture(FakeEnvironment::default());
    fixture.add("B");
    fixture.add("A");
    let frame = fixture.controller.publish(false);
    assert_eq!(
        frame
            .pcs
            .iter()
            .map(|row| row.name.as_str())
            .collect::<Vec<_>>(),
        ["B", "A"]
    );
    assert!(
        frame
            .pcs
            .iter()
            .all(|row| row.state == PowerState::Checking && row.issue.is_none())
    );
    let text = serde_json::to_string(&frame).unwrap()
        + &serde_json::to_string(&fixture.controller.settings()).unwrap();
    assert!(!text.contains(&"ab".repeat(32)) && !text.contains("1212"));
    assert_eq!(fixture.controller.settings()[0].port, 47991);
}

#[tokio::test]
async fn revisions_increase_with_every_frame() {
    let fixture = fixture(FakeEnvironment::default());
    let first = fixture.controller.publish(false).revision;
    let second = fixture.controller.publish(true);
    assert!(second.revision > first && second.refreshed);
}

#[tokio::test]
async fn offline_only_after_two_unanswered_probes() {
    let fixture = fixture(FakeEnvironment::default());
    offline_pc(&fixture).await;
}

#[tokio::test]
async fn test_mode_stop_keeps_online_and_counts_the_stop() {
    let fixture = fixture(FakeEnvironment::default());
    let id = online_pc(&fixture).await;
    *fixture.environment().shutdown.lock().unwrap() = Ok(ShutdownResult { test: true });
    assert_eq!(fixture.controller.stop(&id).await, Ok(true));
    let frame = fixture.controller.publish(false);
    assert_eq!(frame.pcs[0].state, PowerState::Online);
    assert_eq!(
        (frame.test_stops.count, frame.test_stops.name.as_deref()),
        (1, Some("PC"))
    );
}

#[tokio::test]
async fn real_stop_stays_stopping_until_the_pc_goes_away_or_the_deadline() {
    let fixture = fixture(FakeEnvironment::default());
    let id = online_pc(&fixture).await;
    let uuid = Uuid::parse_str(&id).unwrap();
    assert_eq!(fixture.controller.stop(&id).await, Ok(false));
    assert_eq!(state(&fixture, 0).0, PowerState::Stopping);
    // - It still answers online right after the request, so keep showing
    //   stopping.
    fixture.controller.poll(uuid).await;
    assert_eq!(state(&fixture, 0).0, PowerState::Stopping);
    fixture.environment().advance(TRANSITION);
    fixture.controller.poll(uuid).await;
    assert_eq!(state(&fixture, 0).0, PowerState::Online);
}

#[tokio::test]
async fn shutting_down_status_enters_stopping_then_unanswered_probes_go_offline() {
    let fixture = fixture(FakeEnvironment::default());
    let id = Uuid::parse_str(&online_pc(&fixture).await).unwrap();
    *fixture.environment().status.lock().unwrap() = Ok(Status {
        host_id: Uuid::parse_str(HOST_ID).unwrap(),
        state: HostState::ShuttingDown,
    });
    fixture.controller.poll(id).await;
    assert_eq!(state(&fixture, 0).0, PowerState::Stopping);
    *fixture.environment().status.lock().unwrap() = Err(HostError::Timeout);
    fixture.controller.poll(id).await;
    fixture.controller.poll(id).await;
    assert_eq!(state(&fixture, 0).0, PowerState::Offline);
}

#[tokio::test]
async fn stop_refusals_record_the_issue_and_disable_stop() {
    for (error, issue) in [
        (HostError::Unauthorized, PcIssue::Authentication),
        (HostError::PinMismatch, PcIssue::Identity),
        (HostError::UnsupportedProtocol, PcIssue::Compatibility),
    ] {
        let fixture = fixture(FakeEnvironment::default());
        let id = online_pc(&fixture).await;
        *fixture.environment().shutdown.lock().unwrap() = Err(error);
        assert_eq!(
            fixture.controller.stop(&id).await.unwrap_err().0,
            "PC could not be stopped."
        );
        assert_eq!(state(&fixture, 0), (PowerState::Online, Some(issue)));
        assert_eq!(
            fixture.controller.stop(&id).await.unwrap_err().0,
            "PC is not available to stop."
        );
        assert_eq!(
            FakeEnvironment::count(&fixture.environment().calls.shutdown),
            1
        );
    }
}

#[tokio::test]
async fn a_failed_stop_is_not_retried_and_polling_checks_the_result() {
    let fixture = fixture(FakeEnvironment::default());
    let id = online_pc(&fixture).await;
    *fixture.environment().shutdown.lock().unwrap() = Err(HostError::Timeout);
    assert!(fixture.controller.stop(&id).await.is_err());
    assert_eq!(state(&fixture, 0), (PowerState::Stopping, None));
    assert_eq!(
        FakeEnvironment::count(&fixture.environment().calls.shutdown),
        1
    );
}

#[tokio::test]
async fn status_problems_become_issues_without_going_offline() {
    for (status, issue) in [
        (Err(HostError::Unauthorized), PcIssue::Authentication),
        (Err(HostError::PinMismatch), PcIssue::Identity),
        (Err(HostError::Invalid), PcIssue::Compatibility),
        (Err(HostError::UnsupportedProtocol), PcIssue::Compatibility),
        (
            Ok(Status {
                host_id: Uuid::new_v4(),
                state: HostState::Online,
            }),
            PcIssue::Identity,
        ),
    ] {
        let fixture = fixture(FakeEnvironment::default());
        let id = Uuid::parse_str(&online_pc(&fixture).await).unwrap();
        *fixture.environment().status.lock().unwrap() = status;
        assert!(!fixture.controller.poll(id).await);
        assert_eq!(state(&fixture, 0), (PowerState::Online, Some(issue)));
        *fixture.environment().status.lock().unwrap() = online();
        assert!(fixture.controller.poll(id).await);
        assert_eq!(state(&fixture, 0), (PowerState::Online, None));
    }
}

#[tokio::test]
async fn start_wakes_once_from_offline_and_waits_for_the_deadline() {
    let fixture = fixture(FakeEnvironment::default());
    let id = offline_pc(&fixture).await;
    fixture.controller.start(&id).await.unwrap();
    assert_eq!(state(&fixture, 0).0, PowerState::Starting);
    assert_eq!(
        fixture.controller.start(&id).await.unwrap_err().0,
        "PC is not available to start."
    );
    assert_eq!(FakeEnvironment::count(&fixture.environment().calls.wake), 1);
    let uuid = Uuid::parse_str(&id).unwrap();
    fixture.controller.poll(uuid).await;
    fixture.controller.poll(uuid).await;
    fixture.controller.poll(uuid).await;
    assert_eq!(state(&fixture, 0).0, PowerState::Starting);
    fixture.environment().advance(TRANSITION);
    fixture.controller.poll(uuid).await;
    assert_eq!(state(&fixture, 0).0, PowerState::Offline);
}

#[tokio::test]
async fn an_offline_pc_with_an_issue_can_still_be_woken() {
    let fixture = fixture(FakeEnvironment::default());
    let id = offline_pc(&fixture).await;
    *fixture.environment().status.lock().unwrap() = Err(HostError::Unauthorized);
    fixture.controller.poll(Uuid::parse_str(&id).unwrap()).await;
    fixture.controller.publish(false);
    *fixture.environment().status.lock().unwrap() = Err(HostError::Connection);
    for _ in 0..2 {
        fixture.controller.poll(Uuid::parse_str(&id).unwrap()).await;
    }
    assert_eq!(
        state(&fixture, 0),
        (PowerState::Offline, Some(PcIssue::Authentication))
    );
    fixture.controller.start(&id).await.unwrap();
}

#[tokio::test]
async fn a_failed_wake_restores_offline() {
    let fixture = fixture(FakeEnvironment::default());
    let id = offline_pc(&fixture).await;
    *fixture.environment().wake.lock().unwrap() = Err(WakeError::NoInterface);
    assert_eq!(
        fixture.controller.start(&id).await.unwrap_err().0,
        "PC could not be started."
    );
    assert_eq!(state(&fixture, 0).0, PowerState::Offline);
}

#[tokio::test]
async fn an_edit_made_during_a_wake_survives_it() {
    let gate = Arc::new(Semaphore::new(0));
    let fixture = fixture(FakeEnvironment {
        wake_gate: Some(gate.clone()),
        wake: std::sync::Mutex::new(Err(WakeError::Send)),
        ..FakeEnvironment::default()
    });
    let id = offline_pc(&fixture).await;
    let starting = {
        let (controller, id) = (fixture.controller.clone(), id.clone());
        tokio::spawn(async move { controller.start(&id).await })
    };
    while FakeEnvironment::count(&fixture.environment().calls.wake) == 0 {
        tokio::task::yield_now().await;
    }
    let renamed = PcChanges {
        name: Some("Renamed".into()),
        ..PcChanges::default()
    };
    fixture.controller.update_pc(&id, renamed).unwrap();
    gate.add_permits(1);
    assert!(starting.await.unwrap().is_err());
    // - The failed wake was for the old record, so the edit's fresh state stays.
    assert_eq!(state(&fixture, 0).0, PowerState::Checking);
}

#[tokio::test]
async fn edits_recheck_the_pc_and_keep_its_credentials() {
    let fixture = fixture(FakeEnvironment::default());
    let id = online_pc(&fixture).await;
    let settings = fixture
        .controller
        .update_pc(
            &id,
            PcChanges {
                host: Some(" 192.168.1.41 ".into()),
                ..PcChanges::default()
            },
        )
        .unwrap();
    assert_eq!(
        (settings.id.as_str(), settings.host.as_str()),
        (id.as_str(), "192.168.1.41")
    );
    assert_eq!(state(&fixture, 0).0, PowerState::Checking);
    let credentials = PcChanges {
        authentication: Some(fixture.store.configs()[0].authentication.clone()),
        ..PcChanges::default()
    };
    assert_eq!(
        fixture
            .controller
            .update_pc(&id, credentials)
            .unwrap_err()
            .0,
        "Credentials come only from pairing."
    );
}

#[tokio::test]
async fn delete_removes_locally_first_then_unpairs() {
    let fixture = fixture(FakeEnvironment::default());
    let id = online_pc(&fixture).await;
    let result = fixture.controller.delete_pc(&id).await.unwrap();
    assert!(result.unpaired);
    assert!(fixture.store.configs().is_empty() && fixture.controller.publish(false).pcs.is_empty());
    let fixture = self::fixture(FakeEnvironment::default());
    let id = fixture.add("PC").id.to_string();
    *fixture.environment().unpair.lock().unwrap() = Err(HostError::Connection);
    assert!(!fixture.controller.delete_pc(&id).await.unwrap().unpaired);
    assert!(fixture.store.configs().is_empty());
    assert_eq!(
        fixture.controller.delete_pc(&id).await.unwrap_err().0,
        "PC was not found."
    );
    assert_eq!(
        FakeEnvironment::count(&fixture.environment().calls.unpair),
        1
    );
}

#[tokio::test]
async fn malformed_and_missing_ids_have_fixed_errors() {
    let fixture = fixture(FakeEnvironment::default());
    for id in ["bad", "", "03C85317-9D93-4FB3-A04B-91A0E16668C6"] {
        assert_eq!(
            fixture.controller.start(id).await.unwrap_err().0,
            "Invalid PC identifier."
        );
        assert_eq!(
            fixture.controller.stop(id).await.unwrap_err().0,
            "Invalid PC identifier."
        );
    }
    let missing = Uuid::new_v4().to_string();
    assert_eq!(
        fixture.controller.start(&missing).await.unwrap_err().0,
        "PC was not found."
    );
    assert_eq!(
        fixture.controller.delete_pc(&missing).await.unwrap_err().0,
        "PC was not found."
    );
}

#[tokio::test]
async fn refresh_probes_every_pc_with_at_most_four_at_once() {
    let fixture = fixture(FakeEnvironment {
        status_delay: Duration::from_millis(30),
        ..FakeEnvironment::default()
    });
    for index in 0..9 {
        fixture.add(&format!("PC {index}"));
    }
    fixture.controller.refresh().await;
    let calls = &fixture.environment().calls;
    assert_eq!(FakeEnvironment::count(&calls.status), 9);
    assert!(FakeEnvironment::count(&calls.peak_status) > 1);
    assert!(FakeEnvironment::count(&calls.peak_status) <= 4);
    assert!(
        fixture
            .controller
            .publish(true)
            .pcs
            .iter()
            .all(|row| row.state == PowerState::Online)
    );
}

#[tokio::test]
async fn detect_mac_validates_the_host_first() {
    let fixture = fixture(FakeEnvironment::default());
    assert_eq!(
        fixture.controller.detect_mac("PC.local").unwrap_err().0,
        "Enter the PC's IPv4 address."
    );
    assert_eq!(
        fixture.controller.detect_mac("192.168.1.40").unwrap(),
        "02:00:00:00:00:01"
    );
}

#[tokio::test]
async fn live_environment_polls_and_test_stops_a_real_host() {
    use crate::{
        host_client::fixtures::{real_host, token},
        power::{Controller, LiveEnvironment},
        store::{Authentication, PcInput, Store},
    };
    let host = real_host(true).await;
    let directory = tempfile::TempDir::new().unwrap();
    let store =
        Arc::new(Store::open(directory.path(), crate::platform::fixtures::platform()).unwrap());
    let pc = store
        .add(
            PcInput {
                name: "Fixture PC".into(),
                host: "127.0.0.1".into(),
                port: Some(host.address.port().to_string()),
                mac: "02:00:00:00:00:01".into(),
            },
            Authentication {
                host_id: Uuid::parse_str(HOST_ID).unwrap(),
                token: token(1).into(),
                tls_pin: host.pin,
            },
        )
        .unwrap();
    let controller = Arc::new(Controller::new(
        store,
        LiveEnvironment(crate::platform::fixtures::platform()),
    ));
    controller.refresh().await;
    assert_eq!(controller.publish(true).pcs[0].state, PowerState::Online);
    assert_eq!(controller.stop(&pc.id.to_string()).await, Ok(true));
    assert_eq!(controller.publish(false).test_stops.count, 1);
}

#[tokio::test]
async fn an_unchanged_refresh_publishes_nothing_new() {
    // ! Every frame wakes the frontend through Decky, so a poll that changed
    //   nothing mustn't send one every 15 s.
    let fixture = fixture(FakeEnvironment::default());
    online_pc(&fixture).await;
    let action = fixture.controller.publish(false).revision;
    // * The first refresh after an action is always sent, which clears that
    //   action's error.
    let refreshed = fixture.controller.publish(true);
    assert_eq!(
        (refreshed.revision, refreshed.refreshed),
        (action + 1, true)
    );
    assert_eq!(fixture.controller.publish(true).revision, action + 1);
    assert_eq!(fixture.controller.publish(true).revision, action + 1);
}
