//! Adversary pass on `story:explain-reads-an-index` (wave extract-06, unit X): `ekr explain
//! --documents` returns the very records the unversioned answer embedded, and the one-shot verb,
//! the session and MCP agree on a chain that runs through a supersession.
//!
//! The unversioned answer embedded the proposal record as the Proposal link, and the commit
//! receipt as the Commit link and as a Lifecycle link's `receipt`. Those records are exactly what
//! `ekr propose` and `ekr commit` printed when they were written, so `--documents` is held to
//! those outputs, field for field.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{json, Value};

const ALICE: &str = "00000000-0000-4000-8000-000000000501";
const T_ALICE: &str = "00000000-0000-4000-8000-000000000601";
const T_BOB: &str = "00000000-0000-4000-8000-000000000602";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/retraction")
        .join(name)
}

struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}
impl World {
    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().to_path_buf(),
            _ => self.directory.path().join("state.db"),
        }
    }
    fn run(&self, verb: &[&str], stdin: &[u8]) -> Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
        for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
            command.env_remove(var);
        }
        let mut child = command
            .arg("--host")
            .arg(fixture("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(verb)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(stdin).unwrap();
        child.wait_with_output().unwrap()
    }
    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb, b"");
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

/// The links of `explained` whose kind is `kind`, in chain order.
fn links<'a>(explained: &'a Value, kind: &str) -> Vec<&'a Value> {
    explained["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|link| link["kind"] == kind)
        .collect()
}

/// What `ekr commit` printed, as the receipt it holds: the receipt with the result's
/// `"kind": "Committed"` tag beside its fields.
fn receipt_of(committed: &Value) -> Value {
    let mut receipt = committed.clone();
    let fields = receipt.as_object_mut().unwrap();
    assert_eq!(
        fields.remove("kind"),
        Some(Value::String("Committed".into())),
        "commit printed {committed}"
    );
    receipt
}

#[test]
fn documents_returns_the_records_propose_and_commit_printed_on_every_lane() {
    for backend in ["file", "sqlite"] {
        let world = World {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        world.ok(&["seed", fixture("seed.yaml").to_str().unwrap()]);
        let proposed_alice =
            world.ok(&["propose", fixture("propose-alice.yaml").to_str().unwrap()]);
        world.ok(&["validate", T_ALICE, "--against", "0"]);
        let committed_alice = world.ok(&["commit", T_ALICE]);
        let proposed_bob = world.ok(&["propose", fixture("propose-bob.yaml").to_str().unwrap()]);
        world.ok(&["validate", T_BOB, "--against", "1"]);
        let committed_bob = world.ok(&["commit", T_BOB]);

        let whole = world.ok(&["explain", ALICE, "--documents"]);
        let proposals = links(&whole, "Proposal");
        assert_eq!(proposals.len(), 2, "{backend}: {whole}");
        assert_eq!(
            proposals[0]["record"], proposed_alice,
            "{backend}: Alice's proposal"
        );
        assert_eq!(
            proposals[1]["record"], proposed_bob,
            "{backend}: Bob's proposal"
        );
        let commits = links(&whole, "Commit");
        assert_eq!(commits.len(), 2, "{backend}: {whole}");
        assert_eq!(
            &commits[0]["receipt"],
            &receipt_of(&committed_alice),
            "{backend}: Alice's receipt"
        );
        assert_eq!(
            &commits[1]["receipt"],
            &receipt_of(&committed_bob),
            "{backend}: Bob's receipt"
        );
        let lifecycle = links(&whole, "Lifecycle");
        assert_eq!(lifecycle.len(), 1, "{backend}: {whole}");
        assert_eq!(
            &lifecycle[0]["commit"]["receipt"],
            &receipt_of(&committed_bob),
            "{backend}: the supersession's receipt"
        );

        // The session answers the same document for the same argv.
        let request = json!({"argv": ["explain", ALICE, "--documents"]}).to_string() + "\n";
        let session = world.run(&["session"], request.as_bytes());
        let line = String::from_utf8(session.stdout).unwrap();
        let answer: Value = serde_json::from_str(line.lines().last().unwrap()).unwrap();
        assert_eq!(answer["exit"], 0, "{backend} session: {answer}");
        assert_eq!(answer["stdout"], whole, "{backend} session");

        // And MCP, with `documents: true`.
        let requests = [
            json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
                "protocolVersion": "2025-11-25", "capabilities": {},
                "clientInfo": {"name": "adversary-x6-x", "version": "0"}}}),
            json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
            json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
                "name": "explain", "arguments": {"assertion": ALICE, "documents": true}}}),
        ]
        .iter()
        .map(|r| r.to_string() + "\n")
        .collect::<String>();
        let mcp = world.run(&["mcp"], requests.as_bytes());
        let lines = String::from_utf8(mcp.stdout).unwrap();
        let called: Value = lines
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .find(|answer| answer["id"] == 1)
            .unwrap_or_else(|| panic!("{backend}: no tools/call answer in {lines}"));
        assert_eq!(called["result"]["isError"], false, "{backend}: {called}");
        assert_eq!(
            called["result"]["structuredContent"], whole,
            "{backend} mcp"
        );
    }
}
