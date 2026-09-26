//! Adversary pass 2 on unit K `p5-01-kernel` (story:schema-evolution-transactions, parts B and C).
//!
//! Drives the pass-1 correction and what pass 1 did not reach: the hand-written `ModifyProperty`
//! parser, the owned/ownerless encodings, `InstanceState` over edges and inherited types, the v2
//! path end to end across several versions, a schema commit whose publication was lost, two
//! schema changes racing for one head, and design § 95 read as the list of cases it claims.
//!
//! Fixtures use the runtime's own vocabulary and no retained store is embedded here.

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::rc::Rc;

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    RevisionNumber, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue,
    Confidence, Edge, Evidence, EvidenceSource, GraphRoot, GraphSnapshot, Node, Object, Predicate,
    Root, Space, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    Agent, AuthorityStateV1, BootstrapContext, Commit, CommitCommandResult, EdgeDraft,
    GraphOperation, GraphTransaction, KernelAuthority, NodeDraft, Pipeline, PropertyModification,
    PropertyMutation, Runtime, SeedDocument, TransactionDocument, ValidationCommandResult,
    ValidationIssue, ValidationProfileV1,
};
use ekr_ontology::{
    Cardinality, EdgeType, NodeType, Ontology, OntologyDocument, PropertyDefinition, SchemaVersion,
    Value, ValueType,
};
use ekr_store::{Publication, PublicationCommandKey, PublicationPreparationV1, StoreError};
use serde::Serialize;

const TENANT: &str = "test";

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}

fn v2() -> AuthorityStateV1 {
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
        validation_profile: ValidationProfileV1::schema_evolving(c.validator),
    }
}

fn open(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, TENANT, context(), v2())
    } else {
        Runtime::sqlite(&path.join("state.db"), TENANT, context(), v2())
    }
    .unwrap()
}

fn encode(tx: &GraphTransaction) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}

fn transaction(
    operations: Vec<GraphOperation>,
    schema_version: Option<SchemaVersionId>,
) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: BTreeSet::new(),
        schema_version,
    }
}

fn modify(owner: TypeId, property: PropertyDefinition) -> GraphOperation {
    GraphOperation::ModifyProperty(PropertyModification {
        owner: Some(owner),
        property,
    })
}

/// Propose, validate against the head, and commit if validated.
fn submit(kernel: &Runtime, tx: &GraphTransaction, at: i64) -> Result<(), Vec<String>> {
    kernel
        .propose(&encode(tx), context().operator, || {
            Timestamp::from_millis(at)
        })
        .unwrap();
    let head = kernel.head().unwrap().unwrap().revision;
    match kernel
        .validate(tx.id, head, || Timestamp::from_millis(at + 1))
        .unwrap()
    {
        ValidationCommandResult::Validated(_) => {
            let committed = kernel
                .commit(tx.id, context().operator, || Timestamp::from_millis(at + 2))
                .unwrap();
            assert!(
                matches!(committed, CommitCommandResult::Committed(_)),
                "{committed:?}"
            );
            Ok(())
        }
        ValidationCommandResult::Rejected(record) => Err(record
            .issues
            .iter()
            .map(|issue| issue.code.clone())
            .collect()),
    }
}

/// The seed: one `Subject` type with a `label`, one subject node, and one assertion on it.
struct Seeded {
    document: SeedDocument,
    subject_type: TypeId,
    subject: NodeId,
    assertion: AssertionId,
}

fn seeded() -> Seeded {
    let mut document =
        SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let (subject_type, label) = (TypeId::mint(), PropertyId::mint());
    let mut declared = NodeType::new(subject_type, "Subject");
    declared.properties.insert(
        label,
        PropertyDefinition::new(label, "label", ValueType::String),
    );
    document.ontology.node_types.push(declared);
    let mut node = Node::<Value>::new(NodeId::mint(), document.graph.root.id, subject_type, "seed");
    node.properties
        .insert(label, vec![Value::String("seeded".into())]);
    let bytes = b"synthetic operator statement".to_vec();
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
    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id: document.graph.root.id,
        subject: Subject::Node(node.id),
        predicate: Predicate::Property(label),
        object: Object::Value(Value::String("seeded".into())),
        evidence: BTreeSet::from([evidence.id]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    let (subject, assertion_id) = (node.id, assertion.id);
    document.graph.nodes.insert(node.id, node);
    document.graph.assertions.insert(assertion.id, assertion);
    document.graph.evidence.insert(evidence.id, evidence);
    document.evidence_payloads.insert(hash, bytes);
    Seeded {
        document,
        subject_type,
        subject,
        assertion: assertion_id,
    }
}

/// Every revision's graph and head root, read through replay and through a verified read.
fn every_revision(kernel: &Runtime) -> Vec<(CanonicalGraph, Root)> {
    let head = kernel.head().unwrap().unwrap();
    (0..=head.revision.get())
        .map(|number| {
            let revision = RevisionNumber::new(number);
            let graph = kernel.replay(revision).unwrap();
            let read = kernel.read(Some(revision)).unwrap();
            let snapshot = read.snapshot(None).unwrap();
            assert_eq!(
                ContentHash::of(&graph.ontology),
                snapshot.root.ontology_root,
                "revision {number}: the verified read's ontology root is the replayed ontology's"
            );
            (graph, snapshot.root)
        })
        .collect()
}

// 1. Design § 95 as the list of cases it names --------------------------------------------------

/// Every `*.rs` file under `crates/*/tests`, as text.
fn test_sources(crates: &Path) -> Vec<(std::path::PathBuf, String)> {
    fn walk(dir: &Path, out: &mut Vec<(std::path::PathBuf, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                out.push((path.clone(), std::fs::read_to_string(&path).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    for member in std::fs::read_dir(crates).unwrap().flatten() {
        walk(&member.path().join("tests"), &mut out);
    }
    out
}

/// § 95 says of each claim which case executes it. Every case name it gives (a backticked
/// snake_case identifier of four or more words) is a `fn` some test file of this workspace
/// declares. A name nothing declares is a claim nothing executes.
#[test]
fn design_section_95_names_only_cases_that_exist() {
    let manifest = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the manifest directory"),
    );
    let root = manifest.join("../..");
    let design =
        std::fs::read_to_string(root.join("docs/epistemic-knowledge-runtime-design.md")).unwrap();
    let start = design
        .find("\n# 95. Schema Evolution Transactions")
        .expect("§ 95 exists");
    let section = &design[start + 1..];
    let section = section[1..]
        .find("\n# ")
        .map_or(section, |end| &section[..=end]);

    let names: BTreeSet<&str> = section
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|token| {
            token.matches('_').count() >= 3
                && token
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        })
        .collect();
    assert!(
        names.len() >= 10,
        "the scan found {} case names in § 95; it has stopped reading the section",
        names.len()
    );

    let sources = test_sources(&root.join("crates"));
    let missing: Vec<&str> = names
        .iter()
        .copied()
        .filter(|name| {
            let declared = format!("fn {name}(");
            !sources.iter().any(|(_, text)| text.contains(&declared))
        })
        .collect();
    assert!(
        missing.is_empty(),
        "§ 95 names cases no test file declares: {missing:?}"
    );
}

// 2. The hand-written ModifyProperty parser -----------------------------------------------------

const TX: &str = "00000000-0000-4000-8000-0000000000b1";
const AGENT: &str = "00000000-0000-4000-8000-0000000000b2";
const TYPE: &str = "00000000-0000-4000-8000-0000000000b3";
const PROPERTY: &str = "00000000-0000-4000-8000-0000000000b4";

fn document(operation: &str) -> String {
    format!(
        "format: ekr.transaction-document/1\ntransaction:\n  id: {TX}\n  proposer: {AGENT}\n  \
         operations:\n  - {operation}\n  evidence: []\n"
    )
}

fn parsed_modification(operation: &str) -> Result<PropertyModification, String> {
    let parsed =
        TransactionDocument::parse(document(operation).as_bytes()).map_err(|e| e.to_string())?;
    match &parsed.transaction().operations[0] {
        GraphOperation::ModifyProperty(modification) => Ok(modification.clone()),
        other => panic!("not a ModifyProperty: {other:?}"),
    }
}

/// Key order does not matter, nor a tag on the owner; a null or duplicated owner, an owned shape with a P1 key
/// beside it, and a P1 shape with a key of neither shape are refused.
#[test]
fn the_modify_property_parser_refuses_every_ambiguous_mapping() {
    let property = format!("{{id: {PROPERTY}, name: note, value_type: {{value_kind: String}}}}");
    let owned = parsed_modification(&format!(
        "!ModifyProperty {{owner: {TYPE}, property: {property}}}"
    ))
    .unwrap();
    let swapped = parsed_modification(&format!(
        "!ModifyProperty {{property: {property}, owner: {TYPE}}}"
    ))
    .unwrap();
    assert_eq!(owned, swapped);
    assert_eq!(owned.owner, Some(TYPE.parse().unwrap()));
    // The frozen document profile ignores an explicit tag on a typed scalar
    // (`transaction_document.rs`, `builtin_tags_do_not_override_actual_typed_string_and_numeric_decoding`),
    // so a tagged owner reads as the untagged one, as every other id field does.
    assert_eq!(
        parsed_modification(&format!(
            "!ModifyProperty {{owner: !Some {TYPE}, property: {property}}}"
        ))
        .unwrap(),
        owned
    );

    let p1 = parsed_modification(&format!(
        "!ModifyProperty {{value_type: {{value_kind: String}}, name: note, id: {PROPERTY}}}"
    ))
    .unwrap();
    assert_eq!(p1.owner, None);
    assert_eq!(p1.property, owned.property);

    for refused in [
        format!("!ModifyProperty {{owner: null, property: {property}}}"),
        format!("!ModifyProperty {{owner: ~, property: {property}}}"),
        format!("!ModifyProperty {{owner: {TYPE}, owner: {TYPE}, property: {property}}}"),
        format!("!ModifyProperty {{owner: {TYPE}, property: {property}, property: {property}}}"),
        format!("!ModifyProperty {{owner: {TYPE}, property: {property}, required: false}}"),
        format!("!ModifyProperty {{owner: {TYPE}, id: {PROPERTY}, name: note, value_type: {{value_kind: String}}}}"),
        format!("!ModifyProperty {{id: {PROPERTY}, id: {PROPERTY}, name: note, value_type: {{value_kind: String}}}}"),
        format!("!ModifyProperty {{id: {PROPERTY}, name: note, value_type: {{value_kind: String}}, owner_id: {TYPE}}}"),
        format!("!ModifyProperty {{property: {property}}}"),
        format!("!ModifyProperty {{owner: {TYPE}}}"),
        "!ModifyProperty {}".to_owned(),
        format!("!ModifyProperty [{TYPE}, {property}]"),
    ] {
        assert!(
            parsed_modification(&refused).is_err(),
            "the parser accepted {refused}"
        );
    }

    // What the parser reads, the serializer writes back to the same value, for both shapes.
    for modification in [owned, p1] {
        let tx = GraphTransaction {
            id: TX.parse().unwrap(),
            proposer: AGENT.parse().unwrap(),
            operations: vec![GraphOperation::ModifyProperty(modification.clone())],
            evidence: BTreeSet::new(),
            schema_version: None,
        };
        let reread = TransactionDocument::parse(&encode(&tx)).unwrap();
        assert_eq!(reread.transaction(), &tx);
    }
}

// 3. Encodings ----------------------------------------------------------------------------------

fn bytes_of(value: &impl Canonical) -> Vec<u8> {
    let mut out = Encoder::new();
    value.encode(&mut out);
    out.finish()
}

/// The owned shape encodes as the ownerless one with a tagged owner in front, so no owned
/// modification shares an address with an ownerless one, and none with another owner's.
#[test]
fn owned_and_ownerless_modifications_never_share_an_address() {
    let mut declared = PropertyDefinition::new(PropertyId::mint(), "note", ValueType::String);
    declared.constraints = vec!["opaque".to_owned()];
    let ownerless = GraphOperation::<CanonicalValue>::ModifyProperty(PropertyModification {
        owner: None,
        property: declared.clone(),
    });
    let (one, two) = (TypeId::mint(), TypeId::mint());
    let owned = |owner| {
        GraphOperation::<CanonicalValue>::ModifyProperty(PropertyModification {
            owner: Some(owner),
            property: declared.clone(),
        })
    };
    let (bare, first, second) = (
        bytes_of(&ownerless),
        bytes_of(&owned(one)),
        bytes_of(&owned(two)),
    );
    assert_ne!(bare, first);
    assert_ne!(first, second);
    // Variant marker, then either the declaration, or SOME + owner + the declaration.
    assert_eq!(bare[..5], first[..5], "the same variant marker");
    assert!(
        first.ends_with(&bare[5..]),
        "the declaration is written last"
    );
    assert_ne!(
        bare[5], first[5],
        "the owner's tag is not the declaration's first tag"
    );
    assert_ne!(
        ContentHash::of(&ownerless),
        ContentHash::of(&owned(one)),
        "addresses"
    );
}

// 4. InstanceState over edges and inherited types -----------------------------------------------

/// `Base` (abstract, declares `label`), `Leaf` specialising it, `link` between leaves declaring
/// `weight` (Many integers) and `rank` (a string only an assertion on the edge holds).
struct Hierarchy {
    graph: CanonicalGraph,
    base: TypeId,
    link: TypeId,
    label: PropertyId,
    weight: PropertyId,
    rank: PropertyId,
    reviewer: AgentId,
    proposer: AgentId,
}

impl Hierarchy {
    fn new() -> Self {
        let (base, leaf, link) = (TypeId::mint(), TypeId::mint(), TypeId::mint());
        let (label, weight, rank) = (PropertyId::mint(), PropertyId::mint(), PropertyId::mint());
        let root_id = GraphRootId::mint();
        let (proposer, reviewer) = (AgentId::mint(), AgentId::mint());

        let mut base_type = NodeType::new(base, "Base");
        base_type.abstract_type = true;
        base_type.properties.insert(
            label,
            PropertyDefinition::new(label, "label", ValueType::String),
        );
        let mut leaf_type = NodeType::new(leaf, "Leaf");
        leaf_type.parents = [base].into_iter().collect();
        let mut link_type = EdgeType::new(link, "link");
        link_type.source_types = [leaf].into_iter().collect();
        link_type.target_types = [leaf].into_iter().collect();
        let mut many = PropertyDefinition::new(weight, "weight", ValueType::Integer);
        many.cardinality = Cardinality::Many;
        link_type.properties.insert(weight, many);
        link_type.properties.insert(
            rank,
            PropertyDefinition::new(rank, "rank", ValueType::String),
        );
        let ontology = Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
            node_types: vec![base_type, leaf_type],
            edge_types: vec![link_type],
        })
        .expect("the hierarchy coheres");

        let (a, b) = (NodeId::mint(), NodeId::mint());
        let mut labelled = Node::new(a, root_id, leaf, "labelled");
        labelled
            .properties
            .insert(label, vec![CanonicalValue::String("a".into())]);
        let unlabelled = Node::new(b, root_id, leaf, "unlabelled");
        let edge_id = EdgeId::mint();
        let mut edge = Edge::new(
            edge_id,
            root_id,
            link,
            CanonicalRef::new(a),
            CanonicalRef::new(b),
        );
        edge.properties.insert(
            weight,
            vec![CanonicalValue::Integer(1), CanonicalValue::Integer(2)],
        );
        let evidence_id = EvidenceId::mint();
        let evidence = Evidence {
            id: evidence_id,
            source: EvidenceSource::HumanStatement {
                identity: Some("operator".into()),
            },
            content_hash: ContentHash::of(&"the link's rank".to_owned()),
            extracted_by: proposer,
            observed_at: Timestamp::EPOCH,
            confidence: Confidence::CERTAIN,
        };
        let assertion = Assertion {
            id: AssertionId::mint(),
            root_id,
            subject: Subject::Edge(CanonicalRef::new(edge_id)),
            predicate: Predicate::Property(rank),
            object: Object::Value(CanonicalValue::String("first".into())),
            evidence: [CanonicalRef::new(evidence_id)].into_iter().collect(),
            proposed_by: proposer,
            lifecycle: AssertionLifecycle::Active,
            assessment: Assessment::Proposed,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        };
        Self {
            graph: CanonicalGraph {
                root: GraphRoot {
                    id: root_id,
                    space: Space::Canonical,
                    schema_version_id: ontology.version().id,
                    parent: None,
                    created_at: Timestamp::EPOCH,
                },
                revision: RevisionNumber::new(1),
                ontology,
                nodes: [(a, labelled), (b, unlabelled)].into_iter().collect(),
                edges: [(edge_id, edge)].into_iter().collect(),
                assertions: [(assertion.id, assertion)].into_iter().collect(),
                evidence: [(evidence_id, evidence)].into_iter().collect(),
            },
            base,
            link,
            label,
            weight,
            rank,
            reviewer,
            proposer,
        }
    }

    fn validate(&self, operation: GraphOperation) -> Result<(), Vec<ValidationIssue>> {
        Pipeline::schema_evolving(self.reviewer, BTreeSet::new())
            .validate(
                &GraphSnapshot::of(&self.graph),
                &GraphTransaction {
                    id: TransactionId::mint(),
                    proposer: self.proposer,
                    operations: vec![operation],
                    evidence: BTreeSet::new(),
                    schema_version: Some(SchemaVersionId::mint()),
                },
            )
            .map(|_| ())
    }

    fn codes(&self, operation: GraphOperation) -> Vec<String> {
        self.validate(operation)
            .expect_err("expected a refusal")
            .into_iter()
            .map(|issue| issue.code)
            .collect()
    }
}

/// A change on an abstract parent is held to the nodes of the concrete type under it, and a
/// change on an edge type to the edges and to assertions whose subject is an edge.
#[test]
fn instance_state_reads_edges_edge_assertions_and_inherited_types() {
    let world = Hierarchy::new();

    let mut required = PropertyDefinition::new(world.label, "label", ValueType::String);
    required.required = true;
    assert_eq!(
        world.codes(modify(world.base, required)),
        vec!["required-property-missing"],
        "a Leaf node lacks the label its abstract parent now requires"
    );
    assert_eq!(
        world.codes(modify(
            world.base,
            PropertyDefinition::new(world.label, "label", ValueType::Integer)
        )),
        vec!["value-kind-not-admitted"],
        "a Leaf node holds a string label"
    );

    let mut since = PropertyDefinition::new(PropertyId::mint(), "since", ValueType::Timestamp);
    since.required = true;
    assert_eq!(
        world.codes(modify(world.link, since)),
        vec!["required-property-missing"],
        "the edge lacks a newly required property"
    );
    assert_eq!(
        world.codes(modify(
            world.link,
            PropertyDefinition::new(world.weight, "weight", ValueType::Integer)
        )),
        vec!["cardinality-narrowed"],
        "the edge holds two weights"
    );
    let mut stringly = PropertyDefinition::new(world.weight, "weight", ValueType::String);
    stringly.cardinality = Cardinality::Many;
    assert_eq!(
        world.codes(modify(world.link, stringly)),
        vec!["value-kind-not-admitted"],
        "the edge holds integer weights"
    );
    assert_eq!(
        world.codes(modify(
            world.link,
            PropertyDefinition::new(world.rank, "rank", ValueType::Integer)
        )),
        vec!["value-kind-not-admitted"],
        "an active assertion on the edge holds a string rank"
    );

    // And the same state admits what it does not break.
    world
        .validate(modify(
            world.base,
            PropertyDefinition::new(PropertyId::mint(), "note", ValueType::String),
        ))
        .unwrap();
    let mut widened = PropertyDefinition::new(world.rank, "rank", ValueType::String);
    widened.cardinality = Cardinality::Many;
    world.validate(modify(world.link, widened)).unwrap();
}

// 5. The v2 path end to end, across four versions -----------------------------------------------

/// Seed; an edge type and a property on the seed's type; the edge used; the property redeclared
/// Many and given two values; a redeclaration back to One refused; the same redeclaration offered
/// again refused as without effect; a property added to the edge type and used; reopen. Every
/// revision's graph and verified root is the same after the reopen, and the seed's assertion
/// still explains, on both providers.
#[test]
fn a_property_redeclared_across_versions_replays_on_both_providers() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let kernel = open(directory.path(), file);
        kernel
            .seed(seed.document.clone(), || Timestamp::from_millis(10))
            .unwrap();

        let (cites, note, strength) = (TypeId::mint(), PropertyId::mint(), PropertyId::mint());
        let mut cites_type = EdgeType::new(cites, "cites");
        cites_type.source_types = [seed.subject_type].into_iter().collect();
        cites_type.target_types = [seed.subject_type].into_iter().collect();
        // Revision 1: version 1.
        submit(
            &kernel,
            &transaction(
                vec![
                    GraphOperation::DefineEdgeType(Box::new(cites_type)),
                    modify(
                        seed.subject_type,
                        PropertyDefinition::new(note, "note", ValueType::String),
                    ),
                ],
                Some(SchemaVersionId::mint()),
            ),
            20,
        )
        .unwrap();
        // Revision 2: data, using the edge type and the property.
        let other = NodeId::mint();
        submit(
            &kernel,
            &transaction(
                vec![
                    GraphOperation::CreateNode(NodeDraft {
                        id: other,
                        root_id: seed.document.graph.root.id,
                        type_id: seed.subject_type,
                        canonical_name: "cited".into(),
                        properties: BTreeMap::from([(note, vec![Value::String("one".into())])]),
                    }),
                    GraphOperation::CreateEdge(EdgeDraft {
                        id: EdgeId::mint(),
                        root_id: seed.document.graph.root.id,
                        type_id: cites,
                        source: seed.subject,
                        target: other,
                        properties: BTreeMap::new(),
                    }),
                ],
                None,
            ),
            30,
        )
        .unwrap();
        // Revision 3: version 2, `note` redeclared Many.
        let mut many = PropertyDefinition::new(note, "note", ValueType::String);
        many.cardinality = Cardinality::Many;
        submit(
            &kernel,
            &transaction(
                vec![modify(seed.subject_type, many.clone())],
                Some(SchemaVersionId::mint()),
            ),
            40,
        )
        .unwrap();
        // Revision 4: two notes on one node.
        submit(
            &kernel,
            &transaction(
                vec![GraphOperation::UpdateProperty(PropertyMutation {
                    node: other,
                    property: note,
                    values: vec![Value::String("one".into()), Value::String("two".into())],
                })],
                None,
            ),
            50,
        )
        .unwrap();
        // Back to One is refused; Many again is without effect.
        assert_eq!(
            submit(
                &kernel,
                &transaction(
                    vec![modify(
                        seed.subject_type,
                        PropertyDefinition::new(note, "note", ValueType::String)
                    )],
                    Some(SchemaVersionId::mint()),
                ),
                60,
            ),
            Err(vec!["cardinality-narrowed".to_owned()])
        );
        assert_eq!(
            submit(
                &kernel,
                &transaction(
                    vec![modify(seed.subject_type, many)],
                    Some(SchemaVersionId::mint()),
                ),
                70,
            ),
            Err(vec!["schema-change-without-effect".to_owned()])
        );
        // Revision 5: version 3, a property on the edge type that already has an edge.
        submit(
            &kernel,
            &transaction(
                vec![modify(
                    cites,
                    PropertyDefinition::new(strength, "strength", ValueType::Integer),
                )],
                Some(SchemaVersionId::mint()),
            ),
            80,
        )
        .unwrap();
        // Revision 6: a second edge carrying it; a string strength is refused.
        let edge = |value: Value| {
            transaction(
                vec![GraphOperation::CreateEdge(EdgeDraft {
                    id: EdgeId::mint(),
                    root_id: seed.document.graph.root.id,
                    type_id: cites,
                    source: other,
                    target: seed.subject,
                    properties: BTreeMap::from([(strength, vec![value])]),
                })],
                None,
            )
        };
        assert!(submit(&kernel, &edge(Value::String("high".into())), 90).is_err());
        submit(&kernel, &edge(Value::Integer(3)), 100).unwrap();

        let head = kernel.snapshot().unwrap();
        assert_eq!(head.ontology.version().number, 3);
        assert_eq!(head.revision, RevisionNumber::new(6));
        let expected = every_revision(&kernel);
        let records = kernel.transactions().unwrap();
        let explained = kernel.read(None).unwrap().explain(seed.assertion).unwrap();
        drop(kernel);

        let reopened = open(directory.path(), file);
        assert_eq!(every_revision(&reopened), expected);
        assert_eq!(reopened.transactions().unwrap(), records);
        assert_eq!(
            reopened
                .read(None)
                .unwrap()
                .explain(seed.assertion)
                .unwrap(),
            explained
        );
    }
}

// 6. A schema commit whose publication was lost -------------------------------------------------

/// The real provider at the kernel's port, answering the first `resume` it is armed for with
/// `UnknownCommit` before delegating it: the decision was elected and never sent.
struct Lossy<S> {
    inner: S,
    lose: Rc<Cell<bool>>,
}

impl<S: ekr_store::RevisionLog> ekr_store::RevisionLog for Lossy<S> {
    fn preparation(
        &self,
        key: &PublicationCommandKey,
    ) -> Result<Option<PublicationPreparationV1>, StoreError> {
        self.inner.preparation(key)
    }
    fn prepare(
        &self,
        key: &PublicationCommandKey,
        input: ContentHash,
        decision: &Publication,
        previous: Option<&PublicationPreparationV1>,
    ) -> Result<PublicationPreparationV1, StoreError> {
        self.inner.prepare(key, input, decision, previous)
    }
    fn resume(&self, p: &PublicationPreparationV1) -> Result<ekr_store::Appended, StoreError> {
        if self.lose.replace(false) {
            return Err(StoreError::UnknownCommit);
        }
        self.inner.resume(p)
    }
    fn history(&self) -> Result<ekr_store::RetainedHistory, StoreError> {
        self.inner.history()
    }
    fn history_at(&self, r: RevisionNumber) -> Result<ekr_store::RetainedHistory, StoreError> {
        self.inner.history_at(r)
    }
    fn publish(&self, p: &Publication) -> Result<ekr_store::Appended, StoreError> {
        self.inner.publish(p)
    }
    fn seed_bytes(&self) -> Result<Option<Vec<u8>>, StoreError> {
        self.inner.seed_bytes()
    }
    fn fold(&self) -> Result<CanonicalGraph, StoreError> {
        self.inner.fold()
    }
    fn head(&self) -> Result<Option<Root>, StoreError> {
        self.inner.head()
    }
    fn replay(&self, r: RevisionNumber) -> Result<CanonicalGraph, StoreError> {
        self.inner.replay(r)
    }
}
impl<S: ekr_store::Initialize> ekr_store::Initialize for Lossy<S> {
    fn initialize(&self, p: &Publication) -> Result<ekr_store::Appended, StoreError> {
        self.inner.initialize(p)
    }
}
impl<S: ekr_store::ObjectStore> ekr_store::ObjectStore for Lossy<S> {
    fn put(
        &self,
        c: ekr_store::StorageClass,
        b: &[u8],
        t: Timestamp,
    ) -> Result<ekr_store::StoredObject, StoreError> {
        self.inner.put(c, b, t)
    }
    fn get(&self, h: &ContentHash) -> Result<Option<Vec<u8>>, StoreError> {
        self.inner.get(h)
    }
}

/// Proposes and validates a schema change through the real runtime, then commits it through a
/// port that loses the elected publication; a fresh runtime over the same store resumes that
/// exact commit, applies the new version, and reopens to the same state.
#[test]
fn a_schema_commit_lost_before_its_write_resumes_in_a_fresh_runtime_on_both_providers() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let seed = seeded();
        let kernel = open(directory.path(), file);
        kernel
            .seed(seed.document.clone(), || Timestamp::from_millis(10))
            .unwrap();
        let (observation, version) = (TypeId::mint(), SchemaVersionId::mint());
        let schema = transaction(
            vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
                observation,
                "Observation",
            )))],
            Some(version),
        );
        kernel
            .propose(&encode(&schema), context().operator, || {
                Timestamp::from_millis(20)
            })
            .unwrap();
        assert!(matches!(
            kernel
                .validate(schema.id, RevisionNumber::SEED, || Timestamp::from_millis(
                    21
                ))
                .unwrap(),
            ValidationCommandResult::Validated(_)
        ));
        drop(kernel);

        let lose = Rc::new(Cell::new(true));
        let lost = {
            let lose = lose.clone();
            let path = directory.path().to_owned();
            if file {
                Commit::over_with_authority(context(), v2(), move |authority: KernelAuthority| {
                    Ok(Lossy {
                        inner: ekr_store::FileStore::file(&path, TENANT, None)?.under(authority),
                        lose,
                    })
                })
                .unwrap()
                .commit(schema.id, context().operator, || Timestamp::from_millis(22))
                .map(|_| ())
            } else {
                Commit::over_with_authority(context(), v2(), move |authority: KernelAuthority| {
                    Ok(Lossy {
                        inner: ekr_store::SqliteStore::sqlite(
                            &path.join("state.db"),
                            TENANT,
                            None,
                        )?
                        .under(authority),
                        lose,
                    })
                })
                .unwrap()
                .commit(schema.id, context().operator, || Timestamp::from_millis(22))
                .map(|_| ())
            }
        };
        assert!(lost.is_err(), "the lossy port reported {lost:?}");
        assert!(!lose.get(), "the lossy port was reached");

        let resumed = open(directory.path(), file);
        assert_eq!(
            resumed.head().unwrap().unwrap().revision,
            RevisionNumber::SEED,
            "nothing was published"
        );
        let committed = resumed
            .commit(schema.id, context().operator, || {
                panic!("a resumed commit samples no clock")
            })
            .unwrap();
        let CommitCommandResult::Committed(receipt) = committed else {
            panic!("expected the elected commit, got {committed:?}");
        };
        assert_eq!(receipt.committed_at, Timestamp::from_millis(22));
        let head = resumed.snapshot().unwrap();
        assert_eq!(head.ontology.version().id, version);
        assert_eq!(
            head.ontology.version().created_at,
            Timestamp::from_millis(22)
        );
        assert!(head.ontology.node_type(observation).is_some());
        let expected = every_revision(&resumed);
        drop(resumed);
        assert_eq!(every_revision(&open(directory.path(), file)), expected);
    }
}

// 7. Two schema changes validated against one head ----------------------------------------------

/// Two schema changes validated against the same head, one version id between them: the first
/// commits, the second is stale, and revalidated against the new head it is refused as reusing a
/// version and as redeclaring a type; the store reopens with the same records.
#[test]
fn two_schema_changes_validated_against_one_head_commit_once_on_both_providers() {
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let kernel = open(directory.path(), file);
        kernel
            .seed(seeded().document, || Timestamp::from_millis(10))
            .unwrap();
        let (shared_type, shared_version) = (TypeId::mint(), SchemaVersionId::mint());
        let define = || {
            transaction(
                vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
                    shared_type,
                    "Observation",
                )))],
                Some(shared_version),
            )
        };
        let (first, second) = (define(), define());
        for (tx, at) in [(&first, 20), (&second, 30)] {
            kernel
                .propose(&encode(tx), context().operator, || {
                    Timestamp::from_millis(at)
                })
                .unwrap();
            assert!(matches!(
                kernel
                    .validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(
                        at + 1
                    ))
                    .unwrap(),
                ValidationCommandResult::Validated(_)
            ));
        }
        assert!(matches!(
            kernel
                .commit(first.id, context().operator, || Timestamp::from_millis(40))
                .unwrap(),
            CommitCommandResult::Committed(_)
        ));
        assert!(matches!(
            kernel
                .commit(second.id, context().operator, || Timestamp::from_millis(41))
                .unwrap(),
            CommitCommandResult::Stale(_)
        ));
        let again = GraphTransaction {
            id: TransactionId::mint(),
            ..second
        };
        let codes = submit(&kernel, &again, 50).expect_err("a reused version and type");
        assert_eq!(codes, vec!["identity-already-exists".to_owned()]);
        let reused = transaction(
            vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
                TypeId::mint(),
                "Other",
            )))],
            Some(shared_version),
        );
        assert_eq!(
            submit(&kernel, &reused, 60),
            Err(vec!["schema-version-reused".to_owned()])
        );
        let head = kernel.head().unwrap();
        let records = kernel.transactions().unwrap();
        drop(kernel);
        let reopened = open(directory.path(), file);
        assert_eq!(reopened.head().unwrap(), head);
        assert_eq!(reopened.transactions().unwrap(), records);
    }
}

// 8. A property redeclared twice in one transaction (characterisation) -------------------------

/// Two `ModifyProperty` of one property of one owner in one transaction. Adversary pass 2 measured
/// them admitted last-wins while data operations that compete this way are refused as
/// `conflicting-write`; the coordinator decided they are refused the same way, whether or not the
/// two declarations differ, since an identical second one has no effect. Rewritten by the
/// implementor in correction round 2 under the wave rule for a case a decision made wrong; it was
/// `a_property_redeclared_twice_in_one_transaction_is_admitted_last_wins`.
#[test]
fn a_property_redeclared_twice_in_one_transaction_is_refused_as_a_conflicting_write() {
    let world = Hierarchy::new();
    let note = PropertyId::mint();
    for second in [ValueType::String, ValueType::Integer] {
        let tx = GraphTransaction {
            id: TransactionId::mint(),
            proposer: world.proposer,
            operations: vec![
                modify(
                    world.base,
                    PropertyDefinition::new(note, "note", ValueType::Integer),
                ),
                modify(world.base, PropertyDefinition::new(note, "note", second)),
            ],
            evidence: BTreeSet::new(),
            schema_version: Some(SchemaVersionId::mint()),
        };
        let issues = Pipeline::schema_evolving(world.reviewer, BTreeSet::new())
            .validate(&GraphSnapshot::of(&world.graph), &tx)
            .expect_err("a property redeclared twice in one transaction");
        let codes: Vec<&str> = issues.iter().map(|issue| issue.code.as_str()).collect();
        assert_eq!(codes, vec!["conflicting-write"], "{issues:?}");
    }
}
