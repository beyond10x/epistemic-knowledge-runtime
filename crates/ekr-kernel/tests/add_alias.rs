//! `story:node-gains-an-alias`: a transaction adds an alias to a node that already exists, on both
//! providers, under every validation profile.
//!
//! An import keys the nodes it brings in by an alias, so that a later import finds them with
//! `ekr resolve`. A node another path created earlier, without that alias, is found and linked but
//! could never gain the key: aliases were set only by `CreateNode`. `!AddAlias {node, alias}` gives
//! it one; the per-head alias index sees it at the next head, and replay reproduces every root. The
//! refusals are held, message for message, by `tests/validation.rs`.

use std::collections::BTreeSet;

use ekr_core::{ContentHash, EvidenceId, NodeId, RevisionNumber, Timestamp, TransactionId, TypeId};
use ekr_graph::{CanonicalGraph, Node, Root};
use ekr_kernel::{
    Agent, AliasAddition, AuthorityStateV1, BootstrapContext, CommitCommandResult, GraphOperation,
    GraphTransaction, Runtime, SeedDocument, ValidationCommandResult, ValidationProfileV1,
    ValidatorName,
};
use ekr_ontology::{NodeType, Value};
use serde::Serialize;

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

/// The three validation profiles: an alias is data, not a schema change, so each applies it.
fn profiles() -> [(&'static str, AuthorityStateV1); 3] {
    let validator = context().validator;
    [
        ("v1", anchor(ValidationProfileV1::deterministic(validator))),
        (
            "v2",
            anchor(ValidationProfileV1::schema_evolving(validator)),
        ),
        (
            "v3",
            anchor(ValidationProfileV1::identity_keeping(validator)),
        ),
    ]
}

fn open(path: &std::path::Path, file: bool, authority: AuthorityStateV1) -> Runtime {
    if file {
        Runtime::file(path, "test", context(), authority)
    } else {
        Runtime::sqlite(&path.join("state.db"), "test", context(), authority)
    }
    .unwrap()
}

/// A `Repository` type and a `Team` type; two repositories created without an alias, and a team
/// holding the alias the first repository is about to gain, which does not identify a repository.
struct Imported {
    document: SeedDocument,
    repository: TypeId,
    first: NodeId,
    second: NodeId,
}

const KEY: &str = "source-a-repos:4711";

fn imported() -> Imported {
    let mut document =
        SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let (repository, team) = (TypeId::mint(), TypeId::mint());
    document.ontology.node_types.extend([
        NodeType::new(repository, "Repository"),
        NodeType::new(team, "Team"),
    ]);
    let root = document.graph.root.id;
    let mut node = |type_id: TypeId, name: &str, aliases: &[&str]| {
        let mut node = Node::<Value>::new(NodeId::mint(), root, type_id, name);
        node.aliases = aliases.iter().map(|alias| (*alias).to_owned()).collect();
        let id = node.id;
        document.graph.nodes.insert(id, node);
        id
    };
    let first = node(repository, "runtime", &[]);
    let second = node(repository, "website", &[]);
    node(team, "platform", &[KEY]);
    Imported {
        document,
        repository,
        first,
        second,
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

fn transaction(operations: Vec<GraphOperation>) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: BTreeSet::<EvidenceId>::new(),
        schema_version: None,
    }
}

fn add_alias(node: NodeId, alias: &str) -> GraphOperation {
    GraphOperation::AddAlias(AliasAddition {
        node,
        alias: alias.to_owned(),
    })
}

/// Propose, validate against the head, and — if validated — commit, at `at`, `at + 1`, `at + 2`.
fn submit(kernel: &Runtime, tx: &GraphTransaction, at: i64) -> ValidationCommandResult {
    kernel
        .propose(&encode(tx), context().operator, || {
            Timestamp::from_millis(at)
        })
        .unwrap();
    let head = kernel.head().unwrap().unwrap().revision;
    let verdict = kernel
        .validate(tx.id, head, || Timestamp::from_millis(at + 1))
        .unwrap();
    if matches!(verdict, ValidationCommandResult::Validated(_)) {
        let committed = kernel
            .commit(tx.id, context().operator, || Timestamp::from_millis(at + 2))
            .unwrap();
        assert!(
            matches!(committed, CommitCommandResult::Committed(_)),
            "{committed:?}"
        );
    }
    verdict
}

/// Each issue of a rejection as `(validator, code)`.
fn refused(verdict: &ValidationCommandResult) -> Vec<(ValidatorName, String)> {
    match verdict {
        ValidationCommandResult::Rejected(record) => record
            .issues
            .iter()
            .map(|issue| (issue.validator, issue.code.clone()))
            .collect(),
        ValidationCommandResult::Validated(_) => panic!("expected a rejection"),
    }
}

/// Every revision's graph as this runtime replays it, with the root its commit receipt holds.
fn every_revision(kernel: &Runtime) -> Vec<(Option<Root>, CanonicalGraph)> {
    let head = kernel.head().unwrap().unwrap();
    let records = kernel.transactions().unwrap();
    (0..=head.revision.get())
        .map(|number| {
            let graph = kernel.replay(RevisionNumber::new(number)).unwrap();
            let root = records
                .values()
                .filter_map(|record| record.committed.as_ref())
                .find(|receipt| receipt.result.revision == graph.revision)
                .map(|receipt| receipt.result);
            if let Some(root) = root {
                assert_eq!(root.ontology_root, ContentHash::of(&graph.ontology));
            }
            (root, graph)
        })
        .collect()
}

/// Acceptance 1 and 3: a repository created without an alias gains the import's key in a later
/// transaction; the head's alias index answers that repository for it, and the index of the
/// revision before does not; the team holding the same text is another type and is no obstacle;
/// and a reopened store replays every root, with the alias.
#[test]
fn a_node_created_without_an_alias_gains_one_the_next_head_resolves_and_replay_reproduces() {
    for (profile, authority) in profiles() {
        for file in [false, true] {
            let at = format!("{profile} {}", if file { "file" } else { "sqlite" });
            let directory = tempfile::tempdir().unwrap();
            let seed = imported();
            let kernel = open(directory.path(), file, authority.clone());
            kernel
                .seed(seed.document.clone(), || Timestamp::from_millis(10))
                .unwrap();
            assert!(
                kernel
                    .read(None)
                    .unwrap()
                    .aliases()
                    .nodes(seed.repository, KEY)
                    .is_empty(),
                "{at}: the seed gives no repository the key"
            );

            let verdict = submit(&kernel, &transaction(vec![add_alias(seed.first, KEY)]), 20);
            assert!(
                matches!(verdict, ValidationCommandResult::Validated(_)),
                "{at}: {verdict:?}"
            );
            let head = kernel.snapshot().unwrap();
            assert_eq!(head.revision, RevisionNumber::new(1), "{at}");
            assert_eq!(head.nodes[&seed.first].aliases, [KEY], "{at}");
            assert!(head.nodes[&seed.second].aliases.is_empty(), "{at}");
            assert_eq!(
                kernel
                    .read(None)
                    .unwrap()
                    .aliases()
                    .nodes(seed.repository, KEY),
                [seed.first],
                "{at}: the head's alias index answers the repository"
            );
            assert!(
                kernel
                    .read(Some(RevisionNumber::SEED))
                    .unwrap()
                    .aliases()
                    .nodes(seed.repository, KEY)
                    .is_empty(),
                "{at}: the seed revision's index is unchanged"
            );

            // A second alias for the same node, later, is appended after the first.
            let verdict = submit(
                &kernel,
                &transaction(vec![add_alias(seed.first, "runtime")]),
                30,
            );
            assert!(
                matches!(verdict, ValidationCommandResult::Validated(_)),
                "{at}: {verdict:?}"
            );
            assert_eq!(
                kernel.snapshot().unwrap().nodes[&seed.first].aliases,
                [KEY, "runtime"],
                "{at}"
            );

            // The key now identifies the first repository: the second may not take it.
            let taken = submit(&kernel, &transaction(vec![add_alias(seed.second, KEY)]), 40);
            assert_eq!(
                refused(&taken),
                [(ValidatorName::Structural, "alias-already-exists".to_owned())],
                "{at}"
            );
            assert_eq!(
                kernel.head().unwrap().unwrap().revision,
                RevisionNumber::new(2),
                "{at}: a refusal does not move the head"
            );

            let expected = every_revision(&kernel);
            let expected_head = kernel.head().unwrap();
            let expected_records = kernel.transactions().unwrap();
            assert_eq!(expected.len(), 3, "{at}");
            assert_ne!(
                expected[0].0.map(|root| root.knowledge_root),
                expected[1].0.map(|root| root.knowledge_root),
                "{at}: the alias moves the knowledge root"
            );
            drop(kernel);

            let reopened = open(directory.path(), file, authority.clone());
            assert_eq!(reopened.head().unwrap(), expected_head, "{at}");
            assert_eq!(every_revision(&reopened), expected, "{at}");
            assert_eq!(reopened.transactions().unwrap(), expected_records, "{at}");
            assert_eq!(
                reopened
                    .read(None)
                    .unwrap()
                    .aliases()
                    .nodes(seed.repository, KEY),
                [seed.first],
                "{at}: the reopened head resolves the key"
            );
        }
    }
}

/// Acceptance 2, through the store: an alias another node of the type holds, the same alias twice
/// in one transaction and a node no revision holds are each refused under their own name, and none
/// moves the head.
#[test]
fn a_held_alias_a_repeated_alias_and_an_unknown_node_are_each_refused_by_name() {
    for (profile, authority) in profiles() {
        for file in [false, true] {
            let at = format!("{profile} {}", if file { "file" } else { "sqlite" });
            let directory = tempfile::tempdir().unwrap();
            let seed = imported();
            let kernel = open(directory.path(), file, authority.clone());
            kernel
                .seed(seed.document.clone(), || Timestamp::from_millis(10))
                .unwrap();
            let committed = submit(&kernel, &transaction(vec![add_alias(seed.first, KEY)]), 20);
            assert!(
                matches!(committed, ValidationCommandResult::Validated(_)),
                "{at}: {committed:?}"
            );

            let cases = [
                (
                    "an alias another repository holds",
                    vec![add_alias(seed.second, KEY)],
                    (ValidatorName::Structural, "alias-already-exists"),
                ),
                (
                    "one alias given twice",
                    vec![
                        add_alias(seed.first, "twice"),
                        add_alias(seed.second, "twice"),
                    ],
                    (ValidatorName::Structural, "duplicate-alias"),
                ),
                (
                    "a node no revision holds",
                    vec![add_alias(NodeId::mint(), "orphan")],
                    (ValidatorName::Reference, "unresolved-node"),
                ),
                (
                    "the empty alias",
                    vec![add_alias(seed.second, "")],
                    (ValidatorName::Structural, "empty-alias"),
                ),
            ];
            for (offset, (what, operations, (validator, code))) in cases.into_iter().enumerate() {
                let verdict = submit(
                    &kernel,
                    &transaction(operations),
                    30 + 10 * i64::try_from(offset).unwrap(),
                );
                assert_eq!(
                    refused(&verdict),
                    [(validator, code.to_owned())],
                    "{at}: {what}"
                );
            }
            assert_eq!(
                kernel.head().unwrap().unwrap().revision,
                RevisionNumber::new(1),
                "{at}"
            );
        }
    }
}

/// The command lookup follows peer publication and an old-basis validation without changing
/// either the refusal text or an alias index a public reader already captured.
#[test]
fn alias_refusals_follow_peer_and_historical_revisions() {
    for (_, authority) in profiles() {
        for file in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let seed = imported();
            let kernel = open(directory.path(), file, authority.clone());
            kernel
                .seed(seed.document.clone(), || Timestamp::from_millis(10))
                .unwrap();
            let warm = transaction(vec![add_alias(seed.first, "warm")]);
            assert!(matches!(
                submit(&kernel, &warm, 20),
                ValidationCommandResult::Validated(_)
            ));
            let captured = kernel.read(None).unwrap();
            assert!(captured.aliases().nodes(seed.repository, KEY).is_empty());
            let peer = open(directory.path(), file, authority.clone());
            assert!(matches!(
                submit(&peer, &transaction(vec![add_alias(seed.first, KEY)]), 30),
                ValidationCommandResult::Validated(_)
            ));

            let reject = |tx: &GraphTransaction, at| {
                let expected = ekr_kernel::Pipeline::deterministic(context().validator)
                    .validate(&ekr_graph::GraphSnapshot::of(&peer.snapshot().unwrap()), tx)
                    .unwrap_err();
                let ValidationCommandResult::Rejected(found) = submit(&kernel, tx, at) else {
                    panic!("an alias published by the peer must be refused");
                };
                assert_eq!(
                    found
                        .issues
                        .iter()
                        .map(|issue| (issue.validator, issue.code.as_str(), issue.message.as_str()))
                        .collect::<Vec<_>>(),
                    expected
                        .iter()
                        .map(|issue| (issue.validator, issue.code.as_str(), issue.message.as_str()))
                        .collect::<Vec<_>>()
                );
            };
            reject(&transaction(vec![add_alias(seed.second, KEY)]), 40);

            let historical = transaction(vec![add_alias(seed.second, KEY)]);
            kernel
                .propose(&encode(&historical), context().operator, || {
                    Timestamp::from_millis(50)
                })
                .unwrap();
            assert!(matches!(
                kernel
                    .validate(historical.id, RevisionNumber::new(1), || {
                        Timestamp::from_millis(51)
                    })
                    .unwrap(),
                ValidationCommandResult::Validated(_)
            ));
            reject(&transaction(vec![add_alias(seed.second, KEY)]), 60);
            assert!(captured.aliases().nodes(seed.repository, KEY).is_empty());
            assert_eq!(
                captured.aliases().nodes(seed.repository, "warm"),
                [seed.first]
            );
            let cold = open(directory.path(), file, authority.clone());
            assert_eq!(cold.transactions().unwrap(), kernel.transactions().unwrap());
            assert_eq!(cold.snapshot().unwrap(), kernel.snapshot().unwrap());
        }
    }
}
