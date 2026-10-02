//! `task:store-exports-ocel-event-log`: `ekr ocel` prints the `ekr.ocel/1` document of one
//! revision (`ekr.views.ExportOcel`), as a one-shot verb and as an `ekr session` verb, on both
//! providers, with the event types the viewer's rule gives or the ones `--events` names.
//!
//! The store is the example seed; then the example transaction, one more CEO_OF assertion about
//! Alice valid from 2020-01-01, as revision 1. By the viewer's rule the store has no event type
//! at either revision, so every node is an object and only `meta.revision` moves. Named with
//! `--events Person`, Alice and Bob are events at their earliest dated fact, 2020-01-01. The
//! rules themselves are held on a richer store, with an OCEL 2.0 reader, in
//! `crates/ekr-views/tests/ocel.rs` and the views conformance suite.

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

/// Each id of the example store with its name.
fn example_names() -> Value {
    json!({
        "node_types": [
            {"id": PERSON, "name": "Person"},
            {"id": ORGANIZATION, "name": "Organization"},
        ],
        "edge_types": [{"id": CEO_OF, "name": "CEO_OF"}],
        "properties": [{"id": LEGAL_NAME, "name": "legal_name"}],
    })
}

fn organization() -> Value {
    json!({"name": ORGANIZATION, "attributes": [{"name": LEGAL_NAME, "type": "string"}]})
}

/// The example store's log by the viewer's rule, at either revision: no type is an event type —
/// Person's Bob and Organization's Acme each have dated facts spread over years, and Alice alone
/// is not enough — so every node is an object, and the CEO_OF edge an object-to-object
/// relationship on Alice.
fn example_log() -> Value {
    let object = |id: &str, object_type: &str, relationships: Value| json!({"id": id, "type": object_type, "attributes": [], "relationships": relationships});
    json!({
        "eventTypes": [],
        "objectTypes": [{"name": PERSON, "attributes": []}, organization()],
        "events": [],
        "objects": [
            object(ALICE, PERSON, json!([{"objectId": ACME, "qualifier": CEO_OF}])),
            object(BOB, PERSON, json!([])),
            object(ACME, ORGANIZATION, json!([])),
        ],
    })
}

/// The example store's log with Person named as the event type: Alice and Bob events at their
/// earliest dated fact, 2020-01-01, and the CEO_OF edge on Alice.
fn person_log() -> Value {
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
        "objectTypes": [organization()],
        "events": [
            event(ALICE, json!([{"objectId": ACME, "qualifier": CEO_OF}])),
            event(BOB, json!([])),
        ],
        "objects": [
            {"id": ACME, "type": ORGANIZATION, "attributes": [], "relationships": []},
        ],
    })
}

fn document(revision: u64, log: Value) -> Value {
    json!({
        "meta": {"format": "ekr.ocel/1", "revision": revision},
        "names": example_names(),
        "ocel": log,
    })
}

#[test]
fn ocel_counts_are_returned_on_each_requests_stderr_without_leaking_to_the_next_request() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let output = world.run(&["ocel"]);
        assert_eq!(output.status.code(), Some(0));
        let counts: Value =
            serde_json::from_slice(&output.stderr).expect("one JSON summary on stderr");
        assert_eq!(counts["events"], 0);
        assert_eq!(counts["objects"], 3);
        assert_eq!(counts["object_object_relationships"], 1);
        let answers = world.session(&[
            json!({"argv": ["ocel"]}),
            json!({"argv": ["head"]}),
            json!({"argv": ["ocel", "--events", "Person"]}),
        ]);
        assert_eq!(
            answers[0]["stderr"],
            String::from_utf8(output.stderr).unwrap()
        );
        assert_eq!(answers[1]["stderr"], "");
        let named = world.run(&["ocel", "--events", "Person"]);
        assert_eq!(
            answers[2]["stderr"],
            String::from_utf8(named.stderr).unwrap()
        );
    }
}

#[test]
fn ocel_prints_each_revision_of_the_example_store_as_an_ocel_2_0_log_on_both_providers() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        assert_eq!(world.ok(&["ocel"]), document(0, example_log()), "{backend}");
        world.commit_revision_one();
        assert_eq!(world.ok(&["ocel"]), document(1, example_log()), "{backend}");
        assert_eq!(
            world.ok(&["ocel", "--revision", "0"]),
            document(0, example_log()),
            "{backend}: revision 0 after revision 1"
        );
    }
}

#[test]
fn events_names_the_event_types_instead_of_the_rule() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        assert_eq!(
            world.ok(&["ocel", "--events", "Person"]),
            document(0, person_log()),
            "{backend}"
        );
        world.commit_revision_one();
        assert_eq!(
            world.ok(&["ocel", "--revision", "1", "--events", "Person"]),
            document(1, person_log()),
            "{backend}"
        );
        // Both types, as one flag with two values or the flag twice.
        let both = world.stdout(&["ocel", "--events", "Person", "Organization"]);
        assert_eq!(
            both,
            world.stdout(&["ocel", "--events", "Organization", "--events", "Person"]),
            "{backend}"
        );
        let both: Value = serde_json::from_slice(&both).unwrap();
        assert_eq!(
            both["ocel"]["events"].as_array().unwrap().len(),
            3,
            "{backend}"
        );
        assert_eq!(both["ocel"]["objects"], json!([]), "{backend}");
    }
}

#[test]
fn a_name_no_node_type_holds_is_refused_by_name() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let refused = world.run(&["ocel", "--events", "Person", "Nope"]);
        assert_eq!(refused.status.code(), Some(2), "{backend}");
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert!(
            stderr.contains("ekr.views.EventTypeNotFound") && stderr.contains("\"Nope\""),
            "{backend}: {stderr}"
        );
        assert!(refused.stdout.is_empty(), "{backend}");
    }
}

#[test]
fn two_reads_of_one_revision_print_the_same_bytes_on_both_providers() {
    let mut per_backend = Vec::new();
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let seeded = world.stdout(&["ocel"]);
        let named = world.stdout(&["ocel", "--events", "Person"]);
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
        assert_eq!(
            named,
            world.stdout(&["ocel", "--revision", "0", "--events", "Person"]),
            "{backend}: named, revision 0 before and after revision 1"
        );
        assert_ne!(first, seeded, "{backend}: meta.revision differs");
        per_backend.push((seeded, named, first));
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
            json!({"argv": ["ocel", "--revision", "0", "--events", "Person"]}),
            json!({"argv": ["ocel", "--revision", "7"]}),
            json!({"argv": ["ocel", "--events", "Nope"]}),
        ]);
        assert_eq!(answers.len(), 4, "{backend}");
        for (answer, one_shot) in answers.iter().zip([
            world.ok(&["ocel"]),
            world.ok(&["ocel", "--revision", "0", "--events", "Person"]),
        ]) {
            assert_eq!(answer["exit"], 0, "{backend}: {answer}");
            assert_eq!(answer["stdout"], one_shot, "{backend}");
        }
        for (answer, refusal) in answers[2..]
            .iter()
            .zip(["ekr.views.RevisionNotFound", "ekr.views.EventTypeNotFound"])
        {
            assert_eq!(answer["exit"], 2, "{backend}: {answer}");
            assert!(
                answer["stderr"].as_str().unwrap().contains(refusal),
                "{backend}: {answer}"
            );
        }
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

#[test]
fn adversary_ocel_diagnostics_stay_inside_their_request_across_errors_and_raw_reports() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let default = world.run(&["ocel"]);
        let named = world.run(&["ocel", "--events", "Person"]);
        assert_eq!(default.status.code(), Some(0));
        assert_eq!(named.status.code(), Some(0));
        let requests = [
            json!({"argv": ["ocel"]}),
            json!({"argv": ["ocel", "--event-time", "Person.absent"]}),
            json!({"argv": ["fact-quality", "-"], "stdin": "{\"format\":\"ekr.fact-judgements/1\",\"judgements\":[]}"}),
            json!({"argv": ["ocel", "--events", "Person"]}),
            json!({"argv": ["head"]}),
        ];
        let mut child = world
            .command(&["session"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let mut stdin = child.stdin.take().unwrap();
            for request in requests {
                writeln!(stdin, "{request}").unwrap();
            }
        }
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert!(
            output.stderr.is_empty(),
            "success counts must not reach global stderr: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let replies: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(replies.len(), 5);
        for (index, one_shot) in [(0, default), (3, named)] {
            assert_eq!(replies[index]["exit"], 0);
            assert_eq!(
                replies[index]["stdout"],
                serde_json::from_slice::<Value>(&one_shot.stdout).unwrap()
            );
            assert_eq!(
                replies[index]["stderr"],
                String::from_utf8(one_shot.stderr).unwrap()
            );
        }
        assert_eq!(replies[1]["exit"], 2);
        assert!(replies[1]["stderr"]
            .as_str()
            .unwrap()
            .contains("ekr.views.EventTimeInvalid"));
        assert_eq!(replies[2]["stdout"]["rate"], Value::Null);
        assert_eq!(replies[2]["stderr"], "");
        assert_eq!(replies[4]["stderr"], "");
    }
}
