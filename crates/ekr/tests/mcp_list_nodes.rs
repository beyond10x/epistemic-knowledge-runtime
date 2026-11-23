//! `story:mcp-list-nodes-by-type`: `ekr mcp` lists every node of a type page by page, and a
//! search that matched more than one page can be continued.
//!
//! The consumer (a runbook refresh reading a store through `ekr mcp`) had no tool that lists the
//! nodes of a type, and `search` stopped at 100 results; finding 29 of the external review of
//! EKR 0.0.42: for 101 equal names, search reported 101, returned 100 and had no continuation.
//!
//! One store per provider, seeded from the example seed with one more node type, `Plot`, and one
//! committed transaction: 250 `Plot` nodes and 101 `Organization` nodes all named
//! `Shared garden name`. Against it:
//!
//! * `list_nodes` paged by each page's `meta.next` answers each of the 250 `Plot` nodes exactly
//!   once, `meta.total` 250 on every page, and is byte for byte `search`'s answer for the empty
//!   text with that `type`;
//! * `search` for the shared name answers 100 matches and `meta.next` 100, and the call with
//!   `after` 100 answers the 101st and no `next`; the session verb `search --after` answers the
//!   same bytes;
//! * `after` under `ekr.node-matches/1` or /2, a negative `after`, a `list_nodes` without `type`
//!   and one naming no node type are refused.

use std::collections::BTreeSet;
use std::io::{BufRead, BufReader, Write as _};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
/// The node type this file adds to the example seed; the seed holds no node of it.
const PLOT: &str = "00000000-0000-4000-8000-000000000204";
const TRANSACTION: &str = "00000000-0000-4000-8000-00000000a000";
const PLOTS: usize = 250;
const SHARED: usize = 101;
const SHARED_NAME: &str = "Shared garden name";

/// How long one answer may take before the case fails instead of hanging.
const ANSWER_WITHIN: Duration = Duration::from_secs(60);

/// The id of the `ordinal`th node of `block` (`b` for the plots, `c` for the shared names).
fn id(block: char, ordinal: usize) -> String {
    format!("00000000-0000-4000-8000-00000{block}{ordinal:06}")
}

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

/// One provider in its own directory: the example seed with `Plot` declared, then the 250 plots
/// and the 101 shared names committed in one transaction.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn new(backend: &'static str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        world.file("host.json", &world.example("ekr.cli-host/1"));
        let seed = world.example("ekr-seed/2");
        let anchor = "  node_types:\n";
        assert!(seed.contains(anchor), "the example seed lists node_types");
        let plot = format!(
            "{anchor}  - id: {PLOT}\n    name: Plot\n    parents: []\n    properties: {{}}\n    \
             abstract_type: false\n    lifecycle: null\n    operations: {{}}\n"
        );
        world.file("seed.yaml", &seed.replacen(anchor, &plot, 1));
        world.ok(&["seed", "seed.yaml"]);
        let mut document = format!(
            "format: ekr.transaction-document/2\ntransaction:\n  id: {TRANSACTION}\n  proposer: \
             {OPERATOR}\n  operations:\n"
        );
        let nodes = (0..PLOTS)
            .map(|ordinal| (id('b', ordinal), PLOT, format!("Plot {ordinal:03}")))
            .chain((0..SHARED).map(|ordinal| (id('c', ordinal), ORGANIZATION, SHARED_NAME.into())));
        for (id, type_id, name) in nodes {
            document.push_str(&format!(
                "  - !CreateNode\n    id: {id}\n    root_id: {ROOT}\n    type_id: {type_id}\n    \
                 canonical_name: {name}\n    properties: {{}}\n    aliases: []\n"
            ));
        }
        document.push_str("  evidence: []\n");
        world.file("create.yaml", &document);
        let proposed: Value = serde_json::from_str(&world.ok(&["propose", "create.yaml"])).unwrap();
        let transaction = proposed["transaction_id"].as_str().unwrap().to_owned();
        let validated: Value =
            serde_json::from_str(&world.ok(&["validate", &transaction])).unwrap();
        assert_eq!(validated["kind"], "Validated", "{validated}");
        let committed: Value = serde_json::from_str(&world.ok(&["commit", &transaction])).unwrap();
        assert_eq!(committed["kind"], "Committed", "{committed}");
        world
    }

    fn example(&self, document: &str) -> String {
        let output = ekr().args(["example", document]).output().unwrap();
        assert_eq!(output.status.code(), Some(0), "example {document}");
        String::from_utf8(output.stdout).unwrap()
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

    /// A one-shot verb that must exit 0: its stdout.
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

    /// What the session answers `argv` with, run as one request: exit 0 and its `stdout`, the
    /// document as JSON.
    fn session(&self, argv: &[&str]) -> Value {
        let mut session = self
            .command(&["session"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        writeln!(session.stdin.take().unwrap(), "{}", json!({ "argv": argv })).unwrap();
        let output = session.wait_with_output().unwrap();
        let answer: Value =
            serde_json::from_str(String::from_utf8(output.stdout).unwrap().trim()).unwrap();
        assert_eq!(answer["exit"], 0, "{argv:?}: {answer}");
        answer["stdout"].clone()
    }

    /// `ekr mcp` started and initialized.
    fn server(&self) -> Server {
        let mut server = Server::start(self.command(&["mcp"]));
        server.request(
            "initialize",
            Some(json!({
                "protocolVersion": "2025-11-25",
                "capabilities": {},
                "clientInfo": {"name": "mcp-list-nodes-test", "version": "0"}
            })),
        );
        server.send_line(
            &json!({"jsonrpc": "2.0", "method": "notifications/initialized"}).to_string(),
        );
        server
    }
}

/// A running `ekr mcp`: one request a line, each answer read before the next.
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

    fn send_line(&mut self, line: &str) {
        self.input.write_all(line.as_bytes()).unwrap();
        self.input.write_all(b"\n").unwrap();
        self.input.flush().unwrap();
    }

    fn request(&mut self, method: &str, params: Option<Value>) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let mut message = json!({"jsonrpc": "2.0", "id": id, "method": method});
        if let Some(params) = params {
            message["params"] = params;
        }
        self.send_line(&message.to_string());
        let line = self
            .lines
            .recv_timeout(ANSWER_WITHIN)
            .expect("the server answers within the minute");
        let response: Value = serde_json::from_str(&line).unwrap();
        assert_eq!(response["id"], id, "{response}");
        response
    }

    fn call(&mut self, tool: &str, arguments: Value) -> Value {
        self.request(
            "tools/call",
            Some(json!({"name": tool, "arguments": arguments})),
        )
    }

    /// A call that answered a document: its text, which its `structuredContent` equals.
    fn document(&mut self, tool: &str, arguments: Value) -> String {
        let response = self.call(tool, arguments.clone());
        let result = &response["result"];
        assert_eq!(result["isError"], false, "{tool} {arguments}: {response}");
        let text = result["content"][0]["text"].as_str().unwrap().to_owned();
        assert_eq!(
            result["structuredContent"],
            serde_json::from_str::<Value>(&text).unwrap(),
            "{response}"
        );
        text
    }

    /// A call refused by name: the refusal's name and message.
    fn refusal(&mut self, tool: &str, arguments: Value) -> Value {
        let response = self.call(tool, arguments.clone());
        let result = &response["result"];
        assert_eq!(result["isError"], true, "{tool} {arguments}: {response}");
        result["structuredContent"].clone()
    }

    /// A call answered with a JSON-RPC error: its code and message.
    fn error(&mut self, tool: &str, arguments: Value) -> (i64, String) {
        let response = self.call(tool, arguments.clone());
        assert!(
            response.get("result").is_none(),
            "{tool} {arguments}: {response}"
        );
        (
            response["error"]["code"].as_i64().unwrap(),
            response["error"]["message"].as_str().unwrap().to_owned(),
        )
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
        assert!(stderr.is_empty(), "{stderr}");
    }
}

/// The ids of a node-matches document's matches, in order.
fn ids(document: &Value) -> Vec<String> {
    document["matches"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hit| hit["id"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn list_nodes_pages_every_node_of_a_type_once_with_its_total() {
    let started = Instant::now();
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut server = world.server();

        // The tool is listed, requires its type and takes the paging arguments.
        let listed = server.request("tools/list", None);
        let tool = listed["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .find(|tool| tool["name"] == "list_nodes")
            .unwrap_or_else(|| panic!("{backend}: tools/list names no list_nodes: {listed}"))
            .clone();
        assert_eq!(tool["inputSchema"]["required"], json!(["type"]), "{tool}");
        for argument in ["type", "limit", "after", "revision"] {
            assert!(
                tool["inputSchema"]["properties"][argument].is_object(),
                "{backend}: list_nodes takes no {argument}: {tool}"
            );
        }
        assert_eq!(tool["annotations"]["readOnlyHint"], true, "{tool}");

        // Paged by each page's next, by type id: three pages, every plot once.
        let mut seen: Vec<String> = Vec::new();
        let mut pages = 0;
        let mut after: Option<u64> = None;
        let mut revision: Option<u64> = None;
        loop {
            let mut arguments = json!({"type": PLOT});
            if let Some(after) = after {
                arguments["after"] = json!(after);
                arguments["revision"] = json!(revision.unwrap());
            }
            let text = server.document("list_nodes", arguments.clone());
            let page: Value = serde_json::from_str(&text).unwrap();
            let meta = &page["meta"];
            assert_eq!(meta["format"], "ekr.node-matches/3", "{backend}: {meta}");
            assert_eq!(meta["total"], PLOTS, "{backend}: {meta}");
            assert_eq!(meta["types"], json!([PLOT]), "{backend}: {meta}");
            revision.get_or_insert(meta["revision"].as_u64().unwrap());
            assert_eq!(meta["revision"].as_u64(), revision, "{backend}: {meta}");
            // Byte for byte search's answer for the empty text with that type.
            let mut searched = arguments.clone();
            searched["text"] = json!("");
            searched["limit"] = json!(100);
            assert_eq!(
                server.document("search", searched),
                text,
                "{backend}: list_nodes {arguments} is not search for the empty text"
            );
            let page_ids = ids(&page);
            pages += 1;
            seen.extend(page_ids.iter().cloned());
            match meta.get("next").and_then(Value::as_u64) {
                Some(next) => {
                    assert_eq!(
                        page_ids.len(),
                        100,
                        "{backend}: a page before the last is full"
                    );
                    assert_eq!(next as usize, seen.len(), "{backend}: {meta}");
                    after = Some(next);
                }
                None => break,
            }
            assert!(
                pages <= 3,
                "{backend}: more than three pages of 100 for 250 nodes"
            );
        }
        assert_eq!(pages, 3, "{backend}");
        assert_eq!(seen.len(), PLOTS, "{backend}: {} listed", seen.len());
        let unique: BTreeSet<&String> = seen.iter().collect();
        assert_eq!(unique.len(), PLOTS, "{backend}: a node was listed twice");
        let created: BTreeSet<String> = (0..PLOTS).map(|ordinal| id('b', ordinal)).collect();
        assert_eq!(
            unique.into_iter().cloned().collect::<BTreeSet<_>>(),
            created,
            "{backend}"
        );

        // By type name, with a smaller page: the same first nodes in the same order.
        let named: Value = serde_json::from_str(
            &server.document("list_nodes", json!({"type": "Plot", "limit": 7})),
        )
        .unwrap();
        assert_eq!(ids(&named), seen[..7].to_vec(), "{backend}");
        assert_eq!(named["meta"]["next"], 7, "{backend}");

        // A cursor at the end answers an empty page with no next.
        let end: Value = serde_json::from_str(
            &server.document("list_nodes", json!({"type": PLOT, "after": PLOTS})),
        )
        .unwrap();
        assert!(ids(&end).is_empty(), "{backend}: {end}");
        assert!(end["meta"].get("next").is_none(), "{backend}: {end}");
        server.close();
    }
    println!("list_nodes over 2 providers: {:?}", started.elapsed());
}

#[test]
fn a_search_past_one_hundred_equal_names_continues_with_its_token() {
    let started = Instant::now();
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut server = world.server();
        let first_text = server.document("search", json!({"text": SHARED_NAME, "limit": 100}));
        let first: Value = serde_json::from_str(&first_text).unwrap();
        let meta = &first["meta"];
        assert_eq!(meta["total"], SHARED, "{backend}: {meta}");
        assert_eq!(meta["whole_total"], SHARED, "{backend}: {meta}");
        assert_eq!(ids(&first).len(), 100, "{backend}");
        assert_eq!(
            meta["next"], 100,
            "{backend}: the first page names no continuation: {meta}"
        );
        let revision = meta["revision"].clone();

        let rest_text = server.document(
            "search",
            json!({"text": SHARED_NAME, "limit": 100, "after": 100, "revision": revision}),
        );
        let rest: Value = serde_json::from_str(&rest_text).unwrap();
        assert_eq!(ids(&rest).len(), 1, "{backend}: {rest}");
        assert_eq!(rest["meta"]["total"], SHARED, "{backend}");
        assert!(rest["meta"].get("next").is_none(), "{backend}: {rest}");

        let mut every: BTreeSet<String> = ids(&first).into_iter().collect();
        every.extend(ids(&rest));
        let created: BTreeSet<String> = (0..SHARED).map(|ordinal| id('c', ordinal)).collect();
        assert_eq!(
            every, created,
            "{backend}: the two pages are not the 101 matches"
        );

        // The session verb answers the same pages.
        let revision = revision.to_string();
        for (after, page) in [("0", &first), ("100", &rest)] {
            assert_eq!(
                &world.session(&[
                    "search",
                    SHARED_NAME,
                    "--format",
                    "ekr.node-matches/3",
                    "--limit",
                    "100",
                    "--after",
                    after,
                    "--revision",
                    &revision,
                ]),
                page,
                "{backend}: session search --after {after}"
            );
        }
        server.close();
    }
    println!(
        "search continuation over 2 providers: {:?}",
        started.elapsed()
    );
}

#[test]
fn after_outside_node_matches_3_and_a_list_without_a_type_are_refused() {
    let world = World::new("file");
    let mut server = world.server();
    for format in ["ekr.node-matches/1", "ekr.node-matches/2"] {
        let (code, message) = server.error(
            "search",
            json!({"text": SHARED_NAME, "format": format, "after": 1}),
        );
        assert_eq!(code, -32602, "{format}: {message}");
        assert!(message.contains("after"), "{format}: {message}");
    }
    for (tool, arguments) in [
        ("search", json!({"text": SHARED_NAME, "after": -1})),
        ("list_nodes", json!({"type": PLOT, "after": -1})),
        ("list_nodes", json!({"type": PLOT, "limit": 101})),
    ] {
        let refusal = server.refusal(tool, arguments.clone());
        assert_eq!(
            refusal["refusal"], "ekr.views.LimitExceeded",
            "{tool} {arguments}: {refusal}"
        );
    }
    let refusal = server.refusal("list_nodes", json!({"type": "Nothing"}));
    assert_eq!(
        refusal["refusal"], "ekr.views.NodeTypeNotFound",
        "{refusal}"
    );
    for arguments in [json!({}), json!({"type": PLOT, "text": ""})] {
        let (code, message) = server.error("list_nodes", arguments.clone());
        assert_eq!(code, -32602, "{arguments}: {message}");
    }
    server.close();
}
