//! `story:mcp-read-tools`: `ekr mcp` serves canonical state to an agent as read-only MCP tools.
//!
//! The server speaks JSON-RPC 2.0 over stdio, one message per line. Every case drives the built
//! binary over piped stdin and stdout, on both providers, under the example host and seed:
//!
//! * `initialize` then `tools/list` names exactly seven tools, each with a JSON Schema
//!   `inputSchema`, none of which proposes, validates or commits;
//! * each tool's document is the one its read returns on the same store state: the
//!   `ekr_views::Index` answer byte for byte for `overview`, `search`, `describe_node`, `expand`
//!   and `timeline`, and what `ekr explain` and `ekr resolve` print for `explain` and `resolve`;
//! * a refusal is a tool result with `isError` carrying the refusal's name, and the server keeps
//!   serving; a malformed message, an unknown method, bad arguments and an unknown tool are
//!   JSON-RPC errors;
//! * a commit made by another process is what the next tool call reads;
//! * 100 `search` and 100 `describe_node` calls over one server are timed and printed.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Write as _};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Output, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use ekr::host::CliHostConfigurationV1;
use ekr_core::{NodeId, RevisionNumber, TypeId};
use ekr_kernel::Runtime;
use ekr_views::{
    BucketWidth, ExpandRequest, Index, OverviewRequest, ProjectError, SearchRequest,
    TimelineRequest,
};
use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const PERSON: &str = "00000000-0000-4000-8000-000000000201";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const ALICE: &str = "00000000-0000-4000-8000-000000000301";
const ACME: &str = "00000000-0000-4000-8000-000000000303";
const SEEDED_ASSERTION: &str = "00000000-0000-4000-8000-000000000510";
const GLOBEX: &str = "00000000-0000-4000-8000-000000000901";
const TRANSACTION: &str = "00000000-0000-4000-8000-000000000902";
const MANY: &str = "00000000-0000-4000-8000-000000000903";
const UNKNOWN: &str = "00000000-0000-4000-8000-000000000999";

/// The seven tools, and nothing else.
const TOOLS: [&str; 7] = [
    "describe_node",
    "expand",
    "explain",
    "overview",
    "resolve",
    "search",
    "timeline",
];

/// How long one answer may take before the case fails instead of hanging.
const ANSWER_WITHIN: Duration = Duration::from_secs(60);

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

/// Exit 0 and stdout as text.
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

/// A transaction of `CreateNode`s, each an instance of `type_id` known by its one alias.
fn create(transaction: &str, type_id: &str, nodes: &[(String, String)]) -> String {
    let mut document = format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {transaction}\n  proposer: \
         {OPERATOR}\n  operations:\n"
    );
    for (id, name) in nodes {
        document.push_str(&format!(
            "  - !CreateNode\n    id: {id}\n    root_id: {ROOT}\n    type_id: {type_id}\n    \
             canonical_name: {name}\n    properties: {{}}\n    aliases: [{name}]\n"
        ));
    }
    document.push_str("  evidence: []\n");
    document
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
            &create(
                TRANSACTION,
                ORGANIZATION,
                &[(GLOBEX.to_owned(), "Globex".to_owned())],
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

    fn file(&self, name: &str, contents: &str) {
        std::fs::write(self.directory.path().join(name), contents).unwrap();
    }

    /// The store configuration as flags, then `verb`, run from the world's directory.
    fn command(&self, verb: &[&str]) -> std::process::Command {
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

    fn run(&self, verb: &[&str]) -> Output {
        self.command(verb).stdin(Stdio::null()).output().unwrap()
    }

    /// A one-shot verb that must exit 0: its stdout, as printed.
    fn ok(&self, verb: &[&str]) -> String {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    /// Proposes, validates and commits the document in `file`, each as its own process.
    fn commit(&self, file: &str) {
        let proposed: Value = serde_json::from_str(&self.ok(&["propose", file])).unwrap();
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        let validated: Value = serde_json::from_str(&self.ok(&["validate", &id])).unwrap();
        assert_eq!(validated["kind"], "Validated", "{validated}");
        let committed: Value = serde_json::from_str(&self.ok(&["commit", &id])).unwrap();
        assert_eq!(committed["kind"], "Committed", "{committed}");
    }

    /// The store opened in this process, as the binary opens it.
    fn runtime(&self) -> Runtime {
        let host = CliHostConfigurationV1::from_json(
            &std::fs::read(self.directory.path().join("host.json")).unwrap(),
        )
        .unwrap();
        match self.backend {
            "file" => {
                Runtime::file_existing(&self.store(), &host.tenant, host.context, host.authority)
            }
            _ => {
                Runtime::sqlite_existing(&self.store(), &host.tenant, host.context, host.authority)
            }
        }
        .unwrap()
    }

    /// The `ekr.views` index of revision `at` (the head when `None`), loaded in this process.
    fn index(&self, at: Option<u64>) -> Index {
        Index::load(&self.runtime(), at.map(RevisionNumber::new)).unwrap()
    }

    /// `ekr mcp` started and initialized.
    fn server(&self) -> Server {
        let mut server = Server::start(self.command(&["mcp"]));
        server.initialize();
        server
    }
}

/// A running `ekr mcp`: requests written one line at a time, each answer read before the next.
struct Server {
    child: Child,
    input: ChildStdin,
    lines: Receiver<String>,
    next_id: i64,
}

impl Server {
    fn start(mut command: std::process::Command) -> Self {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let input = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (send, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { return };
                if send.send(line).is_err() {
                    return;
                }
            }
        });
        Self {
            child,
            input,
            lines,
            next_id: 0,
        }
    }

    /// Writes one line as it is.
    fn send_line(&mut self, line: &str) {
        self.input.write_all(line.as_bytes()).unwrap();
        self.input.write_all(b"\n").unwrap();
        self.input.flush().unwrap();
    }

    /// The next line the server writes, as JSON.
    fn answer(&self) -> Value {
        let line = self
            .lines
            .recv_timeout(ANSWER_WITHIN)
            .expect("the server answers within the minute");
        assert!(!line.contains('\n'));
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("{e}: {line}"))
    }

    /// One raw line and the one message it is answered with.
    fn raw(&mut self, line: &str) -> Value {
        self.send_line(line);
        self.answer()
    }

    /// A request with a fresh id; its response, whose id and version it checks.
    fn request(&mut self, method: &str, params: Option<Value>) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let mut message = json!({"jsonrpc": "2.0", "id": id, "method": method});
        if let Some(params) = params {
            message["params"] = params;
        }
        let response = self.raw(&message.to_string());
        assert_eq!(response["jsonrpc"], "2.0", "{response}");
        assert_eq!(response["id"], id, "{response}");
        response
    }

    /// A notification: nothing is answered.
    fn notify(&mut self, method: &str) {
        self.send_line(&json!({"jsonrpc": "2.0", "method": method}).to_string());
    }

    fn initialize(&mut self) -> Value {
        let response = self.request(
            "initialize",
            Some(json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "mcp-test", "version": "0"}
            })),
        );
        self.notify("notifications/initialized");
        response
    }

    /// `tools/call` of `tool`: the result of a call that answered, or the JSON-RPC error.
    fn call(&mut self, tool: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            Some(json!({"name": tool, "arguments": arguments})),
        )
    }

    /// A call that answered with a document: its text, checked against its structured copy.
    fn document(&mut self, tool: &str, arguments: Value) -> String {
        let response = self.call(tool, arguments.clone());
        let result = &response["result"];
        assert_eq!(result["isError"], false, "{tool} {arguments}: {response}");
        document_text(result)
    }

    /// A call refused by name: the refusal object it carries.
    fn refusal(&mut self, tool: &str, arguments: Value) -> Value {
        let response = self.call(tool, arguments.clone());
        let result = &response["result"];
        assert_eq!(result["isError"], true, "{tool} {arguments}: {response}");
        let refusal: Value = serde_json::from_str(&document_text(result)).unwrap();
        assert!(refusal["refusal"].is_string(), "{refusal}");
        assert!(refusal["message"].is_string(), "{refusal}");
        refusal
    }

    /// The JSON-RPC error code a request is answered with.
    fn error_code(response: &Value) -> i64 {
        assert!(response.get("result").is_none(), "{response}");
        response["error"]["code"]
            .as_i64()
            .unwrap_or_else(|| panic!("no error code: {response}"))
    }

    /// Ends input and waits: exit 0 and nothing on stderr.
    fn close(self) {
        let Self {
            mut child, input, ..
        } = self;
        drop(input);
        let status = child.wait().unwrap();
        let mut stderr = String::new();
        std::io::Read::read_to_string(&mut child.stderr.take().unwrap(), &mut stderr).unwrap();
        assert_eq!(status.code(), Some(0), "{stderr}");
        assert!(
            stderr.is_empty(),
            "the server writes messages only: {stderr}"
        );
    }
}

/// A tool result's one text content, which its `structuredContent` must equal as JSON.
fn document_text(result: &Value) -> String {
    let content = result["content"].as_array().expect("content is a list");
    assert_eq!(content.len(), 1, "{result}");
    assert_eq!(content[0]["type"], "text", "{result}");
    let text = content[0]["text"].as_str().unwrap().to_owned();
    let parsed: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(result["structuredContent"], parsed, "{result}");
    text
}

fn utf8(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap()
}

fn node(id: &str) -> NodeId {
    id.parse().unwrap()
}

// 1 --------------------------------------------------------------------------------------------

#[test]
fn initialize_then_tools_list_names_exactly_the_seven_read_tools_with_input_schemas() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let mut server = Server::start(world.command(&["mcp"]));
        let initialized = server.initialize();
        let result = &initialized["result"];
        assert_eq!(result["protocolVersion"], "2025-11-25", "{initialized}");
        assert_eq!(result["serverInfo"]["name"], "ekr");
        assert_eq!(result["serverInfo"]["version"], env!("CARGO_PKG_VERSION"));
        assert!(result["capabilities"]["tools"].is_object(), "{initialized}");
        assert!(
            result["instructions"]
                .as_str()
                .is_some_and(|text| text.contains("untrusted")),
            "{initialized}"
        );
        // A version the server also speaks is echoed; one it does not is answered with its own.
        for (asked, answered) in [
            ("2025-06-18", "2025-06-18"),
            ("2025-03-26", "2025-03-26"),
            ("2024-11-05", "2024-11-05"),
            ("1999-01-01", "2025-11-25"),
        ] {
            let again = server.request(
                "initialize",
                Some(json!({"protocolVersion": asked, "capabilities": {}})),
            );
            assert_eq!(again["result"]["protocolVersion"], answered, "{again}");
        }
        let pinged = server.request("ping", None);
        assert_eq!(pinged["result"], json!({}), "{pinged}");

        let listed = server.request("tools/list", None);
        let tools = listed["result"]["tools"].as_array().unwrap();
        let names: BTreeSet<&str> = tools
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, TOOLS.into_iter().collect(), "{listed}");
        assert_eq!(tools.len(), TOOLS.len(), "a tool is listed twice: {listed}");
        for tool in tools {
            let schema = &tool["inputSchema"];
            assert_eq!(schema["type"], "object", "{tool}");
            assert!(schema["properties"].is_object(), "{tool}");
            assert_eq!(schema["additionalProperties"], false, "{tool}");
            for required in schema["required"].as_array().into_iter().flatten() {
                assert!(
                    schema["properties"][required.as_str().unwrap()].is_object(),
                    "{tool}: required {required} is not a property"
                );
            }
            assert!(
                tool["description"].as_str().is_some_and(|d| !d.is_empty()),
                "{tool}"
            );
            assert_eq!(tool["annotations"]["readOnlyHint"], true, "{tool}");
        }
        let required = |name: &str| -> BTreeSet<String> {
            let tool = tools.iter().find(|tool| tool["name"] == name).unwrap();
            tool["inputSchema"]["required"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|r| r.as_str().unwrap().to_owned())
                .collect()
        };
        let set = |names: &[&str]| -> BTreeSet<String> {
            names.iter().map(|name| (*name).to_owned()).collect()
        };
        assert_eq!(required("overview"), set(&[]));
        assert_eq!(required("search"), set(&["text"]));
        assert_eq!(required("describe_node"), set(&["node"]));
        assert_eq!(required("expand"), set(&["seeds", "depth", "limit"]));
        assert_eq!(required("timeline"), set(&["hops", "limit"]));
        assert_eq!(required("explain"), set(&["assertion"]));
        assert_eq!(required("resolve"), set(&["type_id", "aliases"]));
        server.close();
    }
}

// 4 --------------------------------------------------------------------------------------------

#[test]
fn no_tool_writes_and_a_call_of_a_tool_the_server_does_not_have_is_an_error() {
    let world = World::seeded("file");
    let mut server = world.server();
    let listed = server.request("tools/list", None);
    for tool in listed["result"]["tools"].as_array().unwrap() {
        let name = tool["name"].as_str().unwrap();
        for write in ["propose", "validate", "commit", "seed", "mint"] {
            assert!(!name.contains(write), "{name} writes");
        }
        assert_ne!(tool["annotations"]["destructiveHint"], true, "{tool}");
    }
    for tool in ["propose", "validate", "commit", "seed", "snapshot", ""] {
        let refused = server.call(tool, json!({}));
        assert_eq!(Server::error_code(&refused), -32602, "{tool}: {refused}");
    }
    let head_before = world.ok(&["head"]);
    let transactions_before = world.ok(&["transactions"]);
    for (tool, arguments) in [
        ("overview", json!({})),
        ("search", json!({"text": "A"})),
        ("describe_node", json!({"node": ALICE})),
        ("expand", json!({"seeds": [ALICE], "depth": 2, "limit": 10})),
        ("timeline", json!({"hops": 1, "limit": 5})),
        ("explain", json!({"assertion": SEEDED_ASSERTION})),
        (
            "resolve",
            json!({"type_id": ORGANIZATION, "aliases": ["Globex"]}),
        ),
    ] {
        server.document(tool, arguments);
    }
    server.close();
    assert_eq!(world.ok(&["head"]), head_before, "a read moved the head");
    assert_eq!(
        world.ok(&["transactions"]),
        transactions_before,
        "a read retained a transaction"
    );
}

// 2 --------------------------------------------------------------------------------------------

#[test]
fn each_tool_answers_the_document_its_read_returns_on_the_same_store_state() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let mut server = world.server();
        let head = world.index(None);
        let alice = node(ALICE);
        let person: TypeId = PERSON.parse().unwrap();

        let cases: Vec<(&str, Value, Vec<u8>)> = vec![
            (
                "overview",
                json!({}),
                head.overview(&OverviewRequest::new(None).unwrap())
                    .unwrap()
                    .bytes,
            ),
            (
                "overview",
                json!({"limit": 2, "revision": 0}),
                world
                    .index(Some(0))
                    .overview(&OverviewRequest::new(Some(2)).unwrap())
                    .unwrap()
                    .bytes,
            ),
            (
                "search",
                json!({"text": "Ac"}),
                head.search(&SearchRequest::new("Ac".to_owned(), 20).unwrap())
                    .unwrap()
                    .bytes,
            ),
            (
                "search",
                json!({"text": "", "limit": 2, "revision": 0}),
                head.search(&SearchRequest::new(String::new(), 2).unwrap())
                    .unwrap()
                    .bytes,
            ),
            (
                "describe_node",
                json!({"node": ALICE}),
                head.describe(alice).unwrap().bytes,
            ),
            (
                "describe_node",
                json!({"node": ACME, "revision": 0}),
                head.describe(node(ACME)).unwrap().bytes,
            ),
            (
                "expand",
                json!({"seeds": [ALICE], "depth": 1, "limit": 10}),
                head.expand(&ExpandRequest::new(vec![alice], 1, 10, None, None).unwrap())
                    .unwrap()
                    .bytes,
            ),
            (
                "expand",
                json!({"seeds": [ALICE, ACME], "depth": 2, "limit": 1, "edges": 1, "after": 1}),
                head.expand(
                    &ExpandRequest::new(vec![alice, node(ACME)], 2, 1, Some(1), Some(1)).unwrap(),
                )
                .unwrap()
                .bytes,
            ),
            (
                "expand",
                json!({"seeds": [], "depth": 0, "limit": 1}),
                head.expand(&ExpandRequest::new(Vec::new(), 0, 1, None, None).unwrap())
                    .unwrap()
                    .bytes,
            ),
            (
                "timeline",
                json!({"hops": 2, "limit": 10}),
                head.timeline(&TimelineRequest::new(None, 2, 10, None, None).unwrap())
                    .unwrap()
                    .bytes,
            ),
            (
                "timeline",
                json!({"type": PERSON, "hops": 1, "limit": 3, "bucket": "day", "subject": ALICE}),
                head.timeline(
                    &TimelineRequest::new(Some(person), 1, 3, Some(BucketWidth::Day), Some(alice))
                        .unwrap(),
                )
                .unwrap()
                .bytes,
            ),
            (
                "timeline",
                json!({"hops": 3, "limit": 500, "bucket": "week", "revision": 0}),
                head.timeline(
                    &TimelineRequest::new(None, 3, 500, Some(BucketWidth::Week), None).unwrap(),
                )
                .unwrap()
                .bytes,
            ),
        ];
        for (tool, arguments, expected) in cases {
            let answered = server.document(tool, arguments.clone());
            assert_eq!(
                answered,
                utf8(expected),
                "{backend}: {tool} {arguments} is not the ekr.views document"
            );
        }

        // explain and resolve: what the one-shot verb prints, byte for byte.
        let explained = server.document("explain", json!({"assertion": SEEDED_ASSERTION}));
        assert_eq!(
            explained,
            world.ok(&["explain", SEEDED_ASSERTION]),
            "{backend}: explain"
        );
        for (reference, at) in [
            (json!({"type_id": ORGANIZATION, "aliases": ["Acme"]}), None),
            (
                json!({"type_id": PERSON, "aliases": ["Alice", "Al"]}),
                Some(0),
            ),
        ] {
            world.file("reference.json", &reference.to_string());
            let mut arguments = reference.clone();
            let mut verb = vec!["resolve", "reference.json"];
            if let Some(at) = at {
                arguments["at"] = json!(at);
                verb.extend(["--at", "0"]);
            }
            let resolved = server.document("resolve", arguments);
            assert_eq!(resolved, world.ok(&verb), "{backend}: resolve {reference}");
        }
        server.close();
    }
}

// 3 --------------------------------------------------------------------------------------------

#[test]
fn a_refusal_is_a_tool_error_with_its_name_and_the_server_keeps_serving() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let mut server = world.server();
        let head = world.index(None);
        let beyond = ProjectError::RevisionNotFound {
            requested: RevisionNumber::new(7),
            head: RevisionNumber::new(0),
        }
        .to_string();
        let unknown_node = head.describe(node(UNKNOWN)).unwrap_err().to_string();
        let limit = |error: ekr_views::LimitExceeded| error.to_string();

        let cases: Vec<(&str, Value, &str, Option<String>)> = vec![
            (
                "describe_node",
                json!({"node": UNKNOWN}),
                "ekr.views.NodeNotFound",
                Some(unknown_node.clone()),
            ),
            (
                "describe_node",
                json!({"node": "not-a-node-id"}),
                "ekr.views.NodeNotFound",
                None,
            ),
            (
                "describe_node",
                json!({"node": ALICE, "revision": 7}),
                "ekr.views.RevisionNotFound",
                Some(beyond.clone()),
            ),
            (
                "overview",
                json!({"revision": 7}),
                "ekr.views.RevisionNotFound",
                Some(beyond.clone()),
            ),
            (
                "overview",
                json!({"limit": 501}),
                "ekr.views.LimitExceeded",
                Some(limit(OverviewRequest::new(Some(501)).unwrap_err())),
            ),
            // A bound is refused before the store is read, so before the revision.
            (
                "overview",
                json!({"limit": 0, "revision": 7}),
                "ekr.views.LimitExceeded",
                Some(limit(OverviewRequest::new(Some(0)).unwrap_err())),
            ),
            (
                "search",
                json!({"text": "A", "limit": 0}),
                "ekr.views.LimitExceeded",
                Some(limit(SearchRequest::new("A".to_owned(), 0).unwrap_err())),
            ),
            (
                "search",
                json!({"text": "A", "limit": 101}),
                "ekr.views.LimitExceeded",
                Some(limit(SearchRequest::new("A".to_owned(), 101).unwrap_err())),
            ),
            (
                "search",
                json!({"text": "A", "revision": 7}),
                "ekr.views.RevisionNotFound",
                Some(beyond.clone()),
            ),
            (
                "expand",
                json!({"seeds": [ALICE], "depth": 3, "limit": 1}),
                "ekr.views.LimitExceeded",
                Some(limit(
                    ExpandRequest::new(vec![], 3, 1, None, None).unwrap_err(),
                )),
            ),
            (
                "expand",
                json!({"seeds": [ALICE], "depth": 1, "limit": 1, "after": -1}),
                "ekr.views.LimitExceeded",
                Some(limit(
                    ExpandRequest::new(vec![], 1, 1, None, Some(-1)).unwrap_err(),
                )),
            ),
            (
                "expand",
                json!({"seeds": [ALICE], "depth": 1, "limit": 1, "edges": 5001}),
                "ekr.views.LimitExceeded",
                Some(limit(
                    ExpandRequest::new(vec![], 1, 1, Some(5001), None).unwrap_err(),
                )),
            ),
            (
                "expand",
                json!({"seeds": [ALICE, UNKNOWN], "depth": 1, "limit": 1}),
                "ekr.views.NodeNotFound",
                Some(unknown_node),
            ),
            (
                "timeline",
                json!({"hops": 4, "limit": 1}),
                "ekr.views.LimitExceeded",
                Some(limit(
                    TimelineRequest::new(None, 4, 1, None, None).unwrap_err(),
                )),
            ),
            (
                "timeline",
                json!({"hops": 1, "limit": 1, "revision": 7}),
                "ekr.views.RevisionNotFound",
                Some(beyond.clone()),
            ),
        ];
        for (tool, arguments, name, message) in cases {
            let refusal = server.refusal(tool, arguments.clone());
            assert_eq!(refusal["refusal"], name, "{backend}: {tool} {arguments}");
            if let Some(message) = message {
                assert_eq!(refusal["message"], message, "{backend}: {tool} {arguments}");
            }
        }

        // The kernel's refusals: the name and message the one-shot verb writes to stderr.
        world.file(
            "undeclared.json",
            &json!({"type_id": UNKNOWN, "aliases": ["X"]}).to_string(),
        );
        world.file(
            "nameless.json",
            &json!({"type_id": ORGANIZATION, "aliases": [""]}).to_string(),
        );
        world.file(
            "acme.json",
            &json!({"type_id": ORGANIZATION, "aliases": ["Acme"]}).to_string(),
        );
        for (tool, arguments, verb, name) in [
            (
                "explain",
                json!({"assertion": UNKNOWN}),
                vec!["explain", UNKNOWN],
                "ekr.kernel.AssertionNotFound",
            ),
            (
                "resolve",
                json!({"type_id": UNKNOWN, "aliases": ["X"]}),
                vec!["resolve", "undeclared.json"],
                "reference-type-undeclared",
            ),
            (
                "resolve",
                json!({"type_id": ORGANIZATION, "aliases": [""]}),
                vec!["resolve", "nameless.json"],
                "reference-without-identity",
            ),
            (
                "resolve",
                json!({"type_id": ORGANIZATION, "aliases": ["Acme"], "at": 7}),
                vec!["resolve", "acme.json", "--at", "7"],
                "ekr.kernel.RevisionNotFound",
            ),
        ] {
            let refusal = server.refusal(tool, arguments.clone());
            assert_eq!(refusal["refusal"], name, "{backend}: {tool} {arguments}");
            let one_shot = world.run(&verb);
            assert_eq!(one_shot.status.code(), Some(2), "{backend}: {verb:?}");
            assert_eq!(
                String::from_utf8_lossy(&one_shot.stderr),
                format!("ekr: {name}: {}\n", refusal["message"].as_str().unwrap()),
                "{backend}: {tool} {arguments}"
            );
        }

        // Messages that are not a call the server can make: JSON-RPC errors, and it goes on.
        let parse_error = server.raw("{not json");
        assert_eq!(Server::error_code(&parse_error), -32700, "{parse_error}");
        assert_eq!(parse_error["id"], Value::Null);
        let batch = server.raw(&json!([{"jsonrpc": "2.0", "id": 1, "method": "ping"}]).to_string());
        assert_eq!(Server::error_code(&batch), -32600, "{batch}");
        let versionless = server.raw(&json!({"id": 5, "method": "ping"}).to_string());
        assert_eq!(Server::error_code(&versionless), -32600, "{versionless}");
        assert_eq!(versionless["id"], 5);
        let unknown_method = server.request("resources/list", None);
        assert_eq!(Server::error_code(&unknown_method), -32601);
        let missing_version = server.request("initialize", Some(json!({})));
        assert_eq!(Server::error_code(&missing_version), -32602);
        for (tool, arguments) in [
            ("search", json!({})),
            ("search", json!({"text": 3})),
            ("search", json!({"text": "A", "limit": "3"})),
            ("search", json!({"text": "A", "limit": 2.5})),
            ("search", json!({"text": "A", "revision": -1})),
            ("search", json!({"text": "A", "q": "A"})),
            ("overview", json!([])),
            ("describe_node", json!({})),
            (
                "expand",
                json!({"seeds": ["not-a-node-id"], "depth": 1, "limit": 1}),
            ),
            ("expand", json!({"seeds": ALICE, "depth": 1, "limit": 1})),
            ("expand", json!({"seeds": [ALICE], "limit": 1})),
            (
                "timeline",
                json!({"hops": 1, "limit": 1, "bucket": "month"}),
            ),
            ("timeline", json!({"hops": 1, "limit": 1, "type": "person"})),
            (
                "timeline",
                json!({"hops": 1, "limit": 1, "subject": "alice"}),
            ),
            ("explain", json!({"assertion": "not-an-assertion-id"})),
            ("resolve", json!({"aliases": ["Acme"]})),
            (
                "resolve",
                json!({"type_id": ORGANIZATION, "aliases": "Acme"}),
            ),
            (
                "resolve",
                json!({"type_id": ORGANIZATION, "aliases": ["Acme"], "name": "x"}),
            ),
            (
                "resolve",
                json!({"type_id": ORGANIZATION, "aliases": ["Acme"], "at": -1}),
            ),
        ] {
            let refused = server.call(tool, arguments.clone());
            assert_eq!(
                Server::error_code(&refused),
                -32602,
                "{backend}: {tool} {arguments}: {refused}"
            );
        }
        let without_name = server.request("tools/call", Some(json!({"arguments": {}})));
        assert_eq!(Server::error_code(&without_name), -32602);

        // A notification is never answered, whatever its method: the next answer is the ping's.
        server.notify("notifications/cancelled");
        server.notify("no/such/notification");
        let pinged = server.request("ping", None);
        assert_eq!(pinged["result"], json!({}));

        // And the server still reads.
        let alice = server.document("describe_node", json!({"node": ALICE}));
        assert_eq!(alice, utf8(head.describe(node(ALICE)).unwrap().bytes));
        server.close();
    }
}

// 5 --------------------------------------------------------------------------------------------

#[test]
fn a_commit_by_another_process_is_read_by_the_next_tool_call() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let mut server = world.server();
        world.file(
            "globex.json",
            &json!({"type_id": ORGANIZATION, "aliases": ["Globex"]}).to_string(),
        );
        let globex = json!({"type_id": ORGANIZATION, "aliases": ["Globex"]});

        let before = server.refusal("describe_node", json!({"node": GLOBEX}));
        assert_eq!(before["refusal"], "ekr.views.NodeNotFound");
        let unresolved: Value =
            serde_json::from_str(&server.document("resolve", globex.clone())).unwrap();
        assert_eq!(unresolved["kind"], "ProposeNew", "{unresolved}");
        let overview: Value =
            serde_json::from_str(&server.document("overview", json!({}))).unwrap();
        assert_eq!(overview["meta"]["revision"], 0, "{overview}");

        world.commit("create.yaml");

        let head = world.index(None);
        assert_eq!(head.revision(), RevisionNumber::new(1));
        let described = server.document("describe_node", json!({"node": GLOBEX}));
        assert_eq!(
            described,
            utf8(head.describe(node(GLOBEX)).unwrap().bytes),
            "{backend}: the node another process committed"
        );
        let resolved = server.document("resolve", globex);
        assert_eq!(resolved, world.ok(&["resolve", "globex.json"]));
        let resolved: Value = serde_json::from_str(&resolved).unwrap();
        assert_eq!(resolved["kind"], "Resolved", "{resolved}");
        assert_eq!(resolved["node_id"], GLOBEX);
        let overview = server.document("overview", json!({}));
        assert_eq!(
            overview,
            utf8(
                head.overview(&OverviewRequest::new(None).unwrap())
                    .unwrap()
                    .bytes
            )
        );
        // The revision before is still readable, loaded under the new head.
        let at_seed = server.document("overview", json!({"revision": 0}));
        assert_eq!(
            at_seed,
            utf8(
                world
                    .index(Some(0))
                    .overview(&OverviewRequest::new(None).unwrap())
                    .unwrap()
                    .bytes
            )
        );
        server.close();
    }
}

// 6 --------------------------------------------------------------------------------------------

/// 100 `search` and 100 `describe_node` calls over one server, on a store holding the example
/// seed and 200 more nodes; the time of each kind of call is printed (`--nocapture`).
#[test]
fn a_hundred_searches_and_node_descriptions_over_one_server_are_timed() {
    const NODES: usize = 200;
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let nodes: Vec<(String, String)> = (0..NODES)
            .map(|at| {
                (
                    format!("00000000-0000-4000-8000-{:012x}", 0x10000 + at),
                    format!("Member{at:03}"),
                )
            })
            .collect();
        world.file("many.yaml", &create(MANY, PERSON, &nodes));
        world.commit("many.yaml");
        let head = world.index(None);

        let started = Instant::now();
        let mut server = world.server();
        let first = Instant::now();
        server.document("search", json!({"text": "Member"}));
        let first = first.elapsed();
        let opened = started.elapsed();

        let searches = Instant::now();
        for at in 0..100 {
            let text = format!("Member{:03}", at * 2);
            let answered = server.document("search", json!({"text": text}));
            if at % 25 == 0 {
                let expected = head
                    .search(&SearchRequest::new(text, 20).unwrap())
                    .unwrap()
                    .bytes;
                assert_eq!(answered, utf8(expected));
            }
        }
        let searches = searches.elapsed();

        let describes = Instant::now();
        for (at, (id, _)) in nodes.iter().take(100).enumerate() {
            let answered = server.document("describe_node", json!({"node": id}));
            if at % 25 == 0 {
                assert_eq!(answered, utf8(head.describe(node(id)).unwrap().bytes));
            }
        }
        let describes = describes.elapsed();
        server.close();

        eprintln!(
            "ekr mcp timing, {backend}, {} nodes: start and first search {:.1} ms (first search \
             {:.1} ms); search {:.3} ms per call over 100; describe_node {:.3} ms per call over 100",
            head.loaded().graph.nodes.len(),
            opened.as_secs_f64() * 1e3,
            first.as_secs_f64() * 1e3,
            searches.as_secs_f64() * 1e3 / 100.0,
            describes.as_secs_f64() * 1e3 / 100.0,
        );
    }
}

// the verb -------------------------------------------------------------------------------------

#[test]
fn a_server_that_cannot_open_its_store_answers_nothing_and_exits_as_the_verbs_do() {
    let world = World::seeded("file");
    let unconfigured = ekr().arg("mcp").stdin(Stdio::null()).output().unwrap();
    assert_eq!(unconfigured.status.code(), Some(2));
    assert!(unconfigured.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&unconfigured.stderr)
            .starts_with("ekr: `mcp` needs --host or EKR_HOST"),
        "{}",
        String::from_utf8_lossy(&unconfigured.stderr)
    );
    let missing = ekr()
        .arg("--host")
        .arg(world.directory.path().join("host.json"))
        .arg("--store")
        .arg(world.directory.path().join("nothing-here"))
        .args(["--backend", "file", "mcp"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert!(missing.stdout.is_empty());
    assert!(
        String::from_utf8_lossy(&missing.stderr).starts_with("ekr: store-not-found: "),
        "{}",
        String::from_utf8_lossy(&missing.stderr)
    );
    assert!(!world.directory.path().join("nothing-here").exists());
}

/// The in-process seam: `ekr::cli::serve_mcp` answers the messages of any reader into any writer
/// as the binary does, and `ekr::cli::run` of `mcp` returns the same answers.
#[test]
fn the_library_seam_serves_mcp_in_process() {
    let world = World::seeded("sqlite");
    let argv = || -> Vec<std::ffi::OsString> {
        vec![
            "ekr".into(),
            "--host".into(),
            world.directory.path().join("host.json").into(),
            "--store".into(),
            world.store().into(),
            "--backend".into(),
            "sqlite".into(),
            "mcp".into(),
        ]
    };
    let lines = [
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": "2025-11-25", "capabilities": {}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": "two", "method": "tools/call",
               "params": {"name": "describe_node", "arguments": {"node": ALICE}}}),
    ]
    .iter()
    .map(|message| format!("{message}\n"))
    .collect::<String>();
    let cli = <ekr::cli::Cli as clap::Parser>::try_parse_from(argv()).unwrap();
    let mut output = Vec::new();
    ekr::cli::serve_mcp(cli, &mut lines.as_bytes(), &mut output).unwrap();
    let served = String::from_utf8(output).unwrap();
    let clock = || ekr_core::Timestamp::from_millis(0);
    let returned = ekr::cli::run(argv(), &clock, &mut lines.as_bytes()).unwrap();
    assert_eq!(served, returned);
    let answers: Vec<Value> = served
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(answers.len(), 2, "{served}");
    assert_eq!(answers[1]["id"], "two");
    assert_eq!(
        document_text(&answers[1]["result"]),
        utf8(world.index(None).describe(node(ALICE)).unwrap().bytes)
    );
}

/// docs/cli.md § `ekr mcp` lists exactly the tools `tools/list` names, one row each.
#[test]
fn the_page_lists_exactly_the_tools_the_server_has() {
    let root = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    );
    let page = std::fs::read_to_string(root.join("../../docs/cli.md")).unwrap();
    let start = page
        .find("\n### `ekr mcp`\n")
        .expect("docs/cli.md has an `ekr mcp` section");
    let rest = &page[start + 1..];
    let end = rest[4..]
        .find("\n## ")
        .map_or(rest.len(), |at| at + 4)
        .min(rest[4..].find("\n### ").map_or(rest.len(), |at| at + 4));
    let section = &rest[..end];
    let documented: BTreeSet<&str> = section
        .lines()
        .filter_map(|line| line.strip_prefix("| `"))
        .filter_map(|line| line.split_once('`').map(|(name, _)| name))
        .filter(|name| name.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        .collect();
    assert_eq!(documented, TOOLS.into_iter().collect());
}
