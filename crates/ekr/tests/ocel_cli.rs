//! `task:store-exports-ocel-event-log`: `ekr ocel` prints the `ekr.ocel/1` document of one
//! revision (`ekr.views.ExportOcel`), as a one-shot verb and as an `ekr session` verb, on both
//! providers.
//!
//! The store is the example seed. Its two Person nodes are each the subject of a CEO_OF
//! assertion with a valid time, so Person is an event type — Alice and Bob events at their
//! earliest valid time, 2020-01-01 — and Organization, whose one node Acme is the subject of
//! none, an object type. The CEO_OF edge from Alice to Acme is Alice's one relationship. Then the
//! example transaction, one more CEO_OF assertion about Alice valid from the same instant, as
//! revision 1: the log is the same, and only `meta.revision` moves. The rules themselves are held
//! on a richer store, with an OCEL 2.0 reader, in `crates/ekr-views/tests/ocel.rs` and the views
//! conformance suite.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Output, Stdio};

use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const PERSON: &str = "00000000-0000-4000-8000-000000000201";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const CEO_OF: &str = "00000000-0000-4000-8000-000000000203";
const LEGAL_NAME: &str = "00000000-0000-4000-8000-000000000801";
const ALICE: &str = "00000000-0000-4000-8000-000000000301";
const BOB: &str = "00000000-0000-4000-8000-000000000302";
const ACME: &str = "00000000-0000-4000-8000-000000000303";

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

    /// Revision 1: the example transaction, as it is.
    fn commit_revision_one(&self) {
        let document = self.file(
            "revision-1.yaml",
            &text(&["example", "ekr.transaction-document/2"]),
        );
        let id = self.ok(&["propose", &document])["transaction_id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(self.ok(&["validate", &id])["kind"], "Validated");
        assert_eq!(self.ok(&["commit", &id])["kind"], "Committed");
    }
}

/// The example store's log, at any of its revisions.
fn example_log() -> Value {
    let event = |id: &str, relationships: Value| {
        json!({
            "id": id,
            "type": PERSON,
            "time": "2020-01-01T00:00:00.000Z",
            "attributes": [],
            "relationships": relationships,
        })
    };
    json!({
        "eventTypes": [{"name": PERSON, "attributes": []}],
        "objectTypes": [
            {"name": ORGANIZATION, "attributes": [{"name": LEGAL_NAME, "type": "string"}]},
        ],
        "events": [
            event(ALICE, json!([{"objectId": ACME, "qualifier": CEO_OF}])),
            event(BOB, json!([])),
        ],
        "objects": [
            {"id": ACME, "type": ORGANIZATION, "attributes": [], "relationships": []},
        ],
    })
}

#[test]
fn ocel_prints_each_revision_of_the_example_store_as_an_ocel_2_0_log_on_both_providers() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        assert_eq!(
            world.ok(&["ocel"]),
            json!({"meta": {"format": "ekr.ocel/1", "revision": 0}, "ocel": example_log()}),
            "{backend}"
        );
        world.commit_revision_one();
        assert_eq!(
            world.ok(&["ocel"]),
            json!({"meta": {"format": "ekr.ocel/1", "revision": 1}, "ocel": example_log()}),
            "{backend}"
        );
        assert_eq!(
            world.ok(&["ocel", "--revision", "0"])["meta"]["revision"],
            0,
            "{backend}: revision 0 after revision 1"
        );
    }
}

#[test]
fn two_reads_of_one_revision_print_the_same_bytes_on_both_providers() {
    let mut per_backend = Vec::new();
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let seeded = world.stdout(&["ocel"]);
        world.commit_revision_one();
        let first = world.stdout(&["ocel", "--revision", "1"]);
        assert_eq!(
            first,
            world.stdout(&["ocel", "--revision", "1"]),
            "{backend}"
        );
        assert_eq!(first, world.stdout(&["ocel"]), "{backend}: the head is 1");
        assert_eq!(
            seeded,
            world.stdout(&["ocel", "--revision", "0"]),
            "{backend}: revision 0 before and after revision 1"
        );
        assert_ne!(first, seeded, "{backend}: meta.revision differs");
        per_backend.push((seeded, first));
    }
    assert_eq!(per_backend[0], per_backend[1], "file and sqlite");
}

#[test]
fn a_session_serves_ocel_as_the_one_shot_verb_prints_it() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        world.commit_revision_one();
        let answers = world.session(&[
            json!({"argv": ["ocel"]}),
            json!({"argv": ["ocel", "--revision", "0"]}),
            json!({"argv": ["ocel", "--revision", "7"]}),
        ]);
        assert_eq!(answers.len(), 3, "{backend}");
        for (answer, one_shot) in answers
            .iter()
            .zip([world.ok(&["ocel"]), world.ok(&["ocel", "--revision", "0"])])
        {
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
        let beyond = world.run(&["ocel", "--revision", "4"]);
        assert_eq!(beyond.status.code(), Some(2), "{backend}");
        let stderr = String::from_utf8_lossy(&beyond.stderr);
        assert!(stderr.contains("ekr.views.RevisionNotFound"), "{stderr}");
        assert!(beyond.stdout.is_empty());

        let empty = World::empty(backend);
        let missing = empty.run(&["ocel"]);
        assert_eq!(missing.status.code(), Some(1), "{backend}");
        assert!(
            String::from_utf8_lossy(&missing.stderr).contains("store-not-found"),
            "{backend}"
        );
        assert!(!empty.store().exists(), "{backend}: nothing was created");
    }
}
