//! Adversary, wave read-01 unit M, pass 1: `ekr mcp`'s `resolve` tool must answer what
//! `ekr resolve` prints for the same typed reference (docs/cli.md § `ekr mcp`: "what `ekr resolve`
//! prints for that reference, byte for byte").
//!
//! The tool re-serializes its JSON arguments with `serde_json` and hands the text to the verb's
//! YAML reader. `serde_json` writes U+0085 (NEL) and U+007F (DEL) raw; YAML folds a raw NEL
//! inside a quoted scalar into a space and refuses a raw DEL as a control character. The one-shot
//! verb reads the same alias from a file that escapes it, and answers the node that holds it.

use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Output, Stdio};

use serde_json::{json, Value};

const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const TRANSACTION: &str = "00000000-0000-4000-8000-00000000a902";
const HOLDER: &str = "00000000-0000-4000-8000-00000000a911";
const LOOKALIKE: &str = "00000000-0000-4000-8000-00000000a912";
const DEL_HOLDER: &str = "00000000-0000-4000-8000-00000000a913";

fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

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
        let example = |kind: &str| {
            let output = ekr().args(["example", kind]).output().unwrap();
            assert_eq!(output.status.code(), Some(0));
            String::from_utf8(output.stdout).unwrap()
        };
        world.file("host.json", &example("ekr.cli-host/1"));
        world.file("seed.yaml", &example("ekr-seed/2"));
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

    fn commit(&self, file: &str) {
        let proposed: Value = serde_json::from_str(&self.ok(&["propose", file])).unwrap();
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        let validated: Value = serde_json::from_str(&self.ok(&["validate", &id])).unwrap();
        assert_eq!(validated["kind"], "Validated", "{validated}");
        let committed: Value = serde_json::from_str(&self.ok(&["commit", &id])).unwrap();
        assert_eq!(committed["kind"], "Committed", "{committed}");
    }

    /// `ekr mcp` fed `messages`, one per line, then end of input: every line it wrote.
    fn mcp(&self, messages: &[Value]) -> Vec<Value> {
        let mut child = self
            .command(&["mcp"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut input = child.stdin.take().unwrap();
        for message in messages {
            input.write_all(format!("{message}\n").as_bytes()).unwrap();
        }
        drop(input);
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0));
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

/// Organizations created with the aliases given as YAML double-quoted scalars, escapes and all.
fn create(nodes: &[(&str, &str)]) -> String {
    let mut document = format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {TRANSACTION}\n  proposer: \
         {OPERATOR}\n  operations:\n"
    );
    for (id, quoted) in nodes {
        document.push_str(&format!(
            "  - !CreateNode\n    id: {id}\n    root_id: {ROOT}\n    type_id: {ORGANIZATION}\n    \
             canonical_name: \"{quoted}\"\n    properties: {{}}\n    aliases: [\"{quoted}\"]\n"
        ));
    }
    document.push_str("  evidence: []\n");
    document
}

/// The `resolve` tool's answer for `reference`: its text, or the JSON-RPC error it became.
fn resolve_over_mcp(world: &World, reference: &Value) -> String {
    let answers = world.mcp(&[
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
               "params": {"protocolVersion": "2025-11-25", "capabilities": {}}}),
        json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
               "params": {"name": "resolve", "arguments": reference}}),
    ]);
    assert_eq!(answers.len(), 2, "{answers:?}");
    let answer = &answers[1];
    match answer["result"]["content"][0]["text"].as_str() {
        Some(text) => text.to_owned(),
        None => format!("JSON-RPC error: {}", answer["error"]),
    }
}

/// An alias holding U+0085 (NEL): the verb resolves the node holding it; the tool must answer the
/// same, not the node whose alias has a space where the NEL was.
#[test]
fn adversary_mcp_resolve_of_an_alias_holding_nel_answers_what_the_verb_prints() {
    for backend in ["file", "sqlite"] {
        let world = World::seeded(backend);
        world.file(
            "create.yaml",
            &create(&[(HOLDER, "Ac\\u0085me Ltd"), (LOOKALIKE, "Ac me Ltd")]),
        );
        world.commit("create.yaml");
        let reference = json!({"type_id": ORGANIZATION, "aliases": ["Ac\u{85}me Ltd"]});
        // serde_json's writer escapes nothing but controls below U+0020, `"` and `\`; write the
        // file with NEL escaped, as any JSON writer may.
        world.file(
            "reference.json",
            &format!("{{\"type_id\": \"{ORGANIZATION}\", \"aliases\": [\"Ac\\u0085me Ltd\"]}}"),
        );
        let verb = world.ok(&["resolve", "reference.json"]);
        let printed: Value = serde_json::from_str(&verb).unwrap();
        assert_eq!(printed["kind"], "Resolved", "{backend}: {printed}");
        assert_eq!(printed["node_id"], HOLDER, "{backend}: {printed}");

        let tool = resolve_over_mcp(&world, &reference);
        assert_eq!(
            tool, verb,
            "{backend}: the resolve tool answers another node than `ekr resolve` for the same \
             reference"
        );
    }
}

/// An alias holding U+007F (DEL): the verb resolves the node; the tool must not refuse the same
/// reference as one the reader cannot read.
#[test]
fn adversary_mcp_resolve_of_an_alias_holding_del_answers_what_the_verb_prints() {
    for backend in ["file", "sqlite"] {
        let world = World::seeded(backend);
        world.file("create.yaml", &create(&[(DEL_HOLDER, "Initech\\x7f")]));
        world.commit("create.yaml");
        let reference = json!({"type_id": ORGANIZATION, "aliases": ["Initech\u{7f}"]});
        world.file(
            "reference.json",
            &format!("{{\"type_id\": \"{ORGANIZATION}\", \"aliases\": [\"Initech\\u007f\"]}}"),
        );
        let verb = world.ok(&["resolve", "reference.json"]);
        let printed: Value = serde_json::from_str(&verb).unwrap();
        assert_eq!(printed["node_id"], DEL_HOLDER, "{backend}: {printed}");

        let tool = resolve_over_mcp(&world, &reference);
        assert_eq!(
            tool, verb,
            "{backend}: the resolve tool refuses a reference `ekr resolve` resolves"
        );
    }
}
