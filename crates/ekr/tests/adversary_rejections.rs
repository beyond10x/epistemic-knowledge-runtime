//! Adversary cases for `story:validation-findings-read` (`ekr rejections`, `ekr.rejections/1`).
//!
//! Each case drives fresh binary processes on both providers and states the contract it holds
//! from `docs/cli.md` and `systems/ekr/domains/kernel.yaml` (`ekr.kernel.RejectionsV1`), not from
//! the implementation.

use std::path::PathBuf;
use std::process::Output;

use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];

const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const ORGANISATION: &str = "00000000-0000-4000-8000-000000000202";

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

    fn bytes(&self, verb: &[&str]) -> Vec<u8> {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: stderr {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
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

    fn propose(&self, id: &str, operations: &str) {
        let document = format!(
            "format: ekr.transaction-document/2\ntransaction:\n  id: {id}\n  proposer: \
             {OPERATOR}\n  operations:\n{operations}  evidence: []\n"
        );
        let path = self.file(&format!("{id}.yaml"), &document);
        assert_eq!(self.ok(&["propose", &path])["transaction_id"], id);
    }

    fn validate(&self, id: &str, against: u64) -> Value {
        self.ok(&["validate", id, "--against", &against.to_string()])
    }

    /// Proposes `id` with one alias for a node no revision holds, and rejects it against
    /// `against`; returns the issues `ekr validate` printed.
    fn reject(&self, id: &str, against: u64) -> Value {
        self.propose(
            id,
            &format!(
                "  - !AddAlias\n    node: 00000000-0000-4000-8000-00000000d398\n    alias: \
                 alias-{id}\n"
            ),
        );
        let printed = self.validate(id, against);
        assert_eq!(printed["kind"], "Rejected", "{printed}");
        printed["issues"].clone()
    }

    fn node(node: &str) -> String {
        format!(
            "  - !CreateNode\n    id: {node}\n    root_id: {ROOT}\n    type_id: \
             {ORGANISATION}\n    canonical_name: Node {node}\n    properties: {{}}\n"
        )
    }

    fn commit_node(&self, id: &str, node: &str) {
        self.propose(id, &Self::node(node));
        let head = self.ok(&["head"])["revision"].as_u64().unwrap();
        assert_eq!(self.validate(id, head)["kind"], "Validated");
        assert_eq!(self.ok(&["commit", id])["kind"], "Committed");
    }
}

fn listed(document: &Value) -> Vec<(String, u64)> {
    document["rejections"]
        .as_array()
        .unwrap_or_else(|| panic!("no rejections list: {document}"))
        .iter()
        .map(|entry| {
            (
                entry["transaction_id"].as_str().unwrap().to_owned(),
                entry["against"].as_u64().unwrap(),
            )
        })
        .collect()
}

/// `docs/cli.md`: "Entries are ordered by `against`, then `transaction_id`". The unit's own
/// fixtures plant rejections whose id order and basis order agree, so a selection that did not
/// sort by `against` at all (`rejections.rs:235` removed) stays green there. Here the basis order
/// and the id order disagree, and two rejections tie on one basis in reverse proposal order.
#[test]
fn entries_order_by_basis_first_and_ties_by_transaction_id_whatever_the_id_order() {
    const LATE_LOW: &str = "00000000-0000-4000-8000-00000000e001";
    const EARLY_MID: &str = "00000000-0000-4000-8000-00000000e005";
    const EARLY_HIGH: &str = "00000000-0000-4000-8000-00000000e009";
    for backend in BACKENDS {
        let world = World::seeded(backend);
        world.reject(EARLY_HIGH, 0);
        world.reject(EARLY_MID, 0);
        world.commit_node(
            "00000000-0000-4000-8000-00000000e0c1",
            "00000000-0000-4000-8000-00000000e301",
        );
        world.reject(LATE_LOW, 1);
        let all = world.ok(&["rejections"]);
        assert_eq!(
            listed(&all),
            [
                (EARLY_MID.to_owned(), 0),
                (EARLY_HIGH.to_owned(), 0),
                (LATE_LOW.to_owned(), 1)
            ],
            "{backend}: {all}"
        );
    }
}

/// kernel.yaml `ekr.kernel.RejectionsV1`: a Proposed, Validated or Stale transaction never
/// appears; the unit's tests hold only the Committed case.
#[test]
fn open_validated_and_stale_transactions_never_appear() {
    const OPEN: &str = "00000000-0000-4000-8000-00000000e101";
    const VALIDATED: &str = "00000000-0000-4000-8000-00000000e102";
    const WINNER: &str = "00000000-0000-4000-8000-00000000e103";
    const STALE: &str = "00000000-0000-4000-8000-00000000e104";
    const REJECTED: &str = "00000000-0000-4000-8000-00000000e105";
    for backend in BACKENDS {
        let world = World::seeded(backend);
        world.propose(OPEN, &World::node("00000000-0000-4000-8000-00000000e311"));
        world.propose(
            VALIDATED,
            &World::node("00000000-0000-4000-8000-00000000e312"),
        );
        assert_eq!(world.validate(VALIDATED, 0)["kind"], "Validated");
        world.propose(WINNER, &World::node("00000000-0000-4000-8000-00000000e313"));
        world.propose(STALE, &World::node("00000000-0000-4000-8000-00000000e314"));
        assert_eq!(world.validate(WINNER, 0)["kind"], "Validated");
        assert_eq!(world.validate(STALE, 0)["kind"], "Validated");
        assert_eq!(world.ok(&["commit", WINNER])["kind"], "Committed");
        let _ = world.run(&["commit", STALE]);
        let stale = world.ok(&["transactions", "--state", "Stale"]);
        assert_eq!(stale[0]["transaction_id"], STALE, "{backend}: {stale}");
        world.reject(REJECTED, 0);

        for range in [&[][..], &["--from", "0", "--to", "0"], &["--from", "0"]] {
            let mut verb = vec!["rejections"];
            verb.extend_from_slice(range);
            let document = world.ok(&verb);
            assert_eq!(
                listed(&document),
                [(REJECTED.to_owned(), 0)],
                "{backend} {range:?}: {document}"
            );
        }
    }
}

/// A rejection validated against a revision the head had already moved past is keyed on that
/// basis, not on the head at the time of the rejection nor on the head at the time of the read.
#[test]
fn a_rejection_against_a_superseded_revision_is_keyed_on_that_basis() {
    const BEHIND: &str = "00000000-0000-4000-8000-00000000e201";
    for backend in BACKENDS {
        let world = World::seeded(backend);
        world.commit_node(
            "00000000-0000-4000-8000-00000000e2c1",
            "00000000-0000-4000-8000-00000000e321",
        );
        world.commit_node(
            "00000000-0000-4000-8000-00000000e2c2",
            "00000000-0000-4000-8000-00000000e322",
        );
        let issues = world.reject(BEHIND, 0);
        assert_eq!(world.ok(&["head"])["revision"], 2, "{backend}");
        let all = world.ok(&["rejections"]);
        assert_eq!(listed(&all), [(BEHIND.to_owned(), 0)], "{backend}: {all}");
        assert_eq!(all["rejections"][0]["issues"], issues, "{backend}");
        assert!(
            listed(&world.ok(&["rejections", "--from", "1"])).is_empty(),
            "{backend}"
        );
        assert!(
            listed(&world.ok(&["rejections", "--from", "2", "--to", "2"])).is_empty(),
            "{backend}"
        );
        assert_eq!(
            listed(&world.ok(&["rejections", "--to", "0"])).len(),
            1,
            "{backend}"
        );
    }
}

/// Many issues on one transaction, from more than one validator, are listed exactly as
/// `ekr validate` printed them, in their recorded order.
#[test]
fn many_issues_from_several_validators_are_listed_as_validate_printed_them() {
    const MANY: &str = "00000000-0000-4000-8000-00000000e401";
    for backend in BACKENDS {
        let world = World::seeded(backend);
        world.propose(
            MANY,
            "  - !AddAlias\n    node: 00000000-0000-4000-8000-00000000e398\n    alias: same\n  \
             - !AddAlias\n    node: 00000000-0000-4000-8000-00000000e399\n    alias: same\n  \
             - !RetractAssertion\n    assertion: 00000000-0000-4000-8000-00000000e599\n    \
             reason: none\n",
        );
        let printed = world.validate(MANY, 0);
        assert_eq!(printed["kind"], "Rejected", "{printed}");
        let issues = printed["issues"].as_array().unwrap();
        assert!(issues.len() >= 3, "{backend}: {printed}");
        let listed = world.ok(&["rejections", "--from", "0", "--to", "0"]);
        assert_eq!(
            listed["rejections"][0]["issues"], printed["issues"],
            "{backend}: {listed}"
        );
        let rejected_at = listed["rejections"][0]["rejected_at"].clone();
        let transactions = world.ok(&["transactions"]);
        let submitted = transactions
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["transaction_id"] == MANY)
            .unwrap()["submitted_at"]
            .clone();
        assert!(
            rejected_at.as_i64().unwrap() >= submitted.as_i64().unwrap(),
            "{backend}: rejected_at {rejected_at} precedes submission {submitted}"
        );
    }
}

/// Bounds: a signed or negative bound, a bound past u64 and a repeated bound are usage errors
/// (exit 2); a bound past the head selects nothing and echoes as given; only one bound echoes
/// only that bound.
#[test]
fn range_bounds_at_the_edges() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        world.reject("00000000-0000-4000-8000-00000000e501", 0);
        for bad in [
            &["--from", "-1"][..],
            &["--to", "-1"],
            &["--to", "18446744073709551616"],
            &["--from", "0", "--from", "1"],
            &["--from", "1.0"],
            &["--from", ""],
        ] {
            let mut verb = vec!["rejections"];
            verb.extend_from_slice(bad);
            let output = world.run(&verb);
            assert_eq!(output.status.code(), Some(2), "{backend} {bad:?}");
            assert!(output.stdout.is_empty(), "{backend} {bad:?}");
        }
        let far = world.ok(&["rejections", "--from", "7", "--to", "18446744073709551615"]);
        assert_eq!(
            far,
            serde_json::json!({"format": "ekr.rejections/1", "from": 7,
                "to": 18_446_744_073_709_551_615_u64, "rejections": []}),
            "{backend}"
        );
        let upside = world.ok(&["rejections", "--from", "1", "--to", "0"]);
        assert_eq!(
            upside,
            serde_json::json!({"format": "ekr.rejections/1", "from": 1, "to": 0,
                "rejections": []}),
            "{backend}: from above to selects nothing and is not refused"
        );
        let only_to = world.ok(&["rejections", "--to", "0"]);
        assert!(only_to.get("from").is_none(), "{backend}: {only_to}");
        assert_eq!(only_to["to"], 0, "{backend}: {only_to}");
        assert_eq!(listed(&only_to).len(), 1, "{backend}: {only_to}");
    }
}

/// kernel.yaml `ekr.kernel.RejectionsV1`: "Every object's keys ascend by their text". Read from
/// the printed bytes, not from a parsed map that would sort them anyway.
#[test]
fn printed_keys_ascend_by_their_text() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        world.reject("00000000-0000-4000-8000-00000000e601", 0);
        let printed =
            String::from_utf8(world.bytes(&["rejections", "--from", "0", "--to", "5"])).unwrap();
        let at = |key: &str| {
            printed
                .find(&format!("\"{key}\":"))
                .unwrap_or_else(|| panic!("{key} missing: {printed}"))
        };
        let top = ["format", "from", "rejections", "to"];
        let entry = ["against", "issues", "proposer", "rejected_at"];
        for pair in top.windows(2).chain(entry.windows(2)) {
            assert!(
                at(pair[0]) < at(pair[1]),
                "{backend}: {pair:?} in {printed}"
            );
        }
        assert!(printed.ends_with("}\n"), "{backend}: one document, newline");
    }
}

/// A session follows a store replaced at its path: the second read answers from the new store,
/// never the replaced one's rejections.
#[test]
fn a_session_answers_rejections_from_a_store_replaced_at_its_path() {
    for backend in BACKENDS {
        let world = World::seeded(backend);
        world.reject("00000000-0000-4000-8000-00000000e701", 0);
        let other = World::seeded(backend);

        let mut child = ekr()
            .args(world.args(&["session"]))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());
        let mut ask = |line: &str| -> Value {
            std::io::Write::write_all(&mut stdin, format!("{line}\n").as_bytes()).unwrap();
            std::io::Write::flush(&mut stdin).unwrap();
            let mut answer = String::new();
            std::io::BufRead::read_line(&mut stdout, &mut answer).unwrap();
            serde_json::from_str(&answer).unwrap()
        };
        let first = ask(r#"{"argv": ["rejections"]}"#);
        assert_eq!(first["exit"], 0, "{backend}: {first}");
        assert_eq!(listed(&first["stdout"]).len(), 1, "{backend}: {first}");

        let aside = world.directory.path().join("aside");
        std::fs::rename(world.store(), &aside).unwrap();
        std::fs::rename(other.store(), world.store()).unwrap();
        let second = ask(r#"{"argv": ["rejections"]}"#);
        assert_eq!(second["exit"], 0, "{backend}: {second}");
        assert!(
            listed(&second["stdout"]).is_empty(),
            "{backend}: the replaced store's rejections: {second}"
        );
        drop(stdin);
        assert_eq!(child.wait().unwrap().code(), Some(0), "{backend}");
    }
}

/// Both committed suites are synthesized from the one contract at `systems/ekr`, so they carry
/// one `spec_digest` and one `contract_digest`; at the unit's base they did. The unit changed
/// `kernel.yaml` and regenerated `suite.json` only, so `views-suite.json` still names the old
/// contract and `task conform-fresh` (a step of `task check`) fails its `cmp`.
#[test]
fn both_committed_suites_name_the_one_contract_they_were_synthesized_from() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let conformance = PathBuf::from(manifest)
        .ancestors()
        .nth(2)
        .expect("crates/ekr sits two levels below the workspace root")
        .join("systems/ekr/conformance");
    let provenance = |name: &str| -> Value {
        let bytes = std::fs::read(conformance.join(name)).unwrap();
        serde_json::from_slice::<Value>(&bytes).unwrap()["provenance"].clone()
    };
    let kernel = provenance("suite.json");
    let views = provenance("views-suite.json");
    for field in ["spec_digest", "contract_digest"] {
        assert!(kernel[field].is_string(), "suite.json has no {field}");
        assert_eq!(
            views[field], kernel[field],
            "views-suite.json {field} is not the contract suite.json was synthesized from"
        );
    }
}
