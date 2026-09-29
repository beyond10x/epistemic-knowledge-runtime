//! Adversary pass, wave sdk-01, unit L (`story:node-gains-an-alias`).
//!
//! Attacks on `!AddAlias` beyond the unit's own cases: a checkpoint restored across an added
//! alias, two transactions validated at one basis that give one alias (the second must go Stale),
//! an alias given to a node the same transaction creates, byte-exact aliases (case, Unicode
//! normalisation, whitespace), the preserving migration of a `/2` store holding an `AddAlias`,
//! and a property over generated transactions: whatever validation admits leaves no `(type,
//! alias)` of the root identifying more nodes than it did before.

#[allow(dead_code)]
mod current_fixture;
#[allow(dead_code)]
#[path = "support/v2_store.rs"]
mod v2;

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{
    EvidenceId, GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp, TransactionId,
    TypeId,
};
use ekr_graph::{CanonicalGraph, GraphRoot, GraphSnapshot, Node, Space};
use ekr_kernel::{
    Agent, AliasAddition, AuthorityStateV1, BootstrapContext, CommitCommandResult, GraphOperation,
    GraphTransaction, NodeDraft, Pipeline, Runtime, SeedDocument, ValidationCommandResult,
    ValidationProfileV1, ValidatorName,
};
use ekr_ontology::{NodeType, Ontology, OntologyDocument, SchemaVersion, Value};
use proptest::prelude::*;
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

fn profiles() -> [(&'static str, AuthorityStateV1); 2] {
    let validator = context().validator;
    [
        ("v1", anchor(ValidationProfileV1::deterministic(validator))),
        (
            "v3",
            anchor(ValidationProfileV1::identity_keeping(validator)),
        ),
    ]
}

fn open(path: &std::path::Path, file: bool, authority: AuthorityStateV1) -> Runtime {
    if file {
        Runtime::file(path, "adv", context(), authority)
    } else {
        Runtime::sqlite(&path.join("state.db"), "adv", context(), authority)
    }
    .unwrap()
}

/// One `Repository` type; two repositories created without an alias.
struct Imported {
    document: SeedDocument,
    root: GraphRootId,
    repository: TypeId,
    first: NodeId,
    second: NodeId,
}

fn imported() -> Imported {
    let mut document =
        SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let repository = TypeId::mint();
    document
        .ontology
        .node_types
        .push(NodeType::new(repository, "Repository"));
    let root = document.graph.root.id;
    let mut node = |name: &str| {
        let node = Node::<Value>::new(NodeId::mint(), root, repository, name);
        let id = node.id;
        document.graph.nodes.insert(id, node);
        id
    };
    let first = node("runtime");
    let second = node("website");
    Imported {
        document,
        root,
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

fn transaction(proposer: ekr_core::AgentId, operations: Vec<GraphOperation>) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer,
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

fn create(id: NodeId, root: GraphRootId, type_id: TypeId, aliases: &[&str]) -> GraphOperation {
    GraphOperation::CreateNode(NodeDraft {
        id,
        root_id: root,
        type_id,
        canonical_name: "created".to_owned(),
        properties: BTreeMap::new(),
        aliases: aliases.iter().map(|alias| (*alias).to_owned()).collect(),
    })
}

/// Propose and validate against the head at `at`, `at + 1`.
fn validated(kernel: &Runtime, tx: &GraphTransaction, at: i64) -> ValidationCommandResult {
    kernel
        .propose(&encode(tx), tx.proposer, || Timestamp::from_millis(at))
        .unwrap();
    let head = kernel.head().unwrap().unwrap().revision;
    kernel
        .validate(tx.id, head, || Timestamp::from_millis(at + 1))
        .unwrap()
}

/// Propose, validate and commit at `at`..`at + 2`; the transaction must commit.
fn committed(kernel: &Runtime, tx: &GraphTransaction, at: i64) {
    let verdict = validated(kernel, tx, at);
    assert!(
        matches!(verdict, ValidationCommandResult::Validated(_)),
        "{verdict:?}"
    );
    let result = kernel
        .commit(tx.id, tx.proposer, || Timestamp::from_millis(at + 2))
        .unwrap();
    assert!(
        matches!(result, CommitCommandResult::Committed(_)),
        "{result:?}"
    );
}

fn codes(verdict: &ValidationCommandResult) -> Vec<(ValidatorName, String)> {
    match verdict {
        ValidationCommandResult::Rejected(record) => record
            .issues
            .iter()
            .map(|issue| (issue.validator, issue.code.clone()))
            .collect(),
        ValidationCommandResult::Validated(_) => panic!("expected a rejection"),
    }
}

const KEY: &str = "source-a-repos:4711";

/// A checkpoint written at a head an `AddAlias` produced restores that alias: a fresh open that
/// continues from the checkpoint (no replay from the seed) answers the alias from its index,
/// refuses the alias to the other repository as `alias-already-exists`, and holds the graph a
/// full replay holds.
#[test]
fn a_checkpoint_past_an_added_alias_restores_the_alias_for_resolve_and_for_validation() {
    for (profile, authority) in profiles() {
        for file in [false, true] {
            let at = format!("{profile} {}", if file { "file" } else { "sqlite" });
            let directory = tempfile::tempdir().unwrap();
            let seed = imported();
            let kernel = open(directory.path(), file, authority.clone());
            kernel
                .seed(seed.document.clone(), || Timestamp::from_millis(10))
                .unwrap();
            committed(
                &kernel,
                &transaction(context().operator, vec![add_alias(seed.first, KEY)]),
                20,
            );
            kernel.retain_checkpoint_at_rest();
            drop(kernel);

            let reopened = open(directory.path(), file, authority.clone());
            let read = reopened.read(None).unwrap();
            assert_eq!(read.graph.revision, RevisionNumber::new(1), "{at}");
            assert_eq!(
                reopened.seed_replays(),
                0,
                "{at}: the reopened handle continued from the checkpoint"
            );
            assert_eq!(
                read.aliases().nodes(seed.repository, KEY),
                [seed.first],
                "{at}: the restored head's index answers the alias"
            );
            let verdict = validated(
                &reopened,
                &transaction(context().operator, vec![add_alias(seed.second, KEY)]),
                30,
            );
            assert_eq!(
                codes(&verdict),
                [(ValidatorName::Structural, "alias-already-exists".to_owned())],
                "{at}: validation against the restored head sees the alias"
            );

            let mut full = open(directory.path(), file, authority.clone());
            full.set_full_replay(true);
            assert_eq!(
                full.snapshot().unwrap(),
                reopened.snapshot().unwrap(),
                "{at}: checkpoint and full replay agree"
            );
        }
    }
}

/// Two transactions validated against one head give one alias of a type to two nodes: a
/// `CreateNode` and an `AddAlias`. Each is valid alone; the first to commit wins and the other
/// must be Stale, never Committed, so the head never holds the alias twice.
#[test]
fn an_add_alias_and_a_create_node_racing_at_one_basis_commit_once_and_the_other_is_stale() {
    for (profile, authority) in profiles() {
        for file in [false, true] {
            for add_first in [false, true] {
                let at = format!(
                    "{profile} {} {}",
                    if file { "file" } else { "sqlite" },
                    if add_first {
                        "add first"
                    } else {
                        "create first"
                    }
                );
                let directory = tempfile::tempdir().unwrap();
                let seed = imported();
                let kernel = open(directory.path(), file, authority.clone());
                kernel
                    .seed(seed.document.clone(), || Timestamp::from_millis(10))
                    .unwrap();
                let fresh = NodeId::mint();
                let creating = transaction(
                    context().operator,
                    vec![create(fresh, seed.root, seed.repository, &[KEY])],
                );
                let adding = transaction(context().operator, vec![add_alias(seed.first, KEY)]);
                for (offset, tx) in [&creating, &adding].into_iter().enumerate() {
                    let verdict = validated(&kernel, tx, 20 + 2 * i64::try_from(offset).unwrap());
                    assert!(
                        matches!(verdict, ValidationCommandResult::Validated(_)),
                        "{at}: {verdict:?}"
                    );
                }
                let (winner, loser) = if add_first {
                    (&adding, &creating)
                } else {
                    (&creating, &adding)
                };
                let first = kernel
                    .commit(winner.id, winner.proposer, || Timestamp::from_millis(30))
                    .unwrap();
                assert!(
                    matches!(first, CommitCommandResult::Committed(_)),
                    "{at}: {first:?}"
                );
                let second = kernel
                    .commit(loser.id, loser.proposer, || Timestamp::from_millis(31))
                    .unwrap();
                assert!(
                    matches!(second, CommitCommandResult::Stale(_)),
                    "{at}: the second commit at a moved basis is Stale: {second:?}"
                );
                let head = kernel.read(None).unwrap();
                assert_eq!(
                    head.aliases().nodes(seed.repository, KEY).len(),
                    1,
                    "{at}: one node of the type holds the alias"
                );
            }
        }
    }
}

/// An `AddAlias` naming a node the same transaction creates commits, the alias follows the
/// draft's own, and the next head resolves the node by either.
#[test]
fn an_alias_added_to_a_node_created_in_the_same_transaction_is_appended_after_the_drafts() {
    for (profile, authority) in profiles() {
        let directory = tempfile::tempdir().unwrap();
        let seed = imported();
        let kernel = open(directory.path(), false, authority.clone());
        kernel
            .seed(seed.document.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let fresh = NodeId::mint();
        committed(
            &kernel,
            &transaction(
                context().operator,
                vec![
                    add_alias(fresh, "later"),
                    create(fresh, seed.root, seed.repository, &["own"]),
                ],
            ),
            20,
        );
        let head = kernel.read(None).unwrap();
        assert_eq!(
            head.graph.nodes[&fresh].aliases,
            ["own", "later"],
            "{profile}"
        );
        assert_eq!(
            head.aliases().nodes(seed.repository, "later"),
            [fresh],
            "{profile}"
        );
        assert_eq!(
            head.aliases().nodes(seed.repository, "own"),
            [fresh],
            "{profile}"
        );
    }
}

/// Aliases are compared byte for byte, as a `CreateNode`'s are: a case variant, the other Unicode
/// normalisation form of one text, and a whitespace-only alias are each admitted to another node
/// of the type, and each is answered by its own node only.
#[test]
fn aliases_differing_in_case_normalisation_or_whitespace_are_distinct_and_each_resolves_to_one_node(
) {
    let (_, authority) = profiles()[0].clone();
    let directory = tempfile::tempdir().unwrap();
    let seed = imported();
    let kernel = open(directory.path(), true, authority);
    kernel
        .seed(seed.document.clone(), || Timestamp::from_millis(10))
        .unwrap();
    let composed = "caf\u{e9}";
    let decomposed = "cafe\u{301}";
    committed(
        &kernel,
        &transaction(
            context().operator,
            vec![
                add_alias(seed.first, "Acme"),
                add_alias(seed.first, composed),
                add_alias(seed.first, " "),
            ],
        ),
        20,
    );
    committed(
        &kernel,
        &transaction(
            context().operator,
            vec![
                add_alias(seed.second, "acme"),
                add_alias(seed.second, decomposed),
                add_alias(seed.second, "  "),
            ],
        ),
        30,
    );
    let head = kernel.read(None).unwrap();
    let index = head.aliases();
    for (alias, node) in [
        ("Acme", seed.first),
        (composed, seed.first),
        (" ", seed.first),
        ("acme", seed.second),
        (decomposed, seed.second),
        ("  ", seed.second),
    ] {
        assert_eq!(index.nodes(seed.repository, alias), [node], "{alias:?}");
    }
    let taken = validated(
        &kernel,
        &transaction(context().operator, vec![add_alias(seed.second, " ")]),
        40,
    );
    assert_eq!(
        codes(&taken),
        [(ValidatorName::Structural, "alias-already-exists".to_owned())]
    );
}

/// `ekr migrate`'s kernel path over a `/2` store to which this kernel committed an `AddAlias`:
/// the destination holds the alias and its index answers it, as the source's does.
#[test]
fn a_v2_store_holding_an_added_alias_migrates_with_the_alias() {
    use current_fixture::{anchor as fixture_anchor, context as fixture_context, id};
    let subject: TypeId = id(0x05);
    let second: NodeId = id(0x11);
    for file in [false, true] {
        let source_directory = tempfile::tempdir().unwrap();
        v2::fixture_store(
            source_directory.path(),
            file,
            &current_fixture::seed(),
            fixture_context(),
            &fixture_anchor(),
            current_fixture::SEEDED_AT,
        );
        let source = if file {
            Runtime::file_existing(
                source_directory.path(),
                "ekr",
                fixture_context(),
                fixture_anchor(),
            )
        } else {
            Runtime::sqlite_existing(
                &source_directory.path().join("state.db"),
                "ekr",
                fixture_context(),
                fixture_anchor(),
            )
        }
        .unwrap();
        let before_head = source.head().unwrap().unwrap().revision;
        committed(
            &source,
            &transaction(fixture_context().operator, vec![add_alias(second, KEY)]),
            1_000,
        );
        let before = source.read(None).unwrap();
        assert_eq!(before.graph.revision, before_head.next().unwrap());
        assert_eq!(before.aliases().nodes(subject, KEY), [second]);

        let destination_directory = tempfile::tempdir().unwrap();
        let destination = current_fixture::open(
            destination_directory.path(),
            file,
            fixture_context(),
            fixture_anchor(),
        );
        source.migrate_into(&destination).unwrap();
        let after = destination.read(None).unwrap();
        assert_eq!(after.graph.nodes, before.graph.nodes, "file={file}");
        assert_eq!(after.aliases().nodes(subject, KEY), [second], "file={file}");
        // `parent` names the lineage's prior record, which the migration derives again.
        assert_eq!(
            after.root.knowledge_root, before.root.knowledge_root,
            "file={file}"
        );
    }
}

// Property ------------------------------------------------------------------------------------

struct Pool {
    root: GraphRootId,
    elsewhere: GraphRootId,
    types: [TypeId; 2],
    held: [NodeId; 4],
    created: [NodeId; 2],
    proposer: ekr_core::AgentId,
    validator: ekr_core::AgentId,
}

static POOL: std::sync::LazyLock<Pool> = std::sync::LazyLock::new(|| Pool {
    root: GraphRootId::mint(),
    elsewhere: GraphRootId::mint(),
    types: [TypeId::mint(), TypeId::mint()],
    held: [
        NodeId::mint(),
        NodeId::mint(),
        NodeId::mint(),
        NodeId::mint(),
    ],
    created: [NodeId::mint(), NodeId::mint()],
    proposer: ekr_core::AgentId::mint(),
    validator: ekr_core::AgentId::mint(),
});

const ALIASES: [&str; 4] = ["x", "y", "", " "];

/// `(in the root, type, aliases)` for each of the four held nodes.
type Held = (bool, usize, Vec<usize>);

#[derive(Clone, Debug)]
enum Op {
    Create(usize, usize, Vec<usize>),
    Add(usize, usize),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (
            0..2usize,
            0..2usize,
            proptest::collection::vec(0..4usize, 0..3)
        )
            .prop_map(|(node, type_at, aliases)| Op::Create(node, type_at, aliases)),
        (0..6usize, 0..4usize).prop_map(|(node, alias)| Op::Add(node, alias)),
    ]
}

fn basis(held: &[Held]) -> CanonicalGraph {
    let p = &*POOL;
    let schema = SchemaVersionId::mint();
    let mut graph = CanonicalGraph {
        root: GraphRoot {
            id: p.root,
            space: Space::Canonical,
            schema_version_id: schema,
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::new(3),
        ontology: Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(schema, Timestamp::EPOCH),
            node_types: vec![
                NodeType::new(p.types[0], "Repository"),
                NodeType::new(p.types[1], "Team"),
            ],
            edge_types: Vec::new(),
        })
        .expect("two node types cohere"),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    };
    for (at, (home, type_at, aliases)) in held.iter().enumerate() {
        let mut node = ekr_graph::Node::new(
            p.held[at],
            if *home { p.root } else { p.elsewhere },
            p.types[*type_at],
            "held",
        );
        node.aliases = aliases.iter().map(|a| ALIASES[*a].to_owned()).collect();
        graph.nodes.insert(node.id, node);
    }
    graph
}

/// The nodes of the root holding each non-empty `(type, alias)`.
fn holders(graph: &CanonicalGraph) -> BTreeMap<(TypeId, String), BTreeSet<NodeId>> {
    let mut out: BTreeMap<(TypeId, String), BTreeSet<NodeId>> = BTreeMap::new();
    for node in graph.nodes.values() {
        if node.root_id != graph.root.id {
            continue;
        }
        for alias in node.aliases.iter().filter(|alias| !alias.is_empty()) {
            out.entry((node.type_id, alias.clone()))
                .or_default()
                .insert(node.id);
        }
    }
    out
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 2_000,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x00ad_d0a1_1a5e),
        failure_persistence: None,
        ..ProptestConfig::default()
    })]

    /// Invariant 1 through `AddAlias`: whatever the pipeline admits, applied as the kernel applies
    /// it (creates first, then each added alias appended in operation order), leaves every
    /// non-empty `(type, alias)` of the root identifying at most one node, or exactly the nodes
    /// it identified before when the basis already held it twice (a seed may).
    #[test]
    fn an_admitted_transaction_never_makes_an_alias_identify_a_second_node(
        held in proptest::collection::vec(
            (proptest::bool::weighted(0.85), 0..2usize, proptest::collection::vec(0..4usize, 0..3)),
            4..=4,
        ),
        ops in proptest::collection::vec(op(), 1..6),
    ) {
        let p = &*POOL;
        let graph = basis(&held);
        let every = [p.held[0], p.held[1], p.held[2], p.held[3], p.created[0], p.created[1]];
        let proposal = GraphTransaction {
            id: TransactionId::mint(),
            proposer: p.proposer,
            operations: ops
                .iter()
                .map(|op| match op {
                    Op::Create(node, type_at, aliases) => GraphOperation::CreateNode(NodeDraft {
                        id: p.created[*node],
                        root_id: p.root,
                        type_id: p.types[*type_at],
                        canonical_name: "created".to_owned(),
                        properties: BTreeMap::new(),
                        aliases: aliases.iter().map(|a| ALIASES[*a].to_owned()).collect(),
                    }),
                    Op::Add(node, alias) => add_alias(every[*node], ALIASES[*alias]),
                })
                .collect(),
            evidence: BTreeSet::new(),
            schema_version: None,
        };
        let admitted = Pipeline::deterministic(p.validator)
            .validate(&GraphSnapshot::of(&graph), &proposal)
            .is_ok();
        if !admitted {
            return Ok(());
        }

        let mut after = graph.clone();
        for operation in &proposal.operations {
            if let GraphOperation::CreateNode(draft) = operation {
                let mut node = ekr_graph::Node::new(
                    draft.id,
                    draft.root_id,
                    draft.type_id,
                    draft.canonical_name.clone(),
                );
                node.aliases.clone_from(&draft.aliases);
                after.nodes.insert(node.id, node);
            }
        }
        for operation in &proposal.operations {
            if let GraphOperation::AddAlias(addition) = operation {
                after
                    .nodes
                    .get_mut(&addition.node)
                    .expect("an admitted AddAlias names a node")
                    .aliases
                    .push(addition.alias.clone());
            }
        }
        let before = holders(&graph);
        for (key, now) in holders(&after) {
            let was = before.get(&key).cloned().unwrap_or_default();
            if was.len() <= 1 {
                prop_assert!(
                    now.len() <= 1,
                    "{:?} identified {:?} and now {:?} after {:?}",
                    key,
                    was,
                    now,
                    proposal.operations
                );
            } else {
                prop_assert_eq!(&now, &was, "{:?} after {:?}", key, proposal.operations);
            }
        }
    }
}
