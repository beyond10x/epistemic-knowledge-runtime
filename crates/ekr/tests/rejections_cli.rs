//! `story:validation-findings-read`: `ekr rejections` returns, for a range of basis revisions,
//! each rejected transaction with the issues its retained rejection record holds, keyed on the
//! revision it was validated against, as one deterministic `ekr.rejections/1` document.
//!
//! Every case drives fresh binary processes on both providers. The issues a rejection must list
//! are the ones `ekr validate` printed when it recorded that rejection, so the read cannot drift
//! from what the kernel retained.

use std::path::PathBuf;
use std::process::Output;

use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];

/// The example host's operator, the proposer every document names.
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
/// The example seed's graph root and its second node type.
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const ORGANISATION: &str = "00000000-0000-4000-8000-000000000202";

/// Rejected against revision 0: aliases for two nodes no revision holds, so two issues.
const EARLY: &str = "00000000-0000-4000-8000-00000000f101";
/// Rejected against revision 1: a retraction of an assertion no revision holds.
const LATE: &str = "00000000-0000-4000-8000-00000000f102";
/// Committed as revision 1.
const KEPT: &str = "00000000-0000-4000-8000-00000000f103";
/// Committed as revision 2, after the reads it must not change.
const UNRELATED: &str = "00000000-0000-4000-8000-00000000f104";

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// One provider in its own directory, seeded from the printed examples.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
    host: PathBuf,
}

impl World {
    fn seeded(backend: &'static str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let host = directory.path().join("host.json");
        std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
        let world = Self {
            directory,
            backend,
            host,
        };
        let seed = world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        world.ok(&["seed", &seed]);
        world
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            self.host.display().to_string(),
            "--store".to_owned(),
            self.store().display().to_string(),
            "--backend".to_owned(),
            self.backend.to_owned(),
        ];
        args.extend(verb.iter().map(|a| (*a).to_owned()));
        args
    }

    fn run(&self, verb: &[&str]) -> Output {
        ekr().args(self.args(verb)).output().unwrap()
    }

    /// Exit 0, nothing on stderr, and the exact stdout bytes.
    fn bytes(&self, verb: &[&str]) -> Vec<u8> {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: stderr {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty(), "{verb:?}: a success wrote stderr");
        output.stdout
    }

    fn ok(&self, verb: &[&str]) -> Value {
        serde_json::from_slice(&self.bytes(verb)).unwrap()
    }

    fn file(&self, name: &str, contents: &str) -> String {
        let path = self.directory.path().join(name);
        std::fs::write(&path, contents).unwrap();
        path.display().to_string()
    }

    /// Proposes `operations` as transaction `id`.
    fn propose(&self, id: &str, operations: &str) {
        let document = format!(
            "format: ekr.transaction-document/2\ntransaction:\n  id: {id}\n  proposer: \
             {OPERATOR}\n  operations:\n{operations}  evidence: []\n"
        );
        let path = self.file(&format!("{id}.yaml"), &document);
        assert_eq!(self.ok(&["propose", &path])["transaction_id"], id);
    }

    /// Validates `id` against `against` and returns the printed outcome.
    fn validate(&self, id: &str, against: u64) -> Value {
        self.ok(&["validate", id, "--against", &against.to_string()])
    }

    /// Proposes, validates and commits a new organisation node, as transaction `id`.
    fn commit_node(&self, id: &str, node: &str) {
        self.propose(
            id,
            &format!(
                "  - !CreateNode\n    id: {node}\n    root_id: {ROOT}\n    type_id: \
                 {ORGANISATION}\n    canonical_name: Node {node}\n    properties: {{}}\n"
            ),
        );
        let head = self.ok(&["head"])["revision"].as_u64().unwrap();
        assert_eq!(self.validate(id, head)["kind"], "Validated");
        assert_eq!(self.ok(&["commit", id])["kind"], "Committed");
    }
}

/// Plants the two rejections and the committed transaction; returns the issues `ekr validate`
/// printed for the early and the late rejection.
fn planted(world: &World) -> (Value, Value) {
    world.propose(
        EARLY,
        "  - !AddAlias\n    node: 00000000-0000-4000-8000-00000000f398\n    alias: first-unknown\n  \
         - !AddAlias\n    node: 00000000-0000-4000-8000-00000000f399\n    alias: second-unknown\n",
    );
    let early = world.validate(EARLY, 0);
    assert_eq!(early["kind"], "Rejected", "{early}");
    world.commit_node(KEPT, "00000000-0000-4000-8000-00000000f301");
    world.propose(
        LATE,
        "  - !RetractAssertion\n    assertion: 00000000-0000-4000-8000-00000000f599\n    reason: \
         no such assertion\n",
    );
    let late = world.validate(LATE, 1);
    assert_eq!(late["kind"], "Rejected", "{late}");
    assert_eq!(early["issues"].as_array().unwrap().len(), 2, "{early}");
    assert_eq!(late["issues"].as_array().unwrap().len(), 1, "{late}");
    (early["issues"].clone(), late["issues"].clone())
}

/// The transaction ids a document lists, in order.
fn listed(document: &Value) -> Vec<String> {
    document["rejections"]
        .as_array()
        .unwrap_or_else(|| panic!("no rejections list: {document}"))
        .iter()
        .map(|entry| entry["transaction_id"].as_str().unwrap().to_owned())
        .collect()
}

/// Acceptance 1 and 2: every planted rejection is listed with each of its issues, keyed on the
/// revision it was validated against, and the committed transaction never appears.
#[test]
fn every_planted_rejection_is_listed_with_its_issues_by_basis_and_no_committed_one() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        assert_eq!(
            world.ok(&["rejections"]),
            serde_json::json!({"format": "ekr.rejections/1", "rejections": []}),
            "{backend}: a store with no rejection"
        );
        let (early, late) = planted(&world);

        let all = world.ok(&["rejections"]);
        assert_eq!(all["format"], "ekr.rejections/1", "{backend}: {all}");
        assert!(
            all.get("from").is_none() && all.get("to").is_none(),
            "{backend}: an absent bound is omitted, never null: {all}"
        );
        assert_eq!(listed(&all), [EARLY, LATE], "{backend}: {all}");
        let entries = all["rejections"].as_array().unwrap();
        assert_eq!(entries[0]["against"], 0, "{backend}: {all}");
        assert_eq!(entries[0]["issues"], early, "{backend}: {all}");
        assert_eq!(entries[0]["proposer"], OPERATOR, "{backend}: {all}");
        assert_eq!(entries[1]["against"], 1, "{backend}: {all}");
        assert_eq!(entries[1]["issues"], late, "{backend}: {all}");
        for entry in entries {
            assert!(entry["rejected_at"].is_i64(), "{backend}: {entry}");
            let mut keys: Vec<&str> = entry
                .as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect();
            keys.sort_unstable();
            assert_eq!(
                keys,
                [
                    "against",
                    "issues",
                    "proposer",
                    "rejected_at",
                    "transaction_id"
                ],
                "{backend}: the declared fields and no other"
            );
        }

        let first = world.ok(&["rejections", "--from", "0", "--to", "0"]);
        assert_eq!(first["from"], 0, "{backend}: {first}");
        assert_eq!(first["to"], 0, "{backend}: {first}");
        assert_eq!(listed(&first), [EARLY], "{backend}: {first}");
        assert_eq!(first["rejections"][0]["issues"], early, "{backend}");
        let second = world.ok(&["rejections", "--from", "1"]);
        assert_eq!(listed(&second), [LATE], "{backend}: {second}");
        assert!(second.get("to").is_none(), "{backend}: {second}");
        assert_eq!(
            listed(&world.ok(&["rejections", "--to", "0"])),
            [EARLY],
            "{backend}"
        );
        assert!(
            listed(&world.ok(&["rejections", "--from", "2"])).is_empty(),
            "{backend}"
        );
        assert!(
            listed(&world.ok(&["rejections", "--from", "1", "--to", "0"])).is_empty(),
            "{backend}: from above to selects nothing"
        );

        let committed = world.ok(&["transactions", "--state", "Committed"]);
        assert_eq!(
            committed[0]["transaction_id"], KEPT,
            "{backend}: {committed}"
        );
        for range in [&[][..], &["--from", "0", "--to", "0"], &["--from", "1"]] {
            let mut verb = vec!["rejections"];
            verb.extend_from_slice(range);
            assert!(
                !listed(&world.ok(&verb)).iter().any(|id| id == KEPT),
                "{backend}: the committed transaction appears in {range:?}"
            );
        }
        assert_eq!(
            world.run(&["rejections", "--from", "zero"]).status.code(),
            Some(2),
            "{backend}: a bound that is no revision number is a usage error"
        );
    }
}

/// Acceptance 3: two reads of one range are byte-identical, before and after an unrelated commit.
#[test]
fn two_reads_of_one_range_are_byte_identical_before_and_after_an_unrelated_commit() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        planted(&world);
        let ranges: [&[&str]; 3] = [&[], &["--from", "0", "--to", "1"], &["--from", "1"]];
        let read = |range: &[&str]| {
            let mut verb = vec!["rejections"];
            verb.extend_from_slice(range);
            world.bytes(&verb)
        };
        let before: Vec<Vec<u8>> = ranges.iter().map(|range| read(range)).collect();
        for (range, bytes) in ranges.iter().zip(&before) {
            assert_eq!(&read(range), bytes, "{backend} {range:?}: two reads differ");
        }
        world.commit_node(UNRELATED, "00000000-0000-4000-8000-00000000f302");
        assert_eq!(world.ok(&["head"])["revision"], 2, "{backend}");
        for (range, bytes) in ranges.iter().zip(&before) {
            assert_eq!(
                &read(range),
                bytes,
                "{backend} {range:?}: an unrelated commit changed the answer"
            );
        }
        let mut full = world.args(&["rejections"]);
        full.insert(0, "--full-replay".to_owned());
        let output = ekr().args(&full).output().unwrap();
        assert_eq!(
            output.stdout, before[0],
            "{backend}: a full replay answers the same bytes"
        );
    }
}

/// `ekr session` answers the one-shot verb's exact document.
#[test]
fn a_session_answers_the_one_shot_document() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        planted(&world);
        let requests = [
            serde_json::json!({"argv": ["rejections"]}),
            serde_json::json!({"argv": ["rejections", "--from", "1", "--to", "1"]}),
        ];
        let mut input = String::new();
        for request in &requests {
            input.push_str(&request.to_string());
            input.push('\n');
        }
        let mut child = ekr()
            .args(world.args(&["session"]))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        std::io::Write::write_all(child.stdin.as_mut().unwrap(), input.as_bytes()).unwrap();
        drop(child.stdin.take());
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.code(), Some(0), "{backend}");
        let answers: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(answers.len(), 2, "{backend}");
        for (answer, verb) in answers.iter().zip([
            &["rejections"][..],
            &["rejections", "--from", "1", "--to", "1"],
        ]) {
            assert_eq!(answer["exit"], 0, "{backend}: {answer}");
            let mut printed = serde_json::to_string_pretty(&answer["stdout"]).unwrap();
            printed.push('\n');
            assert_eq!(printed.as_bytes(), world.bytes(verb), "{backend} {verb:?}");
        }
    }
}
