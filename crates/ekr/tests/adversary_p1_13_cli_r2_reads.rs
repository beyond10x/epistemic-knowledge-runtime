//! Adversary pass 2 on `story:ekr-cli`: the round-2/3 read verbs and the `--valid-at` correction,
//! through fresh `ekr` processes on both providers.
//!
//! Every process is checked for output discipline: a success writes exactly one JSON document on
//! stdout and nothing on stderr; any failure writes nothing on stdout.

use std::path::{Path, PathBuf};
use std::process::Output;

use ekr::host::CliHostConfigurationV1;
use ekr_core::{AssertionId, RevisionNumber, Timestamp, TransactionId};
use ekr_kernel::{Runtime, TransactionState};
use serde_json::Value;

const MARCH_12: i64 = 1_773_273_600_000;
const ALICE: &str = "00000000-0000-4000-8000-000000000501";
const BOB: &str = "00000000-0000-4000-8000-000000000502";
const E_ALICE: &str = "00000000-0000-4000-8000-000000000401";
const E_BOB: &str = "00000000-0000-4000-8000-000000000402";
const T_ALICE: &str = "00000000-0000-4000-8000-000000000601";
const T_BOB: &str = "00000000-0000-4000-8000-000000000602";
const T_REJECTED: &str = "00000000-0000-4000-8000-000000000603";
const T_STALE: &str = "00000000-0000-4000-8000-000000000604";
const T_BOB_ADD: &str = "00000000-0000-4000-8000-000000000611";
const T_SUPERSEDE: &str = "00000000-0000-4000-8000-000000000613";
const BACKENDS: [&str; 2] = ["file", "sqlite"];

/// The kernel carrier as the CLI prints it (`ekr guide`, OUTPUT): each `document_bytes` number
/// array becomes one standard padded base64 string; every other field is compared unchanged.
fn document_bytes_as_base64(value: &mut Value) {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    match value {
        Value::Object(map) => {
            for (key, item) in map.iter_mut() {
                if key == "document_bytes" {
                    let bytes: Vec<u8> = item
                        .as_array()
                        .expect("document_bytes is a byte array in the kernel carrier")
                        .iter()
                        .map(|b| u8::try_from(b.as_u64().unwrap()).unwrap())
                        .collect();
                    let mut text = String::new();
                    for chunk in bytes.chunks(3) {
                        let mut triple = [0_u8; 3];
                        triple[..chunk.len()].copy_from_slice(chunk);
                        let n = u32::from_be_bytes([0, triple[0], triple[1], triple[2]]);
                        for position in 0..4 {
                            text.push(if position <= chunk.len() {
                                char::from(ALPHABET[((n >> (18 - 6 * position)) & 0x3f) as usize])
                            } else {
                                '='
                            });
                        }
                    }
                    *item = Value::String(text);
                } else {
                    document_bytes_as_base64(item);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(document_bytes_as_base64),
        _ => {}
    }
}

fn fixture(name: &str) -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time");
    Path::new(&manifest)
        .join("tests/fixtures/retraction")
        .join(name)
}

fn fixture_text(name: &str) -> String {
    std::fs::read_to_string(fixture(name)).unwrap()
}

struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn new(backend: &'static str) -> Self {
        Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        }
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    /// Writes a generated document next to the store and returns its path argument.
    fn document(&self, name: &str, text: &str) -> String {
        let path = self.directory.path().join(name);
        std::fs::write(&path, text).unwrap();
        path.display().to_string()
    }

    fn run(&self, verb: &[&str]) -> Output {
        if self.backend == "file" {
            std::fs::create_dir_all(self.store()).unwrap();
        }
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"))
            .arg("--host")
            .arg(fixture("host.json"))
            .arg("--store")
            .arg(self.store())
            .arg("--backend")
            .arg(self.backend)
            .args(verb)
            .stdin(std::process::Stdio::null())
            .output()
            .unwrap();
        disciplined(self.backend, verb, &output);
        output
    }

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

    fn code(&self, verb: &[&str]) -> Option<i32> {
        self.run(verb).status.code()
    }

    fn runtime(&self) -> Runtime {
        let host = CliHostConfigurationV1::from_json(&std::fs::read(fixture("host.json")).unwrap())
            .unwrap();
        match self.backend {
            "file" => Runtime::file(&self.store(), &host.tenant, host.context, host.authority),
            _ => Runtime::sqlite(&self.store(), &host.tenant, host.context, host.authority),
        }
        .unwrap()
    }

    fn states(&self) -> Vec<(TransactionId, TransactionState)> {
        self.runtime()
            .transactions()
            .unwrap()
            .into_iter()
            .map(|(id, record)| (id, record.state()))
            .collect()
    }

    fn kernel_snapshot(&self, at: Option<u64>, valid_at: Option<i64>) -> Value {
        let read = self.runtime().read(at.map(RevisionNumber::new)).unwrap();
        serde_json::to_value(read.snapshot(valid_at.map(Timestamp::from_millis)).unwrap()).unwrap()
    }

    fn kernel_explain(&self, id: &str) -> Value {
        let id: AssertionId = id.parse().unwrap();
        let read = self.runtime().read(None).unwrap();
        let mut value = serde_json::to_value(read.explain(id).unwrap()).unwrap();
        document_bytes_as_base64(&mut value);
        value
    }

    fn step(&self, document: &str, transaction: &str, against: u64) -> Value {
        self.ok(&["propose", document]);
        let validated = self.ok(&["validate", transaction, "--against", &against.to_string()]);
        assert_eq!(
            validated["kind"], "Validated",
            "{}: {validated}",
            self.backend
        );
        self.ok(&["commit", transaction])
    }
}

/// A success is exactly one JSON document and a newline on stdout with a silent stderr; any other
/// exit writes nothing on stdout and says why on stderr.
fn disciplined(backend: &str, verb: &[&str], output: &Output) {
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8");
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.code() == Some(0) {
        assert!(
            stderr.is_empty(),
            "{backend} {verb:?}: a success wrote stderr {stderr}"
        );
        assert!(
            stdout.ends_with("}\n") && !stdout.ends_with("\n\n"),
            "{backend} {verb:?}: not one newline-terminated document: {stdout:?}"
        );
        let mut documents = serde_json::Deserializer::from_str(&stdout).into_iter::<Value>();
        assert!(
            documents.next().is_some_and(|d| d.is_ok()),
            "{backend} {verb:?}"
        );
        assert!(
            documents.next().is_none(),
            "{backend} {verb:?}: two documents"
        );
    } else {
        assert!(
            stdout.is_empty(),
            "{backend} {verb:?}: exit {:?} wrote a result {stdout}",
            output.status.code()
        );
        assert!(
            !stderr.trim().is_empty(),
            "{backend} {verb:?}: silent failure"
        );
    }
}

fn matching(snapshot: &Value) -> Vec<&str> {
    snapshot["matching_assertions"]
        .as_array()
        .unwrap_or_else(|| panic!("no selection in {snapshot}"))
        .iter()
        .map(|id| id.as_str().unwrap())
        .collect()
}

fn kinds(explanation: &Value) -> Vec<&str> {
    explanation["links"]
        .as_array()
        .unwrap()
        .iter()
        .map(|link| link["kind"].as_str().unwrap())
        .collect()
}

fn evidence(explanation: &Value) -> Vec<&str> {
    explanation["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|link| link["kind"] == "Evidence")
        .map(|link| link["id"].as_str().unwrap())
        .collect()
}

/// The retraction seed with Alice's assertion already in the seeded graph.
fn seed_with_alice() -> String {
    let seed = fixture_text("seed.yaml");
    let alice = "    assertions:
      00000000-0000-4000-8000-000000000501:
        id: 00000000-0000-4000-8000-000000000501
        root_id: 00000000-0000-4000-8000-000000000002
        subject: !Node 00000000-0000-4000-8000-000000000301
        predicate: !Relation 00000000-0000-4000-8000-000000000203
        object: !Node 00000000-0000-4000-8000-000000000303
        evidence:
        - 00000000-0000-4000-8000-000000000401
        proposed_by: 00000000-0000-4000-8000-000000000101
        assessment: Proposed
        lifecycle: Active
        valid_time:
          from: 1577836800000
          to: null
        transaction_time:
          recorded_from: 0
          recorded_to: null
";
    assert!(seed.contains("    assertions: {}\n"));
    seed.replace("    assertions: {}\n", alice)
}

/// Bob's addition alone, without the supersession, as its own transaction.
fn bob_added_alone() -> String {
    let bob = fixture_text("propose-bob.yaml");
    let start = bob.find("  - !SupersedeAssertion").unwrap();
    let end = start + bob[start..].find("\n  evidence:\n").unwrap() + 1;
    format!("{}{}", &bob[..start], &bob[end..]).replace(T_BOB, T_BOB_ADD)
}

/// The supersession alone, of Alice by an already accepted Bob.
fn supersession_alone() -> String {
    format!(
        "format: ekr.transaction-document/1
transaction:
  id: {T_SUPERSEDE}
  proposer: 00000000-0000-4000-8000-000000000101
  operations:
  - !SupersedeAssertion
    assertion: {ALICE}
    by: {BOB}
    effective_from: {MARCH_12}
  evidence: []
"
    )
}

#[test]
fn a_negative_valid_at_takes_its_own_value_and_never_a_following_flag() {
    for backend in BACKENDS {
        let world = World::new(backend);
        world.ok(&["seed", &fixture("seed.yaml").display().to_string()]);

        // The negative value binds to --valid-at and --at still parses after it.
        let both = world.ok(&["snapshot", "--valid-at", "-1", "--at", "0"]);
        assert_eq!(both["valid_at"], -1, "{backend}");
        assert_eq!(both["root"]["revision"], 0, "{backend}");
        assert_eq!(both, world.kernel_snapshot(Some(0), Some(-1)), "{backend}");
        let equals = world.ok(&["snapshot", "--at", "0", &format!("--valid-at={}", i64::MIN)]);
        let separated = world.ok(&["snapshot", "--valid-at", &i64::MIN.to_string(), "--at", "0"]);
        assert_eq!(equals, separated, "{backend}");

        // A following flag is never swallowed as the value, and --at stays unsigned.
        for verb in [
            &["snapshot", "--valid-at", "--at", "0"][..],
            &["snapshot", "--at", "0", "--valid-at"],
            &["snapshot", "--at", "-1"],
            &["snapshot", "-1"],
            &["snapshot", "--valid-at", "-1", "--valid-at", "-2"],
            &["explain", "-1"],
        ] {
            assert_eq!(world.code(verb), Some(2), "{backend} {verb:?}");
        }
        // Number-shaped non-canonical spellings reach the selector parser and are refused there.
        for value in [
            "-0",
            "+5",
            "-1e3",
            "-1.5",
            "-9223372036854775809",
            "007",
            "-inf",
        ] {
            let output = world.run(&["snapshot", "--valid-at", value]);
            assert_eq!(output.status.code(), Some(2), "{backend} {value}");
        }
    }
}

#[test]
fn snapshot_is_the_kernel_carrier_at_every_revision_and_selector_and_section_65_holds() {
    for backend in BACKENDS {
        let world = World::new(backend);
        world.ok(&["seed", &fixture("seed.yaml").display().to_string()]);
        world.step(
            &fixture("propose-alice.yaml").display().to_string(),
            T_ALICE,
            0,
        );
        world.step(&fixture("propose-bob.yaml").display().to_string(), T_BOB, 1);

        for at in [None, Some(0), Some(1), Some(2)] {
            for valid_at in [None, Some(MARCH_12 - 1), Some(MARCH_12), Some(i64::MAX)] {
                let mut verb = vec!["snapshot".to_owned()];
                if let Some(at) = at {
                    verb.extend(["--at".to_owned(), at.to_string()]);
                }
                if let Some(valid_at) = valid_at {
                    verb.extend(["--valid-at".to_owned(), valid_at.to_string()]);
                }
                let verb: Vec<&str> = verb.iter().map(String::as_str).collect();
                assert_eq!(
                    world.ok(&verb),
                    world.kernel_snapshot(at, valid_at),
                    "{backend} {verb:?}"
                );
            }
        }
        assert_eq!(
            world.ok(&["snapshot", "--at", "2"]),
            world.ok(&["snapshot"])
        );

        // § 65: Alice's relationship is kept and closed at 2026-03-12; Bob's opens there.
        let latest = world.ok(&["snapshot"]);
        let assertions = &latest["graph"]["graph"]["assertions"];
        assert_eq!(assertions[ALICE]["valid_time"]["to"], MARCH_12, "{backend}");
        assert_eq!(assertions[BOB]["valid_time"]["from"], MARCH_12, "{backend}");
        let historical = world.ok(&["snapshot", "--at", "1"]);
        assert!(historical["graph"]["graph"]["assertions"][ALICE]["valid_time"]["to"].is_null());
        assert!(historical["graph"]["graph"]["assertions"][BOB].is_null());
        assert_eq!(
            matching(&world.ok(&["snapshot", "--valid-at", &i64::MAX.to_string()])),
            [BOB],
            "{backend}: the far future is Bob's alone"
        );
        for huge in [u64::MAX, 1 << 63] {
            let output = world.run(&["snapshot", "--at", &huge.to_string()]);
            assert_eq!(output.status.code(), Some(2), "{backend} {huge}");
            assert!(String::from_utf8_lossy(&output.stderr).contains("ekr.kernel.RevisionNotFound"));
        }
    }
}

#[test]
fn explain_a_seed_only_assertion_superseded_by_a_later_proposal() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let seed = world.document("seed-with-alice.yaml", &seed_with_alice());
        let seeded = world.ok(&["seed", &seed]);
        let commit = world.step(&fixture("propose-bob.yaml").display().to_string(), T_BOB, 0);
        assert_eq!(commit["result"]["revision"], 1);

        let alice = world.ok(&["explain", ALICE]);
        assert_eq!(alice, world.kernel_explain(ALICE), "{backend}");
        assert_eq!(alice["at"], 1);
        assert_eq!(
            kinds(&alice),
            [
                "Assertion",
                "Seed",
                "Lifecycle",
                "Assertion",
                "Proposal",
                "Validation",
                "Commit",
                "Evidence",
                "Evidence"
            ],
            "{backend}: {alice}"
        );
        let links = alice["links"].as_array().unwrap();
        // The seed origin is the original retained result and the actual host identities.
        assert_eq!(links[1]["result"], seeded, "{backend}");
        let host: Value =
            serde_json::from_slice(&std::fs::read(fixture("host.json")).unwrap()).unwrap();
        assert_eq!(links[1]["context"], host["context"], "{backend}");
        assert_eq!(
            links[1]["validation_profile"], host["authority"]["validation_profile"],
            "{backend}"
        );
        assert_eq!(links[2]["receipt"]["event_id"], commit["event_id"]);
        assert_eq!(links[4]["transaction_id"], T_BOB);
        assert_eq!(evidence(&alice), [E_ALICE, E_BOB], "{backend}");

        let bob = world.ok(&["explain", BOB]);
        assert_eq!(bob, world.kernel_explain(BOB), "{backend}");
        assert_eq!(
            kinds(&bob),
            ["Assertion", "Proposal", "Validation", "Commit", "Evidence"]
        );
        assert_eq!(
            evidence(&bob),
            [E_BOB],
            "{backend}: the unused Alice payload is not Bob's"
        );

        assert_eq!(
            matching(&world.ok(&["snapshot", "--valid-at", "2026-03-11"])),
            [ALICE],
            "{backend}: a seeded assertion is believed before the supersession"
        );
        assert_eq!(
            matching(&world.ok(&["snapshot", "--valid-at", "2026-03-12"])),
            [BOB],
            "{backend}"
        );
    }
}

#[test]
fn explain_a_replacement_accepted_before_its_supersession() {
    for backend in BACKENDS {
        let world = World::new(backend);
        world.ok(&["seed", &fixture("seed.yaml").display().to_string()]);
        world.step(
            &fixture("propose-alice.yaml").display().to_string(),
            T_ALICE,
            0,
        );
        let add = world.document("add-bob.yaml", &bob_added_alone());
        let added = world.step(&add, T_BOB_ADD, 1);
        // Both are believed at 2026-03-12 until the supersession commits.
        assert_eq!(
            matching(&world.ok(&["snapshot", "--valid-at", "2026-03-12"])),
            [ALICE, BOB],
            "{backend}"
        );
        let supersede = world.document("supersede.yaml", &supersession_alone());
        let superseded = world.step(&supersede, T_SUPERSEDE, 2);

        let alice = world.ok(&["explain", ALICE]);
        assert_eq!(alice, world.kernel_explain(ALICE), "{backend}");
        assert_eq!(alice["at"], 3);
        assert_eq!(
            kinds(&alice),
            [
                "Assertion",
                "Proposal",
                "Validation",
                "Commit",
                "Lifecycle",
                "Assertion",
                "Proposal",
                "Validation",
                "Commit",
                "Evidence",
                "Evidence"
            ],
            "{backend}: {alice}"
        );
        let links = alice["links"].as_array().unwrap();
        assert_eq!(links[1]["transaction_id"], T_ALICE);
        assert_eq!(links[4]["receipt"]["event_id"], superseded["event_id"]);
        assert_eq!(links[5]["id"], BOB);
        assert_eq!(
            links[6]["transaction_id"], T_BOB_ADD,
            "{backend}: Bob's own origin"
        );
        assert_eq!(links[8]["event_id"], added["event_id"]);
        assert_eq!(evidence(&alice), [E_ALICE, E_BOB], "{backend}");

        assert_eq!(
            matching(&world.ok(&["snapshot", "--valid-at", "2026-03-12"])),
            [BOB],
            "{backend}"
        );
        assert_eq!(
            matching(&world.ok(&["snapshot", "--at", "2", "--valid-at", "2026-03-12"])),
            [ALICE, BOB],
            "{backend}: history at revision 2"
        );
    }
}

#[test]
fn every_outcome_refusal_and_fault_writes_one_result_or_nothing() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let seed = fixture("seed.yaml").display().to_string();
        // Faults before any seed.
        for verb in [&["snapshot"][..], &["explain", ALICE], &["commit", T_ALICE]] {
            assert_eq!(world.code(verb), Some(1), "{backend} {verb:?}");
        }
        world.ok(&["seed", &seed]);
        world.ok(&["seed", &seed]);
        world.step(
            &fixture("propose-alice.yaml").display().to_string(),
            T_ALICE,
            0,
        );
        world.ok(&[
            "propose",
            &fixture("propose-rejected.yaml").display().to_string(),
        ]);
        let rejected = world.ok(&["validate", T_REJECTED, "--against", "1"]);
        assert_eq!(rejected["kind"], "Rejected");
        world.ok(&[
            "propose",
            &fixture("propose-bob.yaml").display().to_string(),
        ]);
        world.ok(&[
            "propose",
            &fixture("propose-stale.yaml").display().to_string(),
        ]);
        world.ok(&["validate", T_BOB, "--against", "1"]);
        world.ok(&["validate", T_STALE, "--against", "1"]);
        world.ok(&["commit", T_BOB]);
        assert_eq!(world.ok(&["commit", T_STALE])["kind"], "Stale");
        world.ok(&["commit", T_BOB]);
        world.ok(&["snapshot", "--valid-at", "2026-03-12"]);
        world.ok(&["explain", ALICE]);
        // Refusals and usage errors.
        for verb in [
            &[
                "seed",
                &fixture("seed-different.yaml").display().to_string(),
            ][..],
            &["commit", T_REJECTED],
            &["commit", T_STALE],
            &["snapshot", "--at", "3"],
            &["explain", "00000000-0000-4000-8000-000000000599"],
            &["explain", "not-an-id"],
            &["snapshot", "--valid-at", "2026-02-30"],
        ] {
            assert_eq!(world.code(verb), Some(2), "{backend} {verb:?}");
        }
    }
}

#[test]
fn a_document_that_cannot_be_read_is_a_fault_and_records_nothing() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let directory = world.directory.path().display().to_string();
        assert_eq!(
            world.code(&["seed", &directory]),
            Some(1),
            "{backend}: seed"
        );
        world.ok(&["seed", &fixture("seed.yaml").display().to_string()]);
        let before = world.states();
        let output = world.run(&["propose", &directory]);
        assert_eq!(output.status.code(), Some(1), "{backend}: propose");
        assert!(!String::from_utf8_lossy(&output.stderr).contains("StructurallyInvalid"));
        assert_eq!(
            world.states(),
            before,
            "{backend}: an unreadable proposal recorded"
        );
    }
}
