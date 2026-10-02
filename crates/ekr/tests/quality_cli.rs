//! `story:store-quality-report`: `ekr quality` prints the `ekr.store-quality/1` document of one
//! revision (`ekr.views.ReportStoreQuality`), as a one-shot verb and as an `ekr session` verb, on
//! both providers.
//!
//! The store is the example seed — three assertions citing seeded evidence, one property
//! declaration (`legal_name`, unconstrained), no name two nodes of one type share — and then one
//! transaction: an `AddEvidence`, an assertion citing only that evidence, and a second Person
//! named `Alice`. So revision 0 reports 3 active assertions, all with evidence and none with
//! evidence added after the seed, and no shared name; revision 1 reports 4, 1 of them citing the
//! added evidence, and `Alice` shared by two Person nodes. The figures themselves are held on a
//! richer store in `crates/ekr-views/tests/quality.rs` and the views conformance suite.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Output, Stdio};

use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const PERSON: &str = "00000000-0000-4000-8000-000000000201";
const ALICE: &str = "00000000-0000-4000-8000-000000000301";

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

fn minted(kind: &str) -> String {
    let minted: Value = serde_json::from_str(&text(&["mint", kind])).unwrap();
    minted["id"].as_str().unwrap().to_owned()
}

/// One provider in its own directory, seeded from `ekr example ekr-seed/2`.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn seeded(backend: &'static str) -> Self {
        let world = Self::empty(backend);
        let seed = world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        assert_eq!(world.ok(&["seed", &seed])["result"]["revision"], 0);
        world
    }

    fn empty(backend: &'static str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        world.file("host.json", &text(&["example", "ekr.cli-host/1"]));
        world
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn file(&self, name: &str, contents: &str) -> String {
        let path = self.directory.path().join(name);
        std::fs::write(&path, contents).unwrap();
        path.display().to_string()
    }

    fn command(&self, verb: &[&str]) -> std::process::Command {
        let mut command = ekr();
        command
            .arg("--host")
            .arg(self.directory.path().join("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(verb);
        command
    }

    fn run(&self, verb: &[&str]) -> Output {
        self.command(verb).output().unwrap()
    }

    /// Exit 0 and one JSON result on stdout.
    fn ok(&self, verb: &[&str]) -> Value {
        serde_json::from_slice(&self.stdout(verb)).unwrap()
    }

    /// Exit 0 and stdout's exact bytes.
    fn stdout(&self, verb: &[&str]) -> Vec<u8> {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    /// The answers of an `ekr session` to `requests`, one per line.
    fn session(&self, requests: &[Value]) -> Vec<Value> {
        let mut child = self
            .command(&["session"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let mut stdin = child.stdin.take().unwrap();
            for request in requests {
                writeln!(stdin, "{request}").unwrap();
            }
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{}", self.backend);
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    /// Revision 1: an `AddEvidence` of the `ekr operations AddEvidence` example, the example
    /// transaction's assertion citing only that evidence, and a second Person named `Alice`.
    /// Answers the new node's id.
    fn commit_revision_one(&self) -> String {
        let listed = text(&["operations", "AddEvidence"]);
        let added: String = listed
            .lines()
            .skip_while(|line| !line.starts_with("- !AddEvidence"))
            .map(|line| format!("  {line}\n"))
            .collect();
        assert!(added.contains("payload:"), "{listed}");
        let node = minted("node");
        let created = format!(
            "  - !CreateNode\n    id: {node}\n    root_id: 00000000-0000-4000-8000-000000000002\n    \
             type_id: {PERSON}\n    canonical_name: Alice\n    properties: {{}}\n    aliases: []\n"
        );
        let transaction = text(&["example", "ekr.transaction-document/2"])
            .replace("000000000401", "000000000403")
            .replace(
                "  operations:\n",
                &format!("  operations:\n{added}{created}"),
            );
        let document = self.file("revision-1.yaml", &transaction);
        let id = self.ok(&["propose", &document])["transaction_id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(
            self.ok(&["validate", &id])["kind"],
            "Validated",
            "{transaction}"
        );
        assert_eq!(self.ok(&["commit", &id])["kind"], "Committed");
        node
    }
}

fn document(revision: u64, active: u64, item: u64, shared: Value, sharing: u64) -> Value {
    json!({
        "meta": {"format": "ekr.store-quality/1", "revision": revision},
        "assertions": {
            "active": active,
            "with_evidence": active,
            "with_item_evidence": item,
            "with_seed_evidence": 3,
            "with_evidence_share": 10_000,
            "with_item_evidence_share": item * 10_000 / active,
        },
        "properties": {"declared": 1, "constrained": 0, "constrained_types": 0, "constrained_share": 0},
        "shared_names": shared,
        "sharing_nodes": sharing,
    })
}

#[test]
fn quality_reports_each_revision_of_a_cli_store_on_both_providers() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let seeded = document(0, 3, 0, json!([]), 0);
        assert_eq!(world.ok(&["quality"]), seeded, "{backend}");
        let node = world.commit_revision_one();
        let mut alices = [ALICE.to_owned(), node];
        alices.sort();
        let shared = json!([{"type": PERSON, "name": "Alice", "nodes": alices}]);
        assert_eq!(
            world.ok(&["quality"]),
            document(1, 4, 1, shared, 2),
            "{backend}"
        );
        assert_eq!(
            world.ok(&["quality", "--revision", "0"]),
            seeded,
            "{backend}: revision 0 after revision 1"
        );
    }
}

#[test]
fn two_reads_of_one_revision_print_the_same_bytes_and_no_revision_reads_the_head() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        world.commit_revision_one();
        let first = world.stdout(&["quality", "--revision", "1"]);
        assert_eq!(
            first,
            world.stdout(&["quality", "--revision", "1"]),
            "{backend}"
        );
        assert_eq!(
            first,
            world.stdout(&["quality"]),
            "{backend}: the head is 1"
        );
        assert_ne!(
            first,
            world.stdout(&["quality", "--revision", "0"]),
            "{backend}"
        );
    }
}

#[test]
fn a_session_serves_quality_as_the_one_shot_verb_prints_it() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        world.commit_revision_one();
        let answers = world.session(&[
            json!({"argv": ["quality"]}),
            json!({"argv": ["quality", "--revision", "0"]}),
            json!({"argv": ["quality", "--revision", "7"]}),
        ]);
        assert_eq!(answers.len(), 3, "{backend}");
        for (answer, one_shot) in answers.iter().zip([
            world.ok(&["quality"]),
            world.ok(&["quality", "--revision", "0"]),
        ]) {
            assert_eq!(answer["exit"], 0, "{backend}: {answer}");
            assert_eq!(answer["stdout"], one_shot, "{backend}");
        }
        assert_eq!(answers[2]["exit"], 2, "{backend}: {}", answers[2]);
        assert!(
            answers[2]["stderr"]
                .as_str()
                .unwrap()
                .contains("ekr.views.RevisionNotFound"),
            "{backend}: {}",
            answers[2]
        );
    }
}

#[test]
fn a_revision_beyond_the_head_is_refused_by_name_and_no_store_is_a_fault() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let beyond = world.run(&["quality", "--revision", "4"]);
        assert_eq!(beyond.status.code(), Some(2), "{backend}");
        let stderr = String::from_utf8_lossy(&beyond.stderr);
        assert!(stderr.contains("ekr.views.RevisionNotFound"), "{stderr}");
        assert!(beyond.stdout.is_empty());

        let empty = World::empty(backend);
        let missing = empty.run(&["quality"]);
        assert_eq!(missing.status.code(), Some(1), "{backend}");
        assert!(
            String::from_utf8_lossy(&missing.stderr).contains("store-not-found"),
            "{backend}"
        );
        assert!(!empty.store().exists(), "{backend}: nothing was created");
    }
}
