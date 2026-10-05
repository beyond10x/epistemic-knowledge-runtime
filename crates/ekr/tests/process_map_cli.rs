//! `story:ocel-process-map`: `ekr process-map` prints the `ekr.process-map/1` document of one
//! revision (`ekr.views.ProjectProcessMap`), as a one-shot verb and as an `ekr session` verb, on
//! both providers.
//!
//! The store is `tests/fixtures/process-map/seed.yaml`: object types Ticket and Reviewer, event
//! types Opened, Reviewed and Closed, each event a node with a Timestamp `at` and a CONCERNS edge
//! to its ticket. T-1 and T-2 are opened, reviewed and closed and T-3 opened and closed, so the
//! log's three tickets follow two variants; the two reviews are BY one reviewer. The rules
//! themselves are held on the engine, against the `ekr.ocel/1` document, in
//! `crates/ekr-views/tests/process_map.rs` and the views conformance suite.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Output, Stdio};

use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const TICKET: &str = "00000000-0000-4000-8000-00000000c201";
const REVIEWER: &str = "00000000-0000-4000-8000-00000000c202";
const OPENED: &str = "00000000-0000-4000-8000-00000000c203";
const REVIEWED: &str = "00000000-0000-4000-8000-00000000c204";
const CLOSED: &str = "00000000-0000-4000-8000-00000000c205";

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

fn seed() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
    .join("tests/fixtures/process-map/seed.yaml")
}

/// One provider in its own directory, seeded from the fixture under the example host.
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
        let host = ekr().args(["example", "ekr.cli-host/1"]).output().unwrap();
        assert_eq!(host.status.code(), Some(0));
        std::fs::write(world.directory.path().join("host.json"), host.stdout).unwrap();
        let seed = seed().display().to_string();
        assert_eq!(
            world.ok(&["seed", &seed])["result"]["revision"],
            0,
            "{backend}"
        );
        world
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
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
        self.command(verb).stdin(Stdio::null()).output().unwrap()
    }

    /// Exit 0 and one JSON result on stdout.
    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
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

    /// The `ocel_hash` `ekr ocel` reports on stderr for `args`.
    fn ocel_hash(&self, args: &[&str]) -> Value {
        let mut verb = vec!["ocel"];
        verb.extend_from_slice(args);
        let output = self.run(&verb);
        assert_eq!(output.status.code(), Some(0), "{}", self.backend);
        let counts: Value = serde_json::from_slice(&output.stderr).expect("the export's counts");
        counts["ocel_hash"].clone()
    }
}

fn variant(activities: &[&str], cases: u64) -> Value {
    json!({"activities": activities, "cases": cases})
}

fn follows(from: &str, to: &str, count: u64) -> Value {
    json!({"from": from, "to": to, "count": count})
}

/// Acceptance: over a store whose OCEL log has 3 cases following two variants, `ekr
/// process-map` prints both variants with their counts and a directly-follows graph whose edge
/// counts match the log, and names the log it was derived from by the hash `ekr ocel` reports.
#[test]
fn process_map_prints_both_variants_and_the_logs_directly_follows_counts() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let output = world.run(&["process-map"]);
        assert_eq!(output.status.code(), Some(0), "{backend}");
        assert!(output.stderr.is_empty(), "{backend}");
        let map: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            map,
            json!({
                "meta": {
                    "format": "ekr.process-map/1",
                    "revision": 0,
                    "ocel_hash": world.ocel_hash(&[]),
                },
                "names": {"node_types": [
                    {"id": TICKET, "name": "Ticket"},
                    {"id": REVIEWER, "name": "Reviewer"},
                    {"id": OPENED, "name": "Opened"},
                    {"id": REVIEWED, "name": "Reviewed"},
                    {"id": CLOSED, "name": "Closed"},
                ]},
                "object_types": [
                    {
                        "object_type": TICKET,
                        "objects": 3,
                        "cases": 3,
                        "variants": [
                            variant(&[OPENED, REVIEWED, CLOSED], 2),
                            variant(&[OPENED, CLOSED], 1),
                        ],
                        "directly_follows": [
                            follows(OPENED, REVIEWED, 2),
                            follows(OPENED, CLOSED, 1),
                            follows(REVIEWED, CLOSED, 2),
                        ],
                    },
                    {
                        "object_type": REVIEWER,
                        "objects": 1,
                        "cases": 1,
                        "variants": [variant(&[REVIEWED, REVIEWED], 1)],
                        "directly_follows": [follows(REVIEWED, REVIEWED, 1)],
                    },
                ],
            }),
            "{backend}"
        );
    }
}

/// `--revision`, `--events` and `--event-time` are `ekr ocel`'s and select the log they select
/// there; a name no node type holds is refused by name, exit 2, with nothing on stdout.
#[test]
fn the_log_selecting_options_are_ekr_ocels_and_refuse_as_it_refuses() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let named = world.ok(&[
            "process-map",
            "--revision",
            "0",
            "--events",
            "Opened",
            "Closed",
        ]);
        assert_eq!(
            named["meta"]["ocel_hash"],
            world.ocel_hash(&["--revision", "0", "--events", "Opened", "Closed"]),
            "{backend}"
        );
        let ticket = &named["object_types"][0];
        assert_eq!(ticket["object_type"], TICKET, "{backend}");
        assert_eq!(ticket["variants"], json!([variant(&[OPENED, CLOSED], 3)]));
        assert_eq!(
            ticket["directly_follows"],
            json!([follows(OPENED, CLOSED, 3)])
        );

        let timed = world.ok(&[
            "process-map",
            "--event-time",
            "Opened.at",
            "--event-time",
            "Closed.at",
        ]);
        assert_eq!(
            timed["object_types"][0]["variants"], ticket["variants"],
            "{backend}"
        );

        let refused = world.run(&["process-map", "--events", "Opened", "Nope"]);
        assert_eq!(refused.status.code(), Some(2), "{backend}");
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert!(
            stderr.contains("ekr.views.EventTypeNotFound") && stderr.contains("\"Nope\""),
            "{backend}: {stderr}"
        );
        assert!(refused.stdout.is_empty(), "{backend}");

        let beyond = world.run(&["process-map", "--revision", "3"]);
        assert_eq!(beyond.status.code(), Some(2), "{backend}");
        assert!(
            String::from_utf8_lossy(&beyond.stderr).contains("ekr.views.RevisionNotFound"),
            "{backend}"
        );
    }
}

/// `docs/cli.md` § `ekr process-map` prints, for the store it describes, what the binary prints
/// for this file's store, byte for byte: the page's example is this fixture's output.
#[test]
fn the_pages_example_is_what_process_map_prints_for_the_store_it_describes() {
    let page = std::fs::read_to_string(
        PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../../docs/cli.md"),
    )
    .unwrap();
    let section = page
        .split("\n### `ekr process-map`\n")
        .nth(1)
        .and_then(|rest| rest.split("\n### ").next())
        .expect("docs/cli.md has an `ekr process-map` section");
    let example = section
        .split("\n```json\n")
        .nth(1)
        .and_then(|rest| rest.split("\n```\n").next())
        .expect("the section has a json example");
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let printed = world.run(&["process-map"]);
        assert_eq!(printed.status.code(), Some(0), "{backend}");
        assert_eq!(
            String::from_utf8(printed.stdout).unwrap(),
            format!("{example}\n"),
            "{backend}"
        );
    }
}

#[test]
fn a_session_serves_process_map_as_the_one_shot_verb_prints_it() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let answers = world.session(&[
            json!({"argv": ["process-map"]}),
            json!({"argv": ["process-map", "--events", "Reviewed"]}),
            json!({"argv": ["process-map", "--events", "Nope"]}),
        ]);
        assert_eq!(answers.len(), 3, "{backend}");
        for (answer, one_shot) in answers.iter().zip([
            world.ok(&["process-map"]),
            world.ok(&["process-map", "--events", "Reviewed"]),
        ]) {
            assert_eq!(answer["exit"], 0, "{backend}: {answer}");
            assert_eq!(answer["stdout"], one_shot, "{backend}");
        }
        assert_eq!(answers[2]["exit"], 2, "{backend}");
        assert!(
            answers[2]["stderr"]
                .as_str()
                .unwrap()
                .contains("ekr.views.EventTypeNotFound"),
            "{backend}: {}",
            answers[2]
        );
    }
}
