//! A `validate` command against a revision the session holds builds one candidate view, and a
//! session builds the per-edge assertion index of each revision it holds once
//! (`task:validate-builds-one-view-per-command`), on both providers and under every profile.
//! Against a revision whose graph the session has released, the command first rebuilds that graph
//! by replaying the history, which revalidates every retained validation up to it
//! (`adversary_validate_once.rs`, ignored for `task:rebuilt-revision-reuses-retained-verdicts`).
//!
//! The command decides the publication by validating the proposal, and the store then admits the
//! staged candidate by replaying it through the same kernel, which reaches the same verdict from
//! the same inputs. That replay takes the verdict the decision reached instead of running the
//! pipeline again, whatever the verdict. The counts are
//! [`ekr_kernel::validate::candidates_built`] and [`ekr_kernel::validate::edge_indexes_built`]:
//! every candidate view, and every index of canonical state's assertions about edges, built on
//! this thread.
//!
//! That the verdicts and roots are the ones the pipeline reached before the change is
//! `adversary_perf_01_validation_scans.rs`'s differential and the replay cases.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const TENANT: &str = "validate-once";

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}
fn anchor(profile: ValidationProfileV1) -> AuthorityStateV1 {
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
        validation_profile: profile,
    }
}
fn profiles() -> [(&'static str, AuthorityStateV1); 3] {
    let v = context().validator;
    [
        ("v1", anchor(ValidationProfileV1::deterministic(v))),
        ("v2", anchor(ValidationProfileV1::schema_evolving(v))),
        ("v3", anchor(ValidationProfileV1::identity_keeping(v))),
    ]
}
fn open(path: &Path, file: bool, authority: &AuthorityStateV1) -> Runtime {
    if file {
        Runtime::file(path, TENANT, context(), authority.clone())
    } else {
        Runtime::sqlite(&path.join("state.db"), TENANT, context(), authority.clone())
    }
    .unwrap()
}

/// One node type, `Subject` with a `label`, and one retained evidence entry.
struct World {
    document: SeedDocument,
    subject: TypeId,
    label: PropertyId,
    evidence: EvidenceId,
}
fn world() -> World {
    let mut document =
        SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let (subject, label) = (TypeId::mint(), PropertyId::mint());
    let mut declared = NodeType::new(subject, "Subject");
    declared.properties.insert(
        label,
        PropertyDefinition::new(label, "label", ValueType::String),
    );
    document.ontology.node_types.push(declared);
    let bytes = b"seeded statement".to_vec();
    let entry = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: ContentHash::of_bytes(&bytes),
        extracted_by: context().operator,
        observed_at: Timestamp::from_millis(5),
        confidence: Confidence::CERTAIN,
    };
    let evidence = entry.id;
    document.graph.evidence.insert(evidence, entry.clone());
    document
        .evidence_payloads
        .insert(entry.content_hash, bytes.into());
    World {
        document,
        subject,
        label,
        evidence,
    }
}
impl World {
    /// A new subject with an evidenced label: every profile accepts it.
    fn accepted(&self) -> GraphTransaction {
        let node = NodeId::mint();
        let root_id = self.document.graph.root.id;
        GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            operations: vec![
                GraphOperation::CreateNode(NodeDraft {
                    id: node,
                    root_id,
                    type_id: self.subject,
                    canonical_name: format!("subject {node}"),
                    properties: BTreeMap::new(),
                    aliases: Vec::new(),
                }),
                GraphOperation::AddAssertion(Box::new(Assertion {
                    id: AssertionId::mint(),
                    root_id,
                    subject: Subject::Node(node),
                    predicate: Predicate::Property(self.label),
                    object: Object::Value(Value::String("a label".into())),
                    evidence: BTreeSet::from([self.evidence]),
                    proposed_by: context().operator,
                    assessment: Assessment::Proposed,
                    lifecycle: AssertionLifecycle::Active,
                    valid_time: TemporalRange::UNBOUNDED,
                    transaction_time: TransactionTime::since(Timestamp::EPOCH),
                })),
            ],
            evidence: BTreeSet::from([self.evidence]),
            schema_version: None,
        }
    }
    /// A label change on a node nothing holds: every profile refuses it.
    fn refused(&self) -> GraphTransaction {
        GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            operations: vec![GraphOperation::UpdateProperty(PropertyMutation {
                node: NodeId::mint(),
                property: self.label,
                values: vec![Value::String("relabelled".into())],
            })],
            evidence: BTreeSet::new(),
            schema_version: None,
        }
    }
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
/// A monotonic clock for one case.
struct Clock(std::cell::Cell<i64>);
impl Clock {
    fn new() -> Self {
        Self(std::cell::Cell::new(1_000))
    }
    fn tick(&self) -> Timestamp {
        let now = self.0.get() + 1;
        self.0.set(now);
        Timestamp::from_millis(now)
    }
}
/// A seeded session.
fn session(
    path: &Path,
    file: bool,
    authority: &AuthorityStateV1,
    w: &World,
    clock: &Clock,
) -> Runtime {
    let runtime = open(path, file, authority);
    runtime.seed(w.document.clone(), || clock.tick()).unwrap();
    runtime
}
/// Validates `tx`, already proposed, against the head: the verdict, with the candidate views and
/// edge indexes the command alone built.
fn validate(
    runtime: &Runtime,
    clock: &Clock,
    tx: &GraphTransaction,
) -> (ValidationCommandResult, u64, u64) {
    let against = runtime.head().unwrap().unwrap().revision;
    let (views, indexes) = (
        ekr_kernel::validate::candidates_built(),
        ekr_kernel::validate::edge_indexes_built(),
    );
    let verdict = runtime.validate(tx.id, against, || clock.tick()).unwrap();
    (
        verdict,
        ekr_kernel::validate::candidates_built() - views,
        ekr_kernel::validate::edge_indexes_built() - indexes,
    )
}

/// Accepted or refused, a validate command builds one candidate view, under every profile and on
/// both providers.
#[test]
fn a_validate_command_builds_one_candidate_view_whatever_the_verdict() {
    for (profile, authority) in profiles() {
        for file in [true, false] {
            let directory = tempfile::tempdir().unwrap();
            let clock = Clock::new();
            let w = world();
            let runtime = session(directory.path(), file, &authority, &w, &clock);
            for (tx, accepts) in [(w.accepted(), true), (w.refused(), false)] {
                runtime
                    .propose(&encode(&tx), context().operator, || clock.tick())
                    .unwrap();
                let (verdict, views, _) = validate(&runtime, &clock, &tx);
                assert_eq!(
                    matches!(verdict, ValidationCommandResult::Validated(_)),
                    accepts,
                    "{profile} file={file}: {verdict:?}"
                );
                assert_eq!(views, 1, "{profile} file={file} accepts={accepts}");
            }
        }
    }
}

/// Three validations against one revision in a session build that revision's per-edge index once,
/// and the first validation against the revision a commit makes builds that revision's once.
#[test]
fn a_session_builds_each_revisions_edge_index_once() {
    for (profile, authority) in profiles() {
        for file in [true, false] {
            let at = format!("{profile} file={file}");
            let directory = tempfile::tempdir().unwrap();
            let clock = Clock::new();
            let w = world();
            let runtime = session(directory.path(), file, &authority, &w, &clock);
            let proposals = [w.accepted(), w.refused(), w.accepted()];
            for tx in &proposals {
                runtime
                    .propose(&encode(tx), context().operator, || clock.tick())
                    .unwrap();
            }
            let built: Vec<u64> = proposals
                .iter()
                .map(|tx| validate(&runtime, &clock, tx).2)
                .collect();
            assert_eq!(built, [1, 0, 0], "{at}: against the seed");

            let committed = runtime
                .commit(proposals[0].id, context().operator, || clock.tick())
                .unwrap();
            assert!(
                matches!(committed, CommitCommandResult::Committed(_)),
                "{at}: {committed:?}"
            );
            let next = [w.accepted(), w.refused()];
            for tx in &next {
                runtime
                    .propose(&encode(tx), context().operator, || clock.tick())
                    .unwrap();
            }
            let built: Vec<u64> = next
                .iter()
                .map(|tx| validate(&runtime, &clock, tx).2)
                .collect();
            assert_eq!(built, [1, 0], "{at}: against revision 1");
        }
    }
}
