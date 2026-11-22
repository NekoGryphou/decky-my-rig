use super::{PcIssue, PcSnapshot, PowerState, SnapshotFrame, TestStops};
use serde_json::json;

#[test]
fn snapshot_frames_have_the_plugin_shape() {
    let frame = SnapshotFrame {
        revision: 3,
        refreshed: true,
        pcs: vec![
            PcSnapshot {
                id: "a".into(),
                name: "PC".into(),
                state: PowerState::Online,
                issue: None,
            },
            PcSnapshot {
                id: "b".into(),
                name: "Other".into(),
                state: PowerState::Checking,
                issue: Some(PcIssue::Authentication),
            },
        ],
        test_stops: TestStops {
            count: 0,
            name: None,
        },
        closed: false,
    };
    assert_eq!(
        serde_json::to_value(&frame).unwrap(),
        json!({"revision":3,"refreshed":true,"pcs":[
            {"id":"a","name":"PC","state":"online"},
            {"id":"b","name":"Other","state":"checking","issue":"authentication"}],
            "test_stops":{"count":0,"name":null},"closed":false})
    );
}
