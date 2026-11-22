use super::*;
use std::{
    collections::VecDeque,
    io,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};
use tokio::io::DuplexStream;

#[derive(Default)]
struct RecordingNotifier(Mutex<Vec<(String, String)>>, AtomicUsize, bool);
impl Notifier for RecordingNotifier {
    fn shows_pairing_codes(&self) -> bool {
        !self.2
    }
    fn unauthorized(&self) {
        self.1.fetch_add(1, Ordering::Relaxed);
    }
    fn notify(&self, title: &str, body: &str) {
        self.0.lock().unwrap().push((title.into(), body.into()));
    }
}

fn event(sequence: u64, name: &str) -> serde_json::Value {
    serde_json::json!({"sequence":sequence,"kind":"test_shutdown","device_name":name,"time_unix":1})
}
fn pairing_event(sequence: u64, name: &str) -> serde_json::Value {
    serde_json::json!({"sequence":sequence,"kind":"pairing_request","device_name":name,"time_unix":1})
}
fn pairing_code(code: &str) -> Vec<u8> {
    serde_json::json!({"version":1,"ok":true,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","code":code,"expires_in_seconds":120}).to_string().into_bytes()
}
fn response(events: Vec<serde_json::Value>) -> Vec<u8> {
    let baseline_sequence = events
        .last()
        .and_then(|event| event["sequence"].as_u64())
        .unwrap_or(0);
    response_with_baseline(baseline_sequence, events)
}
fn response_with_baseline(baseline_sequence: u64, events: Vec<serde_json::Value>) -> Vec<u8> {
    serde_json::json!({"version":1,"ok":true,"host_id":"03c85317-9d93-4fb3-a04b-91a0e16668c6","baseline_sequence":baseline_sequence,"events":events}).to_string().into_bytes()
}
async fn exercise(
    plans: Vec<Result<Vec<u8>, io::ErrorKind>>,
) -> (Vec<(String, String)>, Vec<Duration>, Vec<u64>, usize) {
    exercise_with(plans, RecordingNotifier::default()).await
}

async fn exercise_with(
    plans: Vec<Result<Vec<u8>, io::ErrorKind>>,
    notifier: RecordingNotifier,
) -> (Vec<(String, String)>, Vec<Duration>, Vec<u64>, usize) {
    let plans = Arc::new(Mutex::new(VecDeque::from(plans)));
    let delays = Arc::new(Mutex::new(Vec::new()));
    let afters = Arc::new(Mutex::new(Vec::new()));
    run(
        || {
            let plan = plans.lock().unwrap().pop_front();
            let afters = afters.clone();
            async move {
                let Some(plan) = plan else {
                    return Ok(None);
                };
                let reply = plan.map_err(io::Error::from)?;
                let (client, mut server) = tokio::io::duplex(8192);
                tokio::spawn(async move {
                    let request =
                        myrig_protocol::local_control::framing::read_frame(&mut server, 1024)
                            .await
                            .unwrap();
                    let value: serde_json::Value = serde_json::from_slice(&request).unwrap();
                    if let Some(after) = value["after"].as_u64() {
                        afters.lock().unwrap().push(after);
                    }
                    myrig_protocol::local_control::framing::write_frame(&mut server, &reply, 8192)
                        .await
                        .unwrap();
                });
                Ok::<Option<DuplexStream>, io::Error>(Some(client))
            }
        },
        &notifier,
        |duration| {
            delays.lock().unwrap().push(duration);
            std::future::ready(())
        },
    )
    .await;
    let denials = notifier.1.load(Ordering::Relaxed);
    let notifications = notifier.0.into_inner().unwrap();
    let delays = delays.lock().unwrap().clone();
    let afters = afters.lock().unwrap().clone();
    (notifications, delays, afters, denials)
}

#[tokio::test]
async fn notifier_notifies_each_new_event_with_exact_text() {
    let (notices, _, afters, _) = exercise(vec![
        Ok(response(vec![event(1, "Old device")])),
        Ok(response(vec![event(2, "Device A"), event(3, "Device B")])),
    ])
    .await;
    assert_eq!(afters, vec![0, 1]);
    assert_eq!(
        notices,
        vec![
            (
                "Test shutdown from “Device A”.".into(),
                "You can turn off the test mode in the settings.".into()
            ),
            (
                "Test shutdown from “Device B”.".into(),
                "You can turn off the test mode in the settings.".into()
            ),
        ]
    );
}

#[tokio::test]
async fn notifier_notifies_event_arriving_during_first_wait() {
    let (notices, _, _, _) = exercise(vec![Ok(response_with_baseline(
        0,
        vec![event(1, "Live device")],
    ))])
    .await;
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].0, "Test shutdown from “Live device”.");
}

#[tokio::test]
async fn notifier_backs_off_and_resets_after_success() {
    let (_, delays, _, _) = exercise(vec![
        Err(io::ErrorKind::NotFound),
        Err(io::ErrorKind::ConnectionRefused),
        Err(io::ErrorKind::Interrupted),
        Ok(response(vec![])),
        Err(io::ErrorKind::NotFound),
    ])
    .await;
    assert_eq!(
        delays,
        vec![
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4),
            Duration::from_secs(1)
        ]
    );
}

#[tokio::test]
async fn notifier_backoff_caps_at_thirty_seconds() {
    let (_, delays, _, _) = exercise(vec![Err(io::ErrorKind::NotFound); 8]).await;
    assert_eq!(
        delays,
        vec![
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4),
            Duration::from_secs(8),
            Duration::from_secs(16),
            Duration::from_secs(30),
            Duration::from_secs(30),
            Duration::from_secs(30),
        ]
    );
}

#[tokio::test]
async fn notifier_reconnect_preserves_cursor_and_notifies_outage_events() {
    let (notices, _, afters, _) = exercise(vec![
        Ok(response(vec![event(3, "Before")])),
        Err(io::ErrorKind::ConnectionRefused),
        Ok(response(vec![event(10, "During outage")])),
        Ok(response(vec![event(11, "After"), event(12, "Next")])),
    ])
    .await;
    assert_eq!(afters, vec![0, 3, 10]);
    assert_eq!(notices.len(), 3);
    assert_eq!(notices[0].0, "Test shutdown from “During outage”.");
    assert_eq!(notices[1].0, "Test shutdown from “After”.");
    assert_eq!(notices[2].0, "Test shutdown from “Next”.");
}

#[tokio::test]
async fn notifier_reports_repeated_pipe_denial_once_and_keeps_backing_off() {
    let (notices, delays, _, denials) =
        exercise(vec![Err(io::ErrorKind::PermissionDenied); 3]).await;
    assert!(notices.is_empty());
    assert_eq!(denials, 1);
    assert_eq!(
        delays,
        vec![
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4)
        ]
    );
}

#[tokio::test]
async fn notifier_reports_repeated_unauthorized_replies_once() {
    let reply = br#"{"version":1,"ok":false,"error":"unauthorized"}"#.to_vec();
    let (notices, delays, _, denials) = exercise(vec![Ok(reply); 3]).await;
    assert!(notices.is_empty());
    assert_eq!(denials, 1);
    assert_eq!(
        delays,
        vec![
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(4)
        ]
    );
}

#[tokio::test]
async fn notifier_does_not_report_transport_errors_as_authorization_failures() {
    let (_, _, _, denials) = exercise(vec![
        Err(io::ErrorKind::NotFound),
        Err(io::ErrorKind::ConnectionRefused),
    ])
    .await;
    assert_eq!(denials, 0);
}

#[tokio::test]
async fn notifier_reports_a_new_denial_after_a_valid_reply() {
    let (_, delays, _, denials) = exercise(vec![
        Err(io::ErrorKind::PermissionDenied),
        Ok(response(vec![])),
        Err(io::ErrorKind::PermissionDenied),
    ])
    .await;
    assert_eq!(denials, 2);
    assert_eq!(delays, vec![Duration::from_secs(1), Duration::from_secs(1)]);
}

#[tokio::test]
async fn notifier_keeps_denial_deduplication_across_other_errors() {
    let (_, _, _, denials) = exercise(vec![
        Err(io::ErrorKind::PermissionDenied),
        Err(io::ErrorKind::NotFound),
        Ok(b"{".to_vec()),
        Err(io::ErrorKind::PermissionDenied),
    ])
    .await;
    assert_eq!(denials, 1);
}

#[tokio::test]
async fn notifier_bad_reply_preserves_cursor_for_the_next_connection() {
    let (notices, _, afters, _) = exercise(vec![
        Ok(response(vec![event(5, "Before")])),
        Ok(br#"{"version":1,"ok":false,"error":"unavailable"}"#.to_vec()),
        Ok(response(vec![event(6, "During outage")])),
    ])
    .await;
    assert_eq!(afters, vec![0, 5, 5]);
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0].0, "Test shutdown from “During outage”.");
}

#[tokio::test]
async fn notifier_lower_baseline_delivers_the_restarted_services_events_once() {
    let (notices, _, afters, _) = exercise(vec![
        Ok(response(vec![event(10, "Before restart")])),
        Err(io::ErrorKind::NotFound),
        Ok(response(vec![event(1, "New A"), event(2, "New B")])),
        Ok(response_with_baseline(
            2,
            vec![event(2, "New B"), event(3, "New C")],
        )),
    ])
    .await;
    assert_eq!(afters, vec![0, 10, 2]);
    assert_eq!(
        notices.iter().map(|n| n.0.as_str()).collect::<Vec<_>>(),
        vec![
            "Test shutdown from “New A”.",
            "Test shutdown from “New B”.",
            "Test shutdown from “New C”."
        ]
    );
}

#[tokio::test]
async fn notifier_shows_the_code_for_a_pairing_request() {
    let (notices, _, afters, _) = exercise(vec![
        Ok(response_with_baseline(
            0,
            vec![pairing_event(1, "steamdeck")],
        )),
        Ok(pairing_code("48210937")),
    ])
    .await;
    assert_eq!(afters, vec![0]);
    assert_eq!(
        notices,
        vec![(
            "Pairing code: 48210937".into(),
            "“steamdeck” wants to pair with this PC.".into()
        )]
    );
}

#[tokio::test]
async fn notifier_shows_nothing_when_the_code_cannot_be_opened() {
    let (notices, _, afters, _) = exercise(vec![
        Ok(response_with_baseline(
            0,
            vec![pairing_event(1, "steamdeck")],
        )),
        Ok(br#"{"version":1,"ok":false,"error":"unavailable"}"#.to_vec()),
        Ok(response(vec![])),
    ])
    .await;
    assert!(notices.is_empty());
    assert_eq!(afters, vec![0, 1]);
}

#[tokio::test]
async fn notifier_outside_the_console_session_opens_no_code() {
    // ! A second open from another session would replace the code shown on the
    //   PC.
    let (notices, _, afters, _) = exercise_with(
        vec![
            Ok(response_with_baseline(
                0,
                vec![pairing_event(1, "steamdeck")],
            )),
            Ok(response(vec![])),
        ],
        RecordingNotifier(Mutex::default(), AtomicUsize::default(), true),
    )
    .await;
    assert!(notices.is_empty());
    assert_eq!(afters, vec![0, 1]);
}
