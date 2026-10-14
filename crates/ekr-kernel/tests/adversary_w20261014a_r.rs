//! Adversary pass, wave 2026-10-14a unit R (`story:reads-served-from-a-persisted-read-model`), at
//! `264fbbe16`.
//!
//! The unit's CHANGELOG entry says one-shot `ekr snapshot` and `ekr ontology` answer "byte for
//! byte what a complete read prints", and names one behaviour change only: "a retained record
//! damaged after the checkpoint pointer that covers it was written is no longer refused". Design
//! § 108.2 says the same of records. An evidence payload is not a record: it is the retained
//! admissible evidence invariant 2 rests canonical knowledge on, and a complete read
//! (`Runtime::read`, through `RevisionLog::history`) requires every evidence payload of the
//! lineage and refuses one whose bytes do not hash to their address. These cases hold the read
//! model to that refusal, on both providers: a read the model serves must not answer where the
//! complete read it replaces refuses, unless the change says it does.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, ValueType};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const TENANT: &str = "adversary-read-model";

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
/// A writing handle, as `ekr propose`, `validate` and `commit` open one.
fn open(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, TENANT, context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), TENANT, context(), anchor())
    }
    .unwrap()
}
/// A reading handle, as `ekr snapshot` and `ekr ontology` open one.
fn reader(path: &Path, file: bool) -> Runtime {
    opened(path, file).unwrap()
}
/// [`reader`], or the provider's refusal to open the store. The file provider refuses to open a
/// store whose referenced blob fails its own integrity check: every read of it is refused.
fn opened(path: &Path, file: bool) -> Result<Runtime, String> {
    if file {
        Runtime::file_reading(path, TENANT, context(), anchor())
    } else {
        Runtime::sqlite_reading(&path.join("state.db"), TENANT, context(), anchor())
    }
    .map_err(|error| format!("open refused: {error}"))
}
fn provider(file: bool) -> &'static str {
    if file {
        "file"
    } else {
        "sqlite"
    }
}

/// The seed evidence payload: bytes that occur nowhere else in a store, so that damaging them
/// damages that one object.
const SEED_PAYLOAD: &[u8] = b"adversary w20261014a-r seed evidence payload, unique bytes 7f3a";
/// The payload a committed `AddEvidence` brings.
const ADDED_PAYLOAD: &[u8] = b"adversary w20261014a-r added evidence payload, unique bytes 91c4";

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
    let hash = ContentHash::of_bytes(SEED_PAYLOAD);
    let evidence = Evidence {
        id: "00000000-0000-4000-8000-000000000007".parse().unwrap(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::from_basis_points(10000).unwrap(),
    };
    seed.graph.evidence.insert(evidence.id, evidence);
    seed.evidence_payloads
        .insert(hash, SEED_PAYLOAD.to_vec().into());
    seed
}

fn encode(tx: &GraphTransaction) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/2",
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}
/// One new node with one assertion resting on the seed's evidence.
fn node(seed: &SeedDocument, n: u64) -> GraphTransaction {
    let root = seed.graph.root.id;
    let ty = &seed.ontology.node_types[0];
    let label = *ty.properties.keys().next().unwrap();
    let evidence = *seed.graph.evidence.keys().next().unwrap();
    let id = NodeId::mint();
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![
            GraphOperation::CreateNode(NodeDraft {
                id,
                root_id: root,
                type_id: ty.id,
                canonical_name: format!("subject {n}"),
                properties: BTreeMap::new(),
                aliases: Vec::new(),
            }),
            GraphOperation::AddAssertion(Box::new(Assertion {
                id: AssertionId::mint(),
                root_id: root,
                subject: Subject::Node(id),
                predicate: Predicate::Property(label),
                object: Object::Value(ekr_ontology::Value::String(format!("label {n}"))),
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
    }
}
/// One `AddEvidence` of [`ADDED_PAYLOAD`].
fn added_evidence() -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::AddEvidence(Box::new(EvidenceAddition {
            evidence: Evidence {
                id: EvidenceId::mint(),
                source: EvidenceSource::HumanStatement {
                    identity: Some("operator".into()),
                },
                content_hash: ContentHash::of_bytes(ADDED_PAYLOAD),
                extracted_by: context().operator,
                observed_at: Timestamp::from_millis(5),
                confidence: Confidence::CERTAIN,
            },
            payload: ADDED_PAYLOAD.to_vec(),
        }))],
        evidence: BTreeSet::new(),
        schema_version: None,
    }
}
/// Proposes, validates and commits `tx` as revision `n`, each command through a fresh writing
/// handle, as one-shot `ekr` invocations do.
fn commit(path: &Path, file: bool, n: u64, tx: &GraphTransaction) {
    let at = i64::try_from(n * 100).unwrap();
    open(path, file)
        .propose(&encode(tx), context().operator, || {
            Timestamp::from_millis(at)
        })
        .unwrap();
    let verdict = open(path, file)
        .validate(tx.id, RevisionNumber::new(n - 1), || {
            Timestamp::from_millis(at + 1)
        })
        .unwrap();
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    let result = open(path, file)
        .commit(tx.id, context().operator, || Timestamp::from_millis(at + 2))
        .unwrap();
    assert!(
        matches!(result, CommitCommandResult::Committed(_)),
        "{result:?}"
    );
}
/// A store whose fifth commit writes a replay checkpoint at the head (design § 99), the first
/// commit bringing [`ADDED_PAYLOAD`] when `add` says so.
fn build(path: &Path, file: bool, add: bool) {
    let seed = seed();
    open(path, file)
        .seed(seed.clone(), || Timestamp::from_millis(10))
        .unwrap();
    for n in 1..=5 {
        let tx = if add && n == 1 {
            added_evidence()
        } else {
            node(&seed, n)
        };
        commit(path, file, n, &tx);
    }
}

/// Flips one bit in the middle of every occurrence of `bytes` in every file under `root`: the
/// same length, different bytes. Returns how many occurrences it changed.
fn damage(root: &Path, bytes: &[u8]) -> usize {
    fn walk(path: &Path, bytes: &[u8], damaged: &mut usize) {
        if path.is_dir() {
            for entry in std::fs::read_dir(path).unwrap() {
                walk(&entry.unwrap().path(), bytes, damaged);
            }
            return;
        }
        let mut contents = std::fs::read(path).unwrap();
        let mut at = 0;
        let mut changed = false;
        while let Some(found) = contents[at..]
            .windows(bytes.len())
            .position(|window| window == bytes)
        {
            contents[at + found + bytes.len() / 2] ^= 1;
            at += found + bytes.len();
            changed = true;
            *damaged += 1;
        }
        if changed {
            std::fs::write(path, contents).unwrap();
        }
    }
    let mut damaged = 0;
    walk(root, bytes, &mut damaged);
    damaged
}

/// Builds the store, checks that the model serves its head before any damage, damages `payload`
/// and holds every one-shot read of the head to what a complete read of the same store answers.
fn held_to_the_complete_read(add: bool, payload: &[u8], what: &str) {
    for file in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path();
        build(path, file, add);
        assert!(
            reader(path, file)
                .read_graph(None)
                .unwrap()
                .served_from_model(),
            "{}: precondition: the undamaged head is served from the model",
            provider(file)
        );
        let damaged = damage(path, payload);
        assert!(
            damaged >= 1,
            "{}: precondition: the {what} is retained as raw bytes",
            provider(file)
        );
        // What `ekr snapshot` and `ekr ontology` read before this change: a complete read.
        let complete = opened(path, file)
            .and_then(|runtime| runtime.read(None).map(|_| ()).map_err(|e| e.to_string()));
        let full = opened(path, file).and_then(|mut runtime| {
            runtime.set_full_replay(true);
            runtime.read(None).map(|_| ()).map_err(|e| e.to_string())
        });
        assert!(
            complete.is_err() && full.is_err(),
            "{}: precondition: a complete read refuses a damaged {what} ({damaged} \
             occurrence(s) damaged): complete {complete:?}, full replay {full:?}",
            provider(file)
        );
        let graph = opened(path, file)
            .and_then(|runtime| runtime.read_graph(None).map_err(|e| e.to_string()));
        let ontology = opened(path, file)
            .and_then(|runtime| runtime.read_ontology(None).map_err(|e| e.to_string()));
        assert!(
            graph.is_err(),
            "{}: `ekr snapshot` (Runtime::read_graph) answers over a damaged {what} that the \
             complete read it replaces refuses ({}); served from the model: {:?}",
            provider(file),
            complete.unwrap_err(),
            graph.as_ref().map(GraphRead::served_from_model).ok()
        );
        assert!(
            ontology.is_err(),
            "{}: `ekr ontology` (Runtime::read_ontology) answers over a damaged {what} that the \
             complete read it replaces refuses; served from the model: {:?}",
            provider(file),
            ontology.as_ref().map(OntologyRead::served_from_model).ok()
        );
    }
}

/// The seed's evidence payload, damaged on disk after the checkpoint at the head was written.
/// A complete read refuses the store; the CHANGELOG names no evidence payload among what a
/// read served from the model stops refusing.
#[test]
fn a_damaged_seed_evidence_payload_is_refused_by_snapshot_and_ontology_as_by_a_complete_read() {
    held_to_the_complete_read(false, SEED_PAYLOAD, "seed evidence payload");
}

/// The payload a committed `AddEvidence` brought, damaged on disk after the checkpoint at the
/// head was written: the evidence an assertion of the served graph may rest on.
#[test]
fn a_damaged_added_evidence_payload_is_refused_by_snapshot_and_ontology_as_by_a_complete_read() {
    held_to_the_complete_read(true, ADDED_PAYLOAD, "added evidence payload");
}
