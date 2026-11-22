use super::serve;
use crate::host_client::fixtures::real_host_with;
use serde_json::{Value, json};
use std::time::Duration;
use tempfile::TempDir;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, DuplexStream, Lines, duplex},
    task::JoinHandle,
    time::timeout,
};

struct Agent {
    _directory: TempDir,
    input: Option<DuplexStream>,
    output: Lines<BufReader<DuplexStream>>,
    task: JoinHandle<()>,
    next: u64,
    events: Vec<Value>,
}

impl Agent {
    fn start() -> Self {
        let directory = TempDir::new().unwrap();
        let (input, agent_input) = duplex(64 * 1024);
        let (agent_output, output) = duplex(64 * 1024);
        let store = directory.path().join("settings");
        let task = tokio::spawn(async move {
            serve(
                crate::platform::fixtures::platform(),
                &store,
                BufReader::new(agent_input),
                agent_output,
            )
            .await
            .unwrap();
        });
        Self {
            _directory: directory,
            input: Some(input),
            output: BufReader::new(output).lines(),
            task,
            next: 0,
            events: Vec::new(),
        }
    }

    async fn line(&mut self) -> Value {
        let line = timeout(Duration::from_secs(10), self.output.next_line())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        serde_json::from_str(&line).unwrap()
    }

    async fn send_bytes(&mut self, raw: &[u8]) {
        let input = self.input.as_mut().unwrap();
        input.write_all(raw).await.unwrap();
        input.write_all(b"\n").await.unwrap();
    }

    /// The next reply that is not a snapshot event.
    async fn reply(&mut self) -> Value {
        loop {
            let line = self.line().await;
            if line.get("event").is_none() {
                return line;
            }
        }
    }

    async fn send(&mut self, raw: &str) {
        let input = self.input.as_mut().unwrap();
        input.write_all(raw.as_bytes()).await.unwrap();
        input.write_all(b"\n").await.unwrap();
    }

    /// Sends one request and returns its reply, keeping events seen meanwhile.
    async fn call(&mut self, method: &str, params: Value) -> Value {
        self.next += 1;
        let id = self.next;
        self.send(&json!({"id":id,"method":method,"params":params}).to_string())
            .await;
        loop {
            let line = self.line().await;
            if line["id"] == json!(id) {
                return line;
            }
            self.events.push(line);
        }
    }

    /// Waits for a snapshot event whose frame satisfies `wanted`.
    async fn frame(&mut self, wanted: impl Fn(&Value) -> bool) -> Value {
        if let Some(event) = self.events.iter().find(|event| wanted(&event["frame"])) {
            return event["frame"].clone();
        }
        loop {
            let line = self.line().await;
            assert_eq!(line["event"], "snapshot", "{line}");
            if wanted(&line["frame"]) {
                return line["frame"].clone();
            }
        }
    }
}

#[tokio::test]
async fn pair_poll_stop_and_delete_through_the_agent() {
    let host = real_host_with(true, Some("12345678")).await;
    let port = host.address.port();
    let mut agent = Agent::start();
    assert_eq!(
        agent.call("settings", json!({})).await,
        json!({"id":1,"result":[]})
    );
    let draft = "aaaaaaaa-aaaa-4aaa-aaaa-aaaaaaaaaaaa";
    let begun = agent
        .call(
            "begin_pairing",
            json!({"draft_id":draft,"host":"127.0.0.1","port":port,"code":"12345678"}),
        )
        .await;
    assert_eq!(begun["result"]["name"], "Fixture PC", "{begun}");
    let saved = agent
        .call(
            "save_pairing",
            json!({"draft_id":draft,"name":"Desk","mac":"02:00:00:00:00:01"}),
        )
        .await;
    let id = saved["result"]["id"].as_str().unwrap().to_owned();
    assert_eq!(saved["result"]["port"], json!(port));
    let online = agent
        .frame(|frame| frame["pcs"][0]["state"] == "online")
        .await;
    assert_eq!(online["pcs"][0]["name"], "Desk");
    assert_eq!(
        agent.call("stop", json!({"identifier":id})).await["result"],
        json!(true)
    );
    let stopped = agent.frame(|frame| frame["test_stops"]["count"] == 1).await;
    assert_eq!(stopped["test_stops"]["name"], "Desk");
    let renamed = agent
        .call(
            "update_pc",
            json!({"identifier":id,"changes":{"name":"Office","port":port.to_string()}}),
        )
        .await;
    assert_eq!(renamed["result"]["name"], "Office");
    assert_eq!(
        agent.call("delete_pc", json!({"identifier":id})).await["result"],
        json!({"unpaired":true})
    );
    assert_eq!(agent.call("settings", json!({})).await["result"], json!([]));
}

#[tokio::test]
async fn bad_requests_get_an_error_and_the_agent_keeps_running() {
    let mut agent = Agent::start();
    let failure = "Something went wrong. Try again.";
    assert_eq!(
        agent.call("no_such_method", json!({})).await["error"],
        failure
    );
    assert_eq!(
        agent.call("start", json!({"identifier":7})).await["error"],
        failure
    );
    assert_eq!(
        agent.call("start", json!({"identifier":"bad"})).await["error"],
        "Invalid PC identifier."
    );
    assert_eq!(
        agent.call("detect_mac", json!({"host":"pc.local"})).await["error"],
        "Enter the PC's IPv4 address."
    );
    agent.send("not json").await;
    loop {
        let line = agent.line().await;
        if line.get("event").is_none() {
            assert_eq!(line, json!({"id":null,"error":failure}));
            break;
        }
    }
    assert_eq!(agent.call("settings", json!({})).await["result"], json!([]));
}

#[tokio::test]
async fn oversized_and_non_utf8_lines_are_refused_and_the_session_continues() {
    let mut agent = Agent::start();
    let failure = json!({"id":null,"error":"Something went wrong. Try again."});
    agent.send(&"x".repeat(1024 * 1024 + 1)).await;
    assert_eq!(agent.reply().await, failure);
    agent.send_bytes(b"{\"id\":1,\"method\":\"\xff\"}").await;
    assert_eq!(agent.reply().await, failure);
    assert_eq!(agent.call("settings", json!({})).await["result"], json!([]));
}

#[tokio::test]
async fn snapshots_arrive_without_requests() {
    let mut agent = Agent::start();
    let first = agent.frame(|_| true).await;
    assert_eq!(
        (first["pcs"].clone(), first["closed"].clone()),
        (json!([]), json!(false))
    );
}

#[tokio::test]
async fn stdin_eof_exits_after_joining_work() {
    let mut agent = Agent::start();
    agent.call("settings", json!({})).await;
    agent.input = None;
    timeout(Duration::from_secs(5), &mut agent.task)
        .await
        .unwrap()
        .unwrap();
}

#[test]
fn polling_slows_down_while_nothing_is_on_screen() {
    use super::{
        consts::{IDLE_POLL, STEADY_POLL, TRANSITION_POLL},
        utils::poll_interval,
    };
    // - A PC that's starting or stopping gets polled fast either way.
    assert_eq!(poll_interval(true, false), TRANSITION_POLL);
    assert_eq!(poll_interval(true, true), TRANSITION_POLL);
    assert_eq!(poll_interval(false, true), STEADY_POLL);
    // ? Nobody's looking during a game, so poll less and save TLS handshakes.
    assert_eq!(poll_interval(false, false), IDLE_POLL);
}

#[tokio::test]
async fn the_frontend_says_when_a_view_is_on_screen() {
    let mut agent = Agent::start();
    for visible in [true, false] {
        let reply = agent
            .call("set_attention", json!({ "visible": visible }))
            .await;
        assert_eq!(reply["result"], Value::Null, "{reply}");
    }
    assert_eq!(
        agent.call("set_attention", json!({"visible":"yes"})).await["error"],
        "Something went wrong. Try again."
    );
}
