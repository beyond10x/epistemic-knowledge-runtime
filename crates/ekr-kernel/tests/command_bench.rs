//! Command cost against history length, on both providers. Not part of the default test run.
//!
//! `story:eventlog-0-4-batched-reads`: on the file provider a command's cost grew with the square
//! of the revisions, because every provider call re-hashed the whole log and a command made one
//! call per object it read. This harness builds one history per provider up to revision 40 and
//! times, at revisions 1, 20 and 40, the commands that produce that revision — `propose`,
//! `validate`, `commit` — and a `head` read of it. Each timed command opens the runtime afresh,
//! as one invocation of the `ekr` binary does, so the open is part of what is measured.
//!
//! It is compiled only with the `bench` feature (`[[test]] required-features`), so neither
//! `cargo test` nor the gate runs it. Run it with the release profile:
//!
//! ```text
//! cargo test --release --locked -p ekr-kernel --features bench --test command_bench -- --nocapture
//! ```
//!
//! Each provider's history is built `ROUNDS` times from nothing. It prints every round and then
//! the fastest of them per (provider, revision) and command, and asserts on those fastest rows
//! that growth is linear (the story's coordinator decision on the bench bound): for `propose`,
//! `validate` and `commit` on both providers, the increase from revision 20 to 40 is at most
//! twice the increase from revision 1 to 20. Linear cost adds about as much over the second
//! twenty revisions as over the first nineteen; the quadratic cost this story removed added about
//! three times as much. `head` is printed but not bounded: at 1–30 ms its steps are within timer
//! and scheduling noise. Timings are wall-clock and only ever grow with the machine's load.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::time::{Duration, Instant};

const MEASURED: [u64; 3] = [1, 20, 40];

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}
fn anchor() -> AuthorityStateV1 {
    let c = context();
    AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [(c.operator, "operator"), (c.validator, "validator")]
            .into_iter()
            .map(|(id, name)| {
                (
                    id,
                    Agent {
                        id,
                        name: name.into(),
                        capabilities: BTreeSet::new(),
                    },
                )
            })
            .collect(),
        validation_profile: ValidationProfileV1::deterministic(c.validator),
    }
}
fn open(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, "bench", context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), "bench", context(), anchor())
    }
    .unwrap()
}
fn seed() -> SeedDocument {
    let mut seed = SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let type_id = "00000000-0000-4000-8000-000000000005".parse().unwrap();
    let label = "00000000-0000-4000-8000-000000000006".parse().unwrap();
    let mut declared = NodeType::new(type_id, "Subject");
    declared.properties.insert(
        label,
        PropertyDefinition::new(label, "label", ValueType::String),
    );
    seed.ontology.node_types.push(declared);
    let bytes = b"synthetic human evidence".to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let evidence = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::from_basis_points(10000).unwrap(),
    };
    seed.graph.evidence.insert(evidence.id, evidence);
    seed.evidence_payloads.insert(hash, bytes);
    seed
}
/// One new node with one evidenced assertion, so every revision adds records and objects.
fn document(seed: &SeedDocument, n: u64) -> (TransactionId, Vec<u8>) {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let ty = &seed.ontology.node_types[0];
    let label = *ty.properties.keys().next().unwrap();
    let evidence = *seed.graph.evidence.keys().next().unwrap();
    let id = NodeId::mint();
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![
            GraphOperation::CreateNode(NodeDraft {
                id,
                root_id: seed.graph.root.id,
                type_id: ty.id,
                canonical_name: format!("subject {n}"),
                properties: BTreeMap::new(),
            }),
            GraphOperation::AddAssertion(Box::new(Assertion {
                id: AssertionId::mint(),
                root_id: seed.graph.root.id,
                subject: Subject::Node(id),
                predicate: Predicate::Property(label),
                object: Object::Value(Value::String(format!("label {n}"))),
                evidence: BTreeSet::from([evidence]),
                proposed_by: context().operator,
                assessment: Assessment::Proposed,
                lifecycle: AssertionLifecycle::Active,
                valid_time: TemporalRange::UNBOUNDED,
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            })),
        ],
        evidence: BTreeSet::from([evidence]),
        schema_version: None,
    };
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: &tx,
    })
    .unwrap()
    .into_bytes();
    (tx.id, bytes)
}

/// Wall-clock time of `work` against a freshly opened runtime, open included.
fn timed<T>(path: &Path, file: bool, work: impl FnOnce(&Runtime) -> T) -> (Duration, T) {
    let started = Instant::now();
    let kernel = open(path, file);
    let result = work(&kernel);
    drop(kernel);
    (started.elapsed(), result)
}

#[derive(Debug)]
struct Row {
    provider: &'static str,
    revision: u64,
    propose: Duration,
    validate: Duration,
    commit: Duration,
    head: Duration,
}

fn measure(file: bool) -> Vec<Row> {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    let seed = seed();
    open(path, file)
        .seed(seed.clone(), || Timestamp::from_millis(10))
        .unwrap();
    let mut rows = Vec::new();
    for revision in 1..=*MEASURED.last().unwrap() {
        let at = i64::try_from(revision * 100).unwrap();
        let (tx, bytes) = document(&seed, revision);
        let (propose, _) = timed(path, file, |k| {
            k.propose(&bytes, context().operator, || Timestamp::from_millis(at))
                .unwrap()
        });
        let (validate, validated) = timed(path, file, |k| {
            k.validate(tx, RevisionNumber::new(revision - 1), || {
                Timestamp::from_millis(at + 1)
            })
            .unwrap()
        });
        assert!(
            matches!(validated, ValidationCommandResult::Validated(_)),
            "revision {revision} did not validate: {validated:?}"
        );
        let (commit, committed) = timed(path, file, |k| {
            k.commit(tx, context().operator, || Timestamp::from_millis(at + 2))
                .unwrap()
        });
        let CommitCommandResult::Committed(receipt) = committed else {
            panic!("revision {revision} did not commit: {committed:?}");
        };
        assert_eq!(receipt.result.revision, RevisionNumber::new(revision));
        let (head, root) = timed(path, file, |k| k.head().unwrap());
        assert_eq!(root, Some(receipt.result));
        if MEASURED.contains(&revision) {
            rows.push(Row {
                provider: if file { "file" } else { "sqlite" },
                revision,
                propose,
                validate,
                commit,
                head,
            });
        }
    }
    rows
}

/// One command's cell of a row.
type Pick = fn(&Row) -> Duration;

/// Independent histories built per provider; each cell reports the fastest of them.
const ROUNDS: usize = 3;

/// The fastest of `ROUNDS` rows for each (provider, revision), cell by cell. The minimum, because
/// a shared machine only ever adds time to a run, never removes it.
fn fastest(rounds: &[Vec<Row>]) -> Vec<Row> {
    let min = |pick: fn(&Row) -> Duration, at: usize| {
        rounds.iter().map(|rows| pick(&rows[at])).min().unwrap()
    };
    (0..rounds[0].len())
        .map(|at| Row {
            provider: rounds[0][at].provider,
            revision: rounds[0][at].revision,
            propose: min(|row| row.propose, at),
            validate: min(|row| row.validate, at),
            commit: min(|row| row.commit, at),
            head: min(|row| row.head, at),
        })
        .collect()
}

fn print(title: &str, rows: &[Row]) {
    println!("{title}");
    println!("provider | revision | propose ms | validate ms | commit ms | head ms");
    for row in rows {
        println!(
            "{} | {} | {} | {} | {} | {}",
            row.provider,
            row.revision,
            row.propose.as_millis(),
            row.validate.as_millis(),
            row.commit.as_millis(),
            row.head.as_millis()
        );
    }
}

#[test]
fn command_cost_against_history_length_on_both_providers() {
    let mut rows = Vec::new();
    for file in [false, true] {
        let rounds: Vec<Vec<Row>> = (0..ROUNDS).map(|_| measure(file)).collect();
        for (round, measured) in rounds.iter().enumerate() {
            print(&format!("round {round}"), measured);
        }
        rows.extend(fastest(&rounds));
    }
    print(&format!("fastest of {ROUNDS} rounds"), &rows);
    let mut quadratic = Vec::new();
    for provider in ["sqlite", "file"] {
        let at = |revision| {
            rows.iter()
                .find(|row| row.provider == provider && row.revision == revision)
                .unwrap()
        };
        let cells: [(&str, Pick); 3] = [
            ("propose", |row| row.propose),
            ("validate", |row| row.validate),
            ("commit", |row| row.commit),
        ];
        for (command, pick) in cells {
            let [first, middle, last] = MEASURED.map(|revision| pick(at(revision)));
            let (early, late) = (middle.saturating_sub(first), last.saturating_sub(middle));
            if late > early * 2 {
                quadratic.push(format!(
                    "{provider} {command}: {} -> {} -> {} ms, so revision 20 -> 40 added {} ms, \
                     more than twice the {} ms revision 1 -> 20 added",
                    first.as_millis(),
                    middle.as_millis(),
                    last.as_millis(),
                    late.as_millis(),
                    early.as_millis()
                ));
            }
        }
    }
    assert!(
        quadratic.is_empty(),
        "command cost grows faster than linearly in the revisions:\n  {}",
        quadratic.join("\n  ")
    );
}
