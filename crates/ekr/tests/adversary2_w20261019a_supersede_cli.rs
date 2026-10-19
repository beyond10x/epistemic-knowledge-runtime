//! Adversary pass 2, wave 2026-10-19a, `story:supersede-node`: the `ekr` binary end to end, on
//! both providers, over the example seed (`ekr example ekr-seed/2`). Every write goes through
//! `ekr propose`, `ekr validate` and `ekr commit` of a YAML transaction document.
//!
//! 1. ADR 0017 decision 3: "a read that reaches it from elsewhere names the retired id with its
//!    pointer". Fix `35fe08743` made the fact sample name a superseded *subject*'s pointer
//!    (`subject_superseded_by`). Superseding Acme leaves Alice's seeded assertion
//!    `Alice CEO_OF Acme` (`…0510`) active with Acme as its *object*; the three reads that reach
//!    it from Alice or by its id — `describe_node` (`ekr.node-detail/1`), `ekr sample` and
//!    `ekr explain` — must name Globex, the node Acme is superseded by.
//! 2. `ekr compare` answers "whether two stores hold the same knowledge" (`docs/cli.md`). Two
//!    stores that superseded the same duplicate by different nodes resolve its key to different
//!    nodes, so they do not.

use std::io::Write as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const PERSON: &str = "00000000-0000-4000-8000-000000000201";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ALICE: &str = "00000000-0000-4000-8000-000000000301";
const BOB: &str = "00000000-0000-4000-8000-000000000302";
const ACME: &str = "00000000-0000-4000-8000-000000000303";
/// The seeded assertion `Alice CEO_OF Acme`.
const ALICE_CEO_OF_ACME: &str = "00000000-0000-4000-8000-000000000510";
const ALICE_EVIDENCE: &str = "00000000-0000-4000-8000-000000000401";
/// Globex, the Organization Acme is superseded by.
const GLOBEX: &str = "00000000-0000-4000-8000-00000000b001";
/// A Person recorded twice, known as `dup`.
const DUPLICATE: &str = "00000000-0000-4000-8000-00000000b011";

fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in [
        "EKR_HOST",
        "EKR_STORE",
        "EKR_BACKEND",
        "EKR_FULL_REPLAY",
        "EKR_STAGE",
    ] {
        command.env_remove(var);
    }
    command
}

fn printed(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
    store: &'static str,
}

impl World {
    fn seeded(directory: tempfile::TempDir, backend: &'static str, store: &'static str) -> Self {
        let world = Self {
            directory,
            backend,
            store,
        };
        let host = world.file("host.json");
        if !host.exists() {
            std::fs::write(&host, printed(&["example", "ekr.cli-host/1"])).unwrap();
            std::fs::write(world.file("seed.yaml"), printed(&["example", "ekr-seed/2"])).unwrap();
        }
        let seed = world.file("seed.yaml");
        world.ok(&["seed", seed.to_str().unwrap()]);
        world
    }

    fn new(backend: &'static str) -> Self {
        Self::seeded(tempfile::tempdir().unwrap(), backend, "store")
    }

    fn file(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn command(&self) -> Command {
        let mut command = ekr();
        command
            .arg("--host")
            .arg(self.file("host.json"))
            .arg("--store")
            .arg(self.file(self.store))
            .args(["--backend", self.backend]);
        command
    }

    fn run(&self, args: &[&str]) -> (Option<i32>, String, String) {
        let output = self.command().args(args).output().unwrap();
        (
            output.status.code(),
            String::from_utf8(output.stdout).unwrap(),
            String::from_utf8(output.stderr).unwrap(),
        )
    }

    fn ok(&self, args: &[&str]) -> Value {
        let (code, stdout, stderr) = self.run(args);
        assert_eq!(code, Some(0), "{} {args:?}: {stderr}", self.backend);
        serde_json::from_str(&stdout).unwrap()
    }

    /// Proposes, validates and commits the transaction document `yaml`, whose id is `id`.
    fn commit(&self, name: &str, id: &str, yaml: &str) {
        let path = self.file(name);
        std::fs::write(&path, yaml).unwrap();
        self.ok(&["propose", path.to_str().unwrap()]);
        let validated = self.ok(&["validate", id]);
        assert_eq!(
            validated["kind"], "Validated",
            "{}: {validated}",
            self.backend
        );
        let committed = self.ok(&["commit", id]);
        assert_eq!(
            committed["kind"], "Committed",
            "{}: {committed}",
            self.backend
        );
    }

    /// The answers `ekr mcp` gives to `calls`, each a tool and its arguments, in order.
    fn mcp(&self, calls: &[(&str, Value)]) -> Vec<Value> {
        let mut lines = vec![
            serde_json::json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
                "protocolVersion": "2025-06-18", "capabilities": {},
                "clientInfo": {"name": "adversary", "version": "0"}}}),
            serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
        ];
        for (at, (tool, arguments)) in calls.iter().enumerate() {
            lines.push(serde_json::json!({"jsonrpc": "2.0", "id": at + 1,
                "method": "tools/call", "params": {"name": tool, "arguments": arguments}}));
        }
        let mut child = self
            .command()
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let mut stdin = child.stdin.take().unwrap();
            for line in &lines {
                writeln!(stdin, "{line}").unwrap();
            }
        }
        let output = child.wait_with_output().unwrap();
        let answers: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        (1..=calls.len())
            .map(|at| {
                answers
                    .iter()
                    .find(|answer| answer["id"] == at)
                    .cloned()
                    .unwrap_or(Value::Null)
            })
            .collect()
    }
}

/// The transaction document `id` holding `operations`, resting on `evidence`.
fn document(id: &str, operations: &str, evidence: &[&str]) -> String {
    let listed = if evidence.is_empty() {
        " []".to_owned()
    } else {
        evidence
            .iter()
            .map(|id| format!("\n  - {id}"))
            .collect::<String>()
    };
    format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {id}\n  proposer: {OPERATOR}\n  \
         operations:\n{operations}  evidence:{listed}\n"
    )
}

fn create(node: &str, type_id: &str, name: &str, alias: &str) -> String {
    format!(
        "  - !CreateNode\n    id: {node}\n    root_id: {ROOT}\n    type_id: {type_id}\n    \
         canonical_name: {name}\n    properties: {{}}\n    aliases:\n    - {alias}\n"
    )
}

fn supersede(node: &str, by: &str) -> String {
    format!(
        "  - !SupersedeNode\n    node: {node}\n    by: {by}\n    reason: recorded twice\n    \
         evidence:\n    - {ALICE_EVIDENCE}\n"
    )
}

#[test]
fn adv2_a_read_reaching_an_assertion_whose_object_is_superseded_names_the_pointer() {
    for backend in BACKENDS {
        let world = World::new(backend);
        world.commit(
            "supersede.yaml",
            "00000000-0000-4000-8000-00000000a001",
            &document(
                "00000000-0000-4000-8000-00000000a001",
                &(create(GLOBEX, ORGANIZATION, "Globex", "globex") + &supersede(ACME, GLOBEX)),
                &[ALICE_EVIDENCE],
            ),
        );
        let mut unnamed = Vec::new();

        let answers = world.mcp(&[(
            "describe_node",
            serde_json::json!({"node": ALICE, "full": true}),
        )]);
        let detail = &answers[0]["result"]["structuredContent"];
        let listed: Vec<&Value> = detail["assertions"]
            .as_array()
            .map(|all| {
                all.iter()
                    .filter(|a| a["id"] == ALICE_CEO_OF_ACME)
                    .collect()
            })
            .unwrap_or_default();
        assert_eq!(
            listed.len(),
            1,
            "{backend}: describe_node lists 0510: {detail}"
        );
        if !listed[0].to_string().contains(GLOBEX) {
            unnamed.push(format!("describe_node (ekr.node-detail/1): {}", listed[0]));
        }

        let sample = world.ok(&["sample", "--seed", "1", "--size", "10"]);
        let item = sample["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["assertion"]["id"] == ALICE_CEO_OF_ACME)
            .cloned();
        let item = item.unwrap_or_else(|| panic!("{backend}: the sample holds 0510: {sample}"));
        if !item.to_string().contains(GLOBEX) {
            let shown: Value = item
                .as_object()
                .unwrap()
                .iter()
                .filter(|(key, _)| *key != "evidence")
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<serde_json::Map<_, _>>()
                .into();
            unnamed.push(format!("ekr sample: {shown}"));
        }

        let explained = world.ok(&["explain", ALICE_CEO_OF_ACME]);
        if !explained.to_string().contains(GLOBEX) {
            unnamed.push(format!(
                "ekr explain: its assertion link names object {}",
                explained["links"][0]["object"]
            ));
        }

        assert!(
            unnamed.is_empty(),
            "{backend}: reads that reach Alice CEO_OF Acme name Acme without the node it is \
             superseded by ({GLOBEX}):\n{}",
            unnamed.join("\n")
        );
    }
}

#[test]
fn adv2_two_stores_whose_duplicate_is_superseded_by_different_nodes_do_not_compare_equal() {
    for backend in BACKENDS {
        let directory = tempfile::tempdir().unwrap();
        let mut sides = Vec::new();
        for (store, by) in [("left", ALICE), ("right", BOB)] {
            let world = World::seeded(
                tempfile::TempDir::new_in(directory.path()).unwrap(),
                backend,
                store,
            );
            world.commit(
                "duplicate.yaml",
                "00000000-0000-4000-8000-00000000a011",
                &document(
                    "00000000-0000-4000-8000-00000000a011",
                    &create(DUPLICATE, PERSON, "Dup", "dup"),
                    &[],
                ),
            );
            world.commit(
                "supersede.yaml",
                "00000000-0000-4000-8000-00000000a012",
                &document(
                    "00000000-0000-4000-8000-00000000a012",
                    &supersede(DUPLICATE, by),
                    &[ALICE_EVIDENCE],
                ),
            );
            let reference = world.file("reference.yaml");
            std::fs::write(&reference, format!("type_id: {PERSON}\naliases:\n- dup\n")).unwrap();
            let resolved = world.ok(&["resolve", reference.to_str().unwrap()]);
            assert_eq!(
                resolved["kind"], "Resolved",
                "{backend} {store}: {resolved}"
            );
            assert_eq!(resolved["node_id"], by, "{backend} {store}: {resolved}");
            sides.push(world);
        }
        let other = sides[1].file("right");
        let comparison = sides[0].ok(&["compare", "--other", other.to_str().unwrap()]);
        assert_eq!(
            comparison["equal"], false,
            "{backend}: `dup` resolves to Alice in one store and to Bob in the other, and \
             ekr compare answers that they hold the same knowledge: {comparison}"
        );
    }
}
