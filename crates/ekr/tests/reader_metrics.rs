//! `story:readers-emit-metrics-and-request-logs`: `ekr view` and `ekr mcp-http` count each read
//! at their dispatch by wire name and outcome, serve the counts at `GET /metrics` when run with
//! `--metrics`, and write one `ekr.request-log/1` line per request to `--request-log`.
//!
//! Every case drives the built binary over real loopback connections on synthetic stores (the
//! example host and seed). No case asserts a duration's value: durations are counted.
//! `in_flight_returns_to_zero` holds a call open on the store's thread, which only a handler the
//! case owns can do, so it is a unit case of `crates/ekr/src/cli/http.rs`.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const ALICE: &str = "00000000-0000-4000-8000-000000000301";
const UNKNOWN: &str = "00000000-0000-4000-8000-000000000999";
const MARKED: &str = "00000000-0000-4000-8000-000000000911";
const TRANSACTION: &str = "00000000-0000-4000-8000-000000000912";
/// A string no metric and no log line may carry: the name of the node `create.yaml` creates,
/// and so a search's argument and its answer.
const MARKER: &str = "Zyxmarkerqv";
const TRACE: &str = "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01";

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// One provider in its own directory under the example host, seeded from the example seed.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn seeded(backend: &'static str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        world.file("host.json", &text(&["example", "ekr.cli-host/1"]));
        world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        world.file(
            "create.yaml",
            &format!(
                "format: ekr.transaction-document/2\ntransaction:\n  id: {TRANSACTION}\n  \
                 proposer: {OPERATOR}\n  operations:\n  - !CreateNode\n    id: {MARKED}\n    \
                 root_id: {ROOT}\n    type_id: {ORGANIZATION}\n    canonical_name: {MARKER}\n    \
                 properties: {{}}\n    aliases: [{MARKER}]\n  evidence: []\n"
            ),
        );
        world.ok(&["seed", "seed.yaml"]);
        world
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn log(&self) -> PathBuf {
        self.directory.path().join("requests.log")
    }

    fn file(&self, name: &str, contents: &str) {
        std::fs::write(self.directory.path().join(name), contents).unwrap();
    }

    fn command(&self, verb: &[&str]) -> Command {
        let mut command = ekr();
        command
            .current_dir(self.directory.path())
            .arg("--host")
            .arg(self.directory.path().join("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(verb);
        command
    }

    fn ok(&self, verb: &[&str]) -> String {
        let output = self.command(verb).stdin(Stdio::null()).output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    /// Proposes, validates and commits `create.yaml`, each as its own process.
    fn commit(&self) {
        let proposed: Value = serde_json::from_str(&self.ok(&["propose", "create.yaml"])).unwrap();
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        let validated: Value = serde_json::from_str(&self.ok(&["validate", &id])).unwrap();
        assert_eq!(validated["kind"], "Validated", "{validated}");
        let committed: Value = serde_json::from_str(&self.ok(&["commit", &id])).unwrap();
        assert_eq!(committed["kind"], "Committed", "{committed}");
    }

    /// `verb` (`view` or `mcp-http`) on a free loopback port, with `options` after it.
    fn serve(&self, verb: &'static str, options: &[&str]) -> Served {
        let mut arguments = vec![verb, "--port", "0"];
        arguments.extend_from_slice(options);
        Served::start(verb, self.command(&arguments))
    }

    /// `verb` with `--metrics` and `--request-log` naming this world's log.
    fn observed(&self, verb: &'static str) -> Served {
        let log = self.log();
        self.serve(verb, &["--metrics", "--request-log", log.to_str().unwrap()])
    }

    /// Every line of the request log, parsed.
    fn logged(&self) -> Vec<Value> {
        std::fs::read_to_string(self.log())
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap_or_else(|e| panic!("{e}: {line}")))
            .collect()
    }

    fn log_text(&self) -> String {
        std::fs::read_to_string(self.log()).unwrap_or_default()
    }
}

/// One answer: its status, its head and its body.
struct Response {
    status: u16,
    head: String,
    body: Vec<u8>,
}

impl Response {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body)
            .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&self.body)))
    }

    fn raw(&self) -> String {
        format!(
            "{}\r\n\r\n{}",
            self.head,
            String::from_utf8_lossy(&self.body)
        )
    }
}

/// A running read server.
struct Served {
    verb: &'static str,
    child: Child,
    authority: String,
}

impl Served {
    fn start(verb: &'static str, mut command: Command) -> Self {
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        assert!(!line.is_empty(), "{verb} exited before announcing its URL");
        let printed: Value = serde_json::from_str(&line).unwrap();
        let authority = printed["url"]
            .as_str()
            .unwrap()
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_owned();
        Self {
            verb,
            child,
            authority,
        }
    }

    fn raw(&self, request: &str) -> Response {
        let mut stream = TcpStream::connect(&self.authority).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        stream.write_all(request.as_bytes()).unwrap();
        let mut bytes = Vec::new();
        stream.read_to_end(&mut bytes).unwrap();
        let split = bytes
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .expect("a whole response head");
        let head = String::from_utf8(bytes[..split].to_vec()).unwrap();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        Response {
            status,
            head,
            body: bytes[split + 4..].to_vec(),
        }
    }

    /// `GET path` with `headers` (each ending `\r\n`) and this server's Host.
    fn get_with(&self, path: &str, headers: &str) -> Response {
        self.raw(&format!(
            "GET {path} HTTP/1.1\r\nHost: {}\r\n{headers}Connection: close\r\n\r\n",
            self.authority
        ))
    }

    fn get(&self, path: &str) -> Response {
        self.get_with(path, "")
    }

    /// One MCP `tools/call` of `tool` with `arguments` and `headers`.
    fn call_with(&self, tool: &str, arguments: &Value, headers: &str) -> Response {
        let body = json!({"jsonrpc":"2.0","id":7,"method":"tools/call",
            "params":{"name":tool,"arguments":arguments}})
        .to_string();
        self.raw(&format!(
            "POST /mcp HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nAccept: \
             application/json\r\nMCP-Protocol-Version: 2025-11-25\r\n{headers}Content-Length: \
             {}\r\n\r\n{body}",
            self.authority,
            body.len()
        ))
    }

    /// One read named by its MCP tool: through `tools/call` on `mcp-http`, and through the GET
    /// route that answers it on `view`.
    fn read(&self, tool: &str, arguments: &Value, route: &str) -> Response {
        self.read_with(tool, arguments, route, "")
    }

    fn read_with(&self, tool: &str, arguments: &Value, route: &str, headers: &str) -> Response {
        if self.verb == "view" {
            self.get_with(route, headers)
        } else {
            self.call_with(tool, arguments, headers)
        }
    }

    /// The answered document's text: an MCP tool result's text, or a view body.
    fn document(&self, response: &Response) -> String {
        assert_eq!(response.status, 200, "{}", response.raw());
        if self.verb == "view" {
            String::from_utf8(response.body.clone()).unwrap()
        } else {
            let answer = response.json();
            answer["result"]["content"][0]["text"]
                .as_str()
                .unwrap_or_else(|| panic!("no document: {answer}"))
                .to_owned()
        }
    }

    fn metrics(&self) -> String {
        let response = self.get("/metrics");
        assert_eq!(response.status, 200, "{}", response.raw());
        assert!(
            response
                .head
                .contains("Content-Type: text/plain; version=0.0.4; charset=utf-8"),
            "{}",
            response.head
        );
        String::from_utf8(response.body).unwrap()
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

/// The value of the one sample `series` (name and labels exactly as exposed), if exposed.
fn sample(exposition: &str, series: &str) -> Option<f64> {
    let mut found = exposition.lines().filter_map(|line| {
        line.strip_prefix(series)
            .and_then(|rest| rest.strip_prefix(' '))
            .map(|value| value.parse::<f64>().unwrap())
    });
    let first = found.next();
    assert!(found.next().is_none(), "{series} is exposed twice");
    first
}

fn requests(tool: &str, outcome: &str) -> String {
    format!("ekr_read_requests_total{{tool=\"{tool}\",outcome=\"{outcome}\"}}")
}

/// `describe_node` of `id`, and the `/node/<id>` route answering it as /2.
fn describe(id: &str) -> (Value, String) {
    (
        json!({"node": id}),
        format!("/node/{id}?format=ekr.node-detail/2"),
    )
}

fn search(text: &str) -> (Value, String) {
    (json!({"text": text}), format!("/search?q={text}"))
}

const VERBS: [&str; 2] = ["mcp-http", "view"];

#[test]
fn tool_call_counted_with_outcome() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        for verb in VERBS {
            let served = world.serve(verb, &["--metrics"]);
            let (arguments, route) = describe(ALICE);
            served.document(&served.read("describe_node", &arguments, &route));
            let (arguments, route) = describe(UNKNOWN);
            let refused = served.read("describe_node", &arguments, &route);
            assert!(
                refused.raw().contains("ekr.views.NodeNotFound"),
                "{}",
                refused.raw()
            );
            let metrics = served.metrics();
            for outcome in ["ok", "not_found"] {
                assert_eq!(
                    sample(&metrics, &requests("describe_node", outcome)),
                    Some(1.0),
                    "{backend} {verb} {outcome}:\n{metrics}"
                );
            }
            assert_eq!(
                metrics
                    .lines()
                    .filter(|line| line.starts_with("ekr_read_requests_total{"))
                    .count(),
                2,
                "{backend} {verb}: one series per outcome:\n{metrics}"
            );
        }
    }
}

#[test]
fn result_bytes_match_the_answer() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        for verb in VERBS {
            let served = world.serve(verb, &["--metrics"]);
            let (arguments, route) = search("Alice");
            let answered = served.document(&served.read("search", &arguments, &route));
            let metrics = served.metrics();
            assert_eq!(
                sample(&metrics, "ekr_read_result_bytes_sum{tool=\"search\"}"),
                Some(answered.len() as f64),
                "{backend} {verb}:\n{metrics}"
            );
            assert_eq!(
                sample(&metrics, "ekr_read_result_bytes_count{tool=\"search\"}"),
                Some(1.0),
                "{backend} {verb}:\n{metrics}"
            );
        }
        // A streamed /expand counts its NDJSON lines, not its framing: asked over HTTP/1.0, the
        // body is the lines bare.
        let served = world.serve("view", &["--metrics"]);
        let streamed = served.raw(&format!(
            "GET /expand?seeds={ALICE}&depth=1&limit=10 HTTP/1.0\r\nHost: {}\r\n\r\n",
            served.authority
        ));
        assert_eq!(streamed.status, 200, "{}", streamed.raw());
        let metrics = served.metrics();
        assert_eq!(
            sample(&metrics, "ekr_read_result_bytes_sum{tool=\"expand\"}"),
            Some(streamed.body.len() as f64),
            "{backend}:\n{metrics}"
        );
    }
}

#[test]
fn duration_recorded_per_tool() {
    let world = World::seeded("sqlite");
    for verb in VERBS {
        let served = world.serve(verb, &["--metrics"]);
        let (arguments, route) = search("Alice");
        for _ in 0..3 {
            served.document(&served.read("search", &arguments, &route));
        }
        served.document(&served.read("head", &json!({}), "/head"));
        let metrics = served.metrics();
        for (tool, count) in [("search", 3.0), ("head", 1.0)] {
            for series in [
                format!("ekr_read_duration_seconds_count{{tool=\"{tool}\"}}"),
                format!("ekr_read_duration_seconds_bucket{{tool=\"{tool}\",le=\"+Inf\"}}"),
            ] {
                assert_eq!(
                    sample(&metrics, &series),
                    Some(count),
                    "{verb} {series}:\n{metrics}"
                );
            }
        }
    }
}

#[test]
fn head_revision_is_the_served_revision() {
    for backend in BACKENDS {
        for verb in VERBS {
            let world = World::seeded(backend);
            let served = world.serve(verb, &["--metrics"]);
            served.document(&served.read("head", &json!({}), "/head"));
            let before = served.metrics();
            assert_eq!(
                sample(&before, "ekr_store_head_revision"),
                Some(0.0),
                "{backend} {verb}:\n{before}"
            );
            world.commit();
            let (arguments, route) = search("Alice");
            served.document(&served.read("search", &arguments, &route));
            let metrics = served.metrics();
            assert_eq!(
                sample(&metrics, "ekr_store_head_revision"),
                Some(1.0),
                "{backend} {verb}:\n{metrics}"
            );
        }
    }
}

/// Restores a directory's mode on drop, so a failing case leaves a removable tree behind.
struct Restore(PathBuf, std::fs::Permissions);

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, self.1.clone());
    }
}

#[test]
fn store_query_error_counted() {
    use std::os::unix::fs::PermissionsExt as _;
    // A file store whose directory this process may not read fails every read on the store
    // while its identity stays the one the reader opened. A process that reads it anyway (root)
    // cannot make the read fail this way.
    let world = World::seeded("file");
    for verb in VERBS {
        let served = world.serve(verb, &["--metrics"]);
        served.document(&served.read("head", &json!({}), "/head"));
        let store = world.store();
        let mode = std::fs::metadata(&store).unwrap().permissions();
        let _restore = Restore(store.clone(), mode);
        std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o000)).unwrap();
        if std::fs::read_dir(&store).is_ok() {
            eprintln!("skipped: this process reads a directory of mode 000, so no read fails");
            return;
        }
        let failed = served.read("head", &json!({}), "/head");
        drop(_restore);
        if verb == "view" {
            assert_eq!(failed.status, 500, "{}", failed.raw());
        } else {
            assert_eq!(failed.json()["error"]["code"], -32603, "{}", failed.raw());
        }
        let metrics = served.metrics();
        assert_eq!(
            sample(&metrics, "ekr_store_query_errors_total{tool=\"head\"}"),
            Some(1.0),
            "{verb}:\n{metrics}"
        );
        assert_eq!(
            sample(&metrics, &requests("head", "store_error")),
            Some(1.0),
            "{verb}:\n{metrics}"
        );
        for refusal in ["invalid_input", "not_seeded", "not_found", "unavailable"] {
            assert_eq!(
                sample(&metrics, &requests("head", refusal)),
                None,
                "{verb} {refusal}:\n{metrics}"
            );
        }
    }
}

#[test]
fn request_log_carries_incoming_ids() {
    for backend in BACKENDS {
        for verb in VERBS {
            let world = World::seeded(backend);
            let served = world.observed(verb);
            let (arguments, route) = search("Alice");
            served.document(&served.read_with(
                "search",
                &arguments,
                &route,
                &format!("X-Request-Id: req-41.a:b_c\r\ntraceparent: {TRACE}\r\n"),
            ));
            let logged = world.logged();
            assert_eq!(logged.len(), 1, "{backend} {verb}: {logged:?}");
            let line = &logged[0];
            assert_eq!(line["format"], "ekr.request-log/1", "{line}");
            assert_eq!(line["server"], verb, "{line}");
            assert_eq!(line["request_id"], "req-41.a:b_c", "{line}");
            assert_eq!(line["request_id_source"], "incoming", "{line}");
            assert_eq!(line["traceparent"], TRACE, "{line}");
            assert_eq!(line["traceparent_source"], "incoming", "{line}");
            assert_eq!(line["tool"], "search", "{line}");
            assert_eq!(line["outcome"], "ok", "{line}");
            assert_eq!(line["status"], 200, "{line}");
            assert!(line["duration_seconds"].is_number(), "{line}");
            assert!(line["result_bytes"].as_u64().unwrap() > 0, "{line}");

            served.document(&served.read("search", &arguments, &route));
            let logged = world.logged();
            assert_eq!(logged.len(), 2, "{backend} {verb}: {logged:?}");
            let line = &logged[1];
            assert_eq!(line["request_id_source"], "generated", "{line}");
            assert_eq!(line["traceparent_source"], "generated", "{line}");
            let generated = line["traceparent"].as_str().unwrap();
            assert_eq!(generated.len(), 55, "{line}");
            assert!(generated.starts_with("00-"), "{line}");
            assert_ne!(line["request_id"], logged[0]["request_id"], "{line}");
            assert!(!line["request_id"].as_str().unwrap().is_empty(), "{line}");
        }
    }
}

#[test]
fn malformed_trace_header_is_replaced() {
    let oversized = format!("00-{}-b7ad6b7169203331-01", "a".repeat(400));
    let world = World::seeded("sqlite");
    for verb in VERBS {
        let served = world.observed(verb);
        for (traceparent, request_id) in [
            (oversized.as_str(), "fine-id"),
            ("not-a-trace-context", "spaces are not an id"),
            (
                "00-00000000000000000000000000000000-b7ad6b7169203331-01",
                &*"r".repeat(129),
            ),
        ] {
            let before = world.logged().len();
            let (arguments, route) = search("Alice");
            let answered = served.read_with(
                "search",
                &arguments,
                &route,
                &format!("X-Request-Id: {request_id}\r\ntraceparent: {traceparent}\r\n"),
            );
            assert_eq!(answered.status, 200, "{}", answered.raw());
            assert!(
                !answered.raw().contains(traceparent),
                "{verb}: the answer echoes {traceparent}"
            );
            let logged = world.logged();
            assert_eq!(logged.len(), before + 1, "{verb}: {logged:?}");
            let line = &logged[before];
            assert_eq!(line["traceparent_source"], "replaced", "{line}");
            assert_ne!(line["traceparent"], traceparent, "{line}");
            assert_eq!(line["traceparent"].as_str().unwrap().len(), 55, "{line}");
            assert!(!world.log_text().contains(traceparent), "{verb}");
            if request_id == "fine-id" {
                assert_eq!(line["request_id"], "fine-id", "{line}");
            } else {
                assert_eq!(line["request_id_source"], "replaced", "{line}");
                assert!(!world.log_text().contains(request_id), "{verb}");
            }
        }
    }
}

#[test]
fn metrics_hold_no_arguments_or_results() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        world.commit();
        for verb in VERBS {
            let served = world.observed(verb);
            let (arguments, route) = search(MARKER);
            let answered = served.document(&served.read("search", &arguments, &route));
            assert!(answered.contains(MARKER), "precondition: {answered}");
            let (arguments, route) = describe(MARKED);
            let answered = served.document(&served.read("describe_node", &arguments, &route));
            assert!(answered.contains(MARKER), "precondition: {answered}");
            let metrics = served.metrics();
            assert_eq!(
                sample(&metrics, &requests("search", "ok")),
                Some(1.0),
                "{metrics}"
            );
            assert!(!metrics.contains(MARKER), "{backend} {verb}:\n{metrics}");
            assert!(!metrics.contains(MARKED), "{backend} {verb}:\n{metrics}");
            let log = world.log_text();
            assert!(!log.is_empty());
            assert!(!log.contains(MARKER), "{backend} {verb}:\n{log}");
            assert!(!log.contains(MARKED), "{backend} {verb}:\n{log}");
        }
    }
}

#[test]
fn metrics_endpoint_off_by_default_and_host_checked() {
    let world = World::seeded("sqlite");
    for verb in VERBS {
        let served = world.serve(verb, &[]);
        assert_eq!(
            served.get("/metrics").status,
            404,
            "{verb} without --metrics"
        );

        let served = world.serve(verb, &["--metrics"]);
        let exposition = served.metrics();
        assert!(
            exposition.contains("# TYPE ekr_read_requests_total counter"),
            "{exposition}"
        );
        assert_eq!(sample(&exposition, "ekr_read_in_flight"), Some(0.0));
        let elsewhere = served.raw(
            "GET /metrics HTTP/1.1\r\nHost: elsewhere.example:80\r\nConnection: close\r\n\r\n",
        );
        assert_eq!(elsewhere.status, 421, "{verb}: {}", elsewhere.raw());
        assert!(!elsewhere.raw().contains("ekr_read"), "{}", elsewhere.raw());
        let posted = served.raw(&format!(
            "POST /metrics HTTP/1.1\r\nHost: {}\r\nContent-Length: 0\r\n\r\n",
            served.authority
        ));
        assert_eq!(posted.status, 405, "{verb}: {}", posted.raw());
    }
}

#[test]
fn view_and_mcp_count_alike() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let view = world.serve("view", &["--metrics"]);
        let mcp = world.serve("mcp-http", &["--metrics"]);
        for served in [&view, &mcp] {
            let (arguments, route) = search("Alice");
            served.document(&served.read("search", &arguments, &route));
            let (arguments, route) = describe(ALICE);
            served.document(&served.read("describe_node", &arguments, &route));
            served.document(&served.read(
                "changes_since",
                &json!({"since_revision": 0}),
                "/changes?since_revision=0",
            ));
        }
        let counted = |served: &Served| {
            served
                .metrics()
                .lines()
                .filter(|line| line.starts_with("ekr_read_requests_total{"))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        let viewed = counted(&view);
        assert_eq!(viewed, counted(&mcp), "{backend}");
        assert_eq!(viewed.len(), 3, "{backend}: {viewed:?}");
        for tool in ["search", "describe_node", "changes_since"] {
            let line = format!("{} 1", requests(tool, "ok"));
            assert!(viewed.contains(&line), "{backend}: {line} in {viewed:?}");
        }
    }
}
