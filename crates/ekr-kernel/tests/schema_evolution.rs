//! Schema changes admitted by the kernel: `story:schema-evolution-transactions`, parts B and C.
//!
//! The first acceptance bullet, at the validation boundary: a transaction can add a node type, add
//! an edge type, and add or redeclare a property on an existing type, producing a new schema
//! version with lineage; the ontology validators check it against canonical state, and a change
//! that state would violate is refused with a named issue.
//!
//! Every case here runs the pipeline of validation profile v2 (`ekr.p2-deterministic/1`), except
//! the ones that say they run v1, which still refuses the three kinds. The both-provider replay
//! half lives in `schema_evolution_replay.rs`.
//!
//! The fixture is the runtime's own vocabulary: `Decision` nodes and a `depends_on` edge.

use std::collections::BTreeSet;

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{
    AgentId, AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId,
    RevisionNumber, SchemaVersionId, Timestamp, TransactionId, TypeId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue,
    Confidence, Edge, Evidence, EvidenceSource, GraphRoot, GraphSnapshot, Node, Object, Predicate,
    RetractionReason, Space, Subject, TemporalRange, TransactionTime,
};
use ekr_kernel::{
    GraphOperation, GraphTransaction, NodeDraft, Pipeline, PropertyModification,
    TransactionDocument, ValidationIssue, ValidatorName,
};
use ekr_ontology::{
    Cardinality, EdgeType, NodeType, Ontology, OntologyDocument, PropertyDefinition, SchemaVersion,
    Value, ValueType,
};

/// Canonical state at revision 3: two decisions, one `depends_on` edge, and one active property
/// assertion on `note`, a property no node holds a value of.
struct World {
    graph: CanonicalGraph,
    root_id: GraphRootId,
    seed_version: SchemaVersionId,
    decision: TypeId,
    depends_on: TypeId,
    title: PropertyId,
    tags: PropertyId,
    note: PropertyId,
    note_assertion: AssertionId,
    proposer: AgentId,
    reviewer: AgentId,
}

impl World {
    fn new() -> Self {
        let root_id = GraphRootId::mint();
        let seed_version = SchemaVersionId::mint();
        let (decision, depends_on) = (TypeId::mint(), TypeId::mint());
        let (title, tags, note) = (PropertyId::mint(), PropertyId::mint(), PropertyId::mint());
        let (open, decided) = (NodeId::mint(), NodeId::mint());
        let (note_assertion, evidence_id) = (AssertionId::mint(), EvidenceId::mint());
        let (proposer, reviewer) = (AgentId::mint(), AgentId::mint());

        let mut decision_type = NodeType::new(decision, "Decision");
        let mut required_title = PropertyDefinition::new(title, "title", ValueType::String);
        required_title.required = true;
        decision_type.properties.insert(title, required_title);
        let mut many_tags = PropertyDefinition::new(tags, "tags", ValueType::String);
        many_tags.cardinality = Cardinality::Many;
        decision_type.properties.insert(tags, many_tags);
        decision_type.properties.insert(
            note,
            PropertyDefinition::new(note, "note", ValueType::String),
        );

        let mut depends_on_type = EdgeType::new(depends_on, "depends_on");
        depends_on_type.source_types = [decision].into_iter().collect();
        depends_on_type.target_types = [decision].into_iter().collect();

        let ontology = Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(seed_version, Timestamp::EPOCH),
            node_types: vec![decision_type],
            edge_types: vec![depends_on_type],
        })
        .expect("the fixture ontology coheres");

        let text = |text: &str| CanonicalValue::String(text.to_owned());
        let mut open_node = Node::new(open, root_id, decision, "Adopt the eventlog store");
        open_node
            .properties
            .insert(title, vec![text("Adopt the eventlog store")]);
        open_node
            .properties
            .insert(tags, vec![text("storage"), text("phase-1")]);
        let mut decided_node = Node::new(decided, root_id, decision, "Hash canonical state only");
        decided_node
            .properties
            .insert(title, vec![text("Hash canonical state only")]);

        let edge_id = EdgeId::mint();
        let edge = Edge::new(
            edge_id,
            root_id,
            depends_on,
            CanonicalRef::new(open),
            CanonicalRef::new(decided),
        );
        let evidence = Evidence {
            id: evidence_id,
            source: EvidenceSource::Document {
                document_id: "docs/roadmap.md".to_owned(),
                section: Some("Phase 5".to_owned()),
            },
            content_hash: ContentHash::of(&"the roadmap's phase 5".to_owned()),
            extracted_by: proposer,
            observed_at: Timestamp::EPOCH,
            confidence: Confidence::CERTAIN,
        };
        let assertion = Assertion {
            id: note_assertion,
            root_id,
            subject: Subject::Node(CanonicalRef::new(open)),
            predicate: Predicate::Property(note),
            object: Object::Value(text("reviewed in the phase 5 planning")),
            evidence: [CanonicalRef::new(evidence_id)].into_iter().collect(),
            proposed_by: proposer,
            lifecycle: AssertionLifecycle::Active,
            assessment: Assessment::Accepted {
                validators: [reviewer].into_iter().collect(),
            },
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        };

        Self {
            graph: CanonicalGraph {
                root: GraphRoot {
                    id: root_id,
                    space: Space::Canonical,
                    schema_version_id: seed_version,
                    parent: None,
                    created_at: Timestamp::EPOCH,
                },
                revision: RevisionNumber::new(3),
                ontology,
                nodes: [(open, open_node), (decided, decided_node)]
                    .into_iter()
                    .collect(),
                edges: [(edge_id, edge)].into_iter().collect(),
                assertions: [(note_assertion, assertion)].into_iter().collect(),
                evidence: [(evidence_id, evidence)].into_iter().collect(),
            },
            root_id,
            seed_version,
            decision,
            depends_on,
            title,
            tags,
            note,
            note_assertion,
            proposer,
            reviewer,
        }
    }

    fn proposal(
        &self,
        operations: Vec<GraphOperation>,
        schema_version: Option<SchemaVersionId>,
    ) -> GraphTransaction {
        GraphTransaction {
            id: TransactionId::mint(),
            proposer: self.proposer,
            operations,
            evidence: BTreeSet::new(),
            schema_version,
        }
    }

    /// The v2 pipeline, with no version on the lineage but the snapshot's own.
    fn v2(&self) -> Pipeline {
        Pipeline::schema_evolving(self.reviewer, BTreeSet::new())
    }

    fn validate(
        &self,
        pipeline: &Pipeline,
        operations: Vec<GraphOperation>,
        schema_version: Option<SchemaVersionId>,
    ) -> Result<(), Vec<ValidationIssue>> {
        pipeline
            .validate(
                &GraphSnapshot::of(&self.graph),
                &self.proposal(operations, schema_version),
            )
            .map(|_| ())
    }

    fn refuse(&self, operations: Vec<GraphOperation>) -> Vec<ValidationIssue> {
        self.validate(&self.v2(), operations, Some(SchemaVersionId::mint()))
            .expect_err("expected a refusal")
    }

    fn modify(&self, owner: TypeId, property: PropertyDefinition) -> GraphOperation {
        GraphOperation::ModifyProperty(PropertyModification {
            owner: Some(owner),
            property,
        })
    }

    fn declared(&self, property: PropertyId) -> PropertyDefinition {
        self.graph
            .ontology
            .properties_of(self.decision)
            .get(&property)
            .map(|declared| (*declared).clone())
            .expect("a Decision property")
    }

    fn draft(&self) -> NodeDraft {
        NodeDraft {
            id: NodeId::mint(),
            root_id: self.root_id,
            type_id: self.decision,
            canonical_name: "Replay across schema versions".to_owned(),
            properties: [(
                self.title,
                vec![Value::String("Replay across schema versions".to_owned())],
            )]
            .into_iter()
            .collect(),
        }
    }
}

fn codes(issues: &[ValidationIssue]) -> Vec<&str> {
    issues.iter().map(|issue| issue.code.as_str()).collect()
}

fn validators(issues: &[ValidationIssue]) -> Vec<ValidatorName> {
    let mut named: Vec<ValidatorName> = issues.iter().map(|issue| issue.validator).collect();
    named.dedup();
    named
}

fn observation() -> NodeType {
    let mut declared = NodeType::new(TypeId::mint(), "Observation");
    let summary = PropertyId::mint();
    declared.properties.insert(
        summary,
        PropertyDefinition::new(summary, "summary", ValueType::String),
    );
    declared
}

// Admission -------------------------------------------------------------------------------------

/// Each of the three kinds, alone and together, validates under v2.
#[test]
fn each_schema_change_kind_validates_under_profile_two() {
    let world = World::new();
    let fresh = PropertyId::mint();
    let mut renamed = world.declared(world.tags);
    renamed.name = "labels".to_owned();
    let mut observes = EdgeType::new(TypeId::mint(), "observes");
    observes.source_types = [world.decision].into_iter().collect();
    observes.target_types = [world.decision].into_iter().collect();

    let cases: Vec<(&str, Vec<GraphOperation>)> = vec![
        (
            "a node type",
            vec![GraphOperation::DefineNodeType(Box::new(observation()))],
        ),
        (
            "an edge type",
            vec![GraphOperation::DefineEdgeType(Box::new(observes.clone()))],
        ),
        (
            "a property added to an existing type",
            vec![world.modify(
                world.decision,
                PropertyDefinition::new(fresh, "rationale", ValueType::String),
            )],
        ),
        (
            "a property redeclared on an existing type",
            vec![world.modify(world.decision, renamed.clone())],
        ),
        (
            "a property added to an existing edge type",
            vec![world.modify(
                world.depends_on,
                PropertyDefinition::new(PropertyId::mint(), "strength", ValueType::Integer),
            )],
        ),
        (
            "all three together",
            vec![
                GraphOperation::DefineNodeType(Box::new(observation())),
                GraphOperation::DefineEdgeType(Box::new(observes)),
                world.modify(world.decision, renamed),
            ],
        ),
    ];
    for (what, operations) in cases {
        assert_eq!(
            world.validate(&world.v2(), operations, Some(SchemaVersionId::mint())),
            Ok(()),
            "{what}"
        );
    }
}

/// Under v1 the three kinds are still refused, and the refusal is exactly the retained P1 one.
#[test]
fn profile_one_still_refuses_the_three_schema_kinds() {
    let world = World::new();
    let v1 = Pipeline::deterministic(world.reviewer);
    for (name, operation) in [
        (
            "DefineNodeType",
            GraphOperation::DefineNodeType(Box::new(observation())),
        ),
        (
            "ModifyProperty",
            world.modify(
                world.decision,
                PropertyDefinition::new(PropertyId::mint(), "rationale", ValueType::String),
            ),
        ),
    ] {
        for version in [None, Some(SchemaVersionId::mint())] {
            let issues = world
                .validate(&v1, vec![operation.clone()], version)
                .expect_err("v1 admits no schema change");
            assert_eq!(codes(&issues), vec!["unsupported-operation"], "{name}");
            assert_eq!(issues[0].message, format!("{name} is not supported in P1"));
        }
    }
}

// The version field ------------------------------------------------------------------------------

/// A schema change names the version it produces, and nothing else names one.
#[test]
fn the_version_id_is_required_exactly_when_a_schema_change_is_present() {
    let world = World::new();
    let missing = world
        .validate(
            &world.v2(),
            vec![GraphOperation::DefineNodeType(Box::new(observation()))],
            None,
        )
        .expect_err("a schema change without a version id");
    assert_eq!(codes(&missing), vec!["schema-version-missing"]);
    assert_eq!(validators(&missing), vec![ValidatorName::Structural]);

    for pipeline in [world.v2(), Pipeline::deterministic(world.reviewer)] {
        let stray = world
            .validate(
                &pipeline,
                vec![GraphOperation::CreateNode(world.draft())],
                Some(SchemaVersionId::mint()),
            )
            .expect_err("a version id on a transaction with no schema change");
        assert_eq!(codes(&stray), vec!["schema-version-without-schema-change"]);
        assert_eq!(validators(&stray), vec![ValidatorName::Structural]);
    }

    // And a data transaction without one is unaffected.
    assert_eq!(
        world.validate(
            &world.v2(),
            vec![GraphOperation::CreateNode(world.draft())],
            None
        ),
        Ok(())
    );
}

/// A schema change travels alone: mixed with any other kind it is refused.
#[test]
fn a_schema_change_mixed_with_another_kind_is_refused() {
    let world = World::new();
    for other in [
        GraphOperation::CreateNode(world.draft()),
        GraphOperation::DeleteEdge(*world.graph.edges.keys().next().unwrap()),
        GraphOperation::RetractAssertion(ekr_kernel::Retraction {
            assertion: world.note_assertion,
            reason: RetractionReason::new("superseded by the schema change"),
        }),
    ] {
        let issues = world.refuse(vec![
            GraphOperation::DefineNodeType(Box::new(observation())),
            other,
        ]);
        assert_eq!(codes(&issues), vec!["mixed-schema-transaction"]);
        assert_eq!(validators(&issues), vec![ValidatorName::Structural]);
    }
}

// What `Ontology::evolve` refuses ---------------------------------------------------------------

/// Each `EvolveError` the kernel can reach arrives as an issue carrying the ontology's own code,
/// raised by the ontology-constraint validator.
#[test]
fn evolve_refusals_arrive_as_ontology_issues_with_the_ontologys_codes() {
    let world = World::new();
    let mut orphan = NodeType::new(TypeId::mint(), "Orphan");
    orphan.parents.insert(TypeId::mint());
    let cases: Vec<(&str, Vec<GraphOperation>)> = vec![
        (
            "unknown-property-owner",
            vec![world.modify(
                TypeId::mint(),
                PropertyDefinition::new(PropertyId::mint(), "stray", ValueType::String),
            )],
        ),
        (
            "incoherent-schema",
            vec![GraphOperation::DefineNodeType(Box::new(orphan))],
        ),
        (
            "schema-change-without-effect",
            vec![world.modify(world.decision, world.declared(world.title))],
        ),
    ];
    for (code, operations) in cases {
        let issues = world.refuse(operations);
        assert_eq!(codes(&issues), vec![code]);
        assert_eq!(validators(&issues), vec![ValidatorName::OntologyConstraint]);
        assert!(
            issues[0].message.starts_with(code),
            "the message leads with the code: {}",
            issues[0].message
        );
    }
}

/// A new version id is new on the whole lineage, not only to the prior version and its parent.
#[test]
fn a_version_id_already_on_the_lineage_is_refused() {
    let world = World::new();
    let change = || vec![GraphOperation::DefineNodeType(Box::new(observation()))];

    // The prior version's own id: `evolve` sees it.
    let own = world
        .validate(&world.v2(), change(), Some(world.seed_version))
        .expect_err("the prior version's id");
    assert_eq!(codes(&own), vec!["schema-version-reused"]);
    assert_eq!(validators(&own), vec![ValidatorName::OntologyConstraint]);

    // An older ancestor: only the lineage the kernel holds can see it.
    let ancestor = SchemaVersionId::mint();
    let pipeline = Pipeline::schema_evolving(world.reviewer, [ancestor].into_iter().collect());
    let older = world
        .validate(&pipeline, change(), Some(ancestor))
        .expect_err("an older version's id");
    assert_eq!(codes(&older), vec!["schema-version-reused"]);
    assert_eq!(validators(&older), vec![ValidatorName::OntologyConstraint]);

    assert_eq!(
        world.validate(&pipeline, change(), Some(SchemaVersionId::mint())),
        Ok(())
    );
}

// What canonical state refuses -------------------------------------------------------------------

/// A change the canonical state would violate is refused with the incompatibility's own code.
#[test]
fn a_change_canonical_state_would_violate_is_refused_with_a_named_issue() {
    let world = World::new();

    let mut single_tags = world.declared(world.tags);
    single_tags.cardinality = Cardinality::One;
    let mut integer_title = world.declared(world.title);
    integer_title.value_type = ValueType::Integer;
    let mut constrained_title = world.declared(world.title);
    constrained_title.constraints = vec!["non-empty".to_owned()];
    let mut required_new = PropertyDefinition::new(PropertyId::mint(), "owner", ValueType::String);
    required_new.required = true;
    let mut required_on_edge =
        PropertyDefinition::new(PropertyId::mint(), "strength", ValueType::Integer);
    required_on_edge.required = true;

    let cases: Vec<(&str, GraphOperation)> = vec![
        (
            "cardinality-narrowed",
            world.modify(world.decision, single_tags),
        ),
        (
            "value-kind-not-admitted",
            world.modify(world.decision, integer_title),
        ),
        (
            "constraint-changed",
            world.modify(world.decision, constrained_title),
        ),
        (
            "required-property-missing",
            world.modify(world.decision, required_new),
        ),
        (
            "required-property-missing",
            world.modify(world.depends_on, required_on_edge),
        ),
    ];
    for (code, operation) in cases {
        let issues = world.refuse(vec![operation]);
        assert_eq!(codes(&issues), vec![code]);
        assert_eq!(validators(&issues), vec![ValidatorName::OntologyConstraint]);
        assert!(issues[0].message.starts_with(code), "{}", issues[0].message);
    }
}

/// `value_kinds` includes the objects of active property assertions: `note` has no held value,
/// only an assertion whose object is a string, and redeclaring it an integer breaks that object.
#[test]
fn a_value_type_change_is_refused_when_an_active_assertion_object_would_break() {
    let world = World::new();
    let mut integer_note = world.declared(world.note);
    integer_note.value_type = ValueType::Integer;
    let issues = world.refuse(vec![world.modify(world.decision, integer_note.clone())]);
    assert_eq!(codes(&issues), vec!["value-kind-not-admitted"]);

    // A retracted assertion is not active, and holds nothing back.
    let mut retracted = World::new();
    let assertion = retracted
        .graph
        .assertions
        .get_mut(&retracted.note_assertion)
        .unwrap();
    assertion.lifecycle = AssertionLifecycle::Retracted {
        at_revision: RevisionNumber::new(2),
        reason: RetractionReason::new("withdrawn"),
    };
    let mut integer_note = retracted.declared(retracted.note);
    integer_note.value_type = ValueType::Integer;
    assert_eq!(
        retracted.validate(
            &retracted.v2(),
            vec![retracted.modify(retracted.decision, integer_note)],
            Some(SchemaVersionId::mint()),
        ),
        Ok(())
    );
}

/// `min_values` counts held values only: an assertion on `note` does not make `note` held, so
/// requiring it is refused.
#[test]
fn requiring_a_property_held_only_through_assertions_is_refused() {
    let world = World::new();
    let mut required_note = world.declared(world.note);
    required_note.required = true;
    let issues = world.refuse(vec![world.modify(world.decision, required_note)]);
    assert_eq!(codes(&issues), vec!["required-property-missing"]);

    // Redeclaring a property every instance holds, changing only its name, validates.
    let mut required_title = world.declared(world.title);
    required_title.name = "headline".to_owned();
    assert_eq!(
        world.validate(
            &world.v2(),
            vec![world.modify(world.decision, required_title)],
            Some(SchemaVersionId::mint()),
        ),
        Ok(())
    );
}

/// Two definitions of one type id in one transaction are refused under v2 as they are under v1,
/// as `duplicate-identity`: the type-level half of correction 2's conflicting-write rule.
#[test]
fn a_type_defined_twice_in_one_transaction_is_refused_under_profile_two() {
    let world = World::new();
    let declared = observation();
    let mut edge = EdgeType::new(declared.id, "observes");
    edge.source_types = [world.decision].into_iter().collect();
    edge.target_types = [world.decision].into_iter().collect();
    for second in [
        GraphOperation::DefineNodeType(Box::new(declared.clone())),
        GraphOperation::DefineEdgeType(Box::new(edge)),
    ] {
        let issues = world.refuse(vec![
            GraphOperation::DefineNodeType(Box::new(declared.clone())),
            second,
        ]);
        assert_eq!(codes(&issues), vec!["duplicate-identity"]);
        assert_eq!(validators(&issues), vec![ValidatorName::Structural]);
    }
}

/// Under v2 a `ModifyProperty` names its owner: the ownerless P1 shape still parses, for v1 stores
/// that retain it, and v2 refuses it by name.
#[test]
fn an_ownerless_modify_property_is_refused_under_profile_two() {
    let world = World::new();
    let ownerless = GraphOperation::ModifyProperty(PropertyModification {
        owner: None,
        property: PropertyDefinition::new(PropertyId::mint(), "rationale", ValueType::String),
    });
    let issues = world.refuse(vec![ownerless.clone()]);
    assert_eq!(codes(&issues), vec!["modify-property-without-owner"]);
    assert_eq!(validators(&issues), vec![ValidatorName::Structural]);

    // And v1 refuses it exactly as P1 did.
    let v1 = Pipeline::deterministic(world.reviewer);
    let issues = world
        .validate(&v1, vec![ownerless], None)
        .expect_err("v1 admits no schema change");
    assert_eq!(codes(&issues), vec!["unsupported-operation"]);
    assert_eq!(issues[0].message, "ModifyProperty is not supported in P1");
}

// Encoding and document -------------------------------------------------------------------------

/// A transaction without a version id encodes exactly as the four fields did before the field
/// existed; one with a version id encodes differently for each id.
#[test]
fn a_transaction_without_a_version_id_encodes_byte_identically_to_before() {
    let world = World::new();
    let proposal = world.proposal(vec![GraphOperation::CreateNode(world.draft())], None);
    let canonical = GraphTransaction::<CanonicalValue>::try_from(proposal.clone()).unwrap();
    let mut before = Encoder::new();
    canonical.id.encode(&mut before);
    canonical.proposer.encode(&mut before);
    before.list(canonical.operations.iter());
    before.set(canonical.evidence.iter());
    assert_eq!(canonical.canonical_bytes(), before.finish());

    let (one, two) = (SchemaVersionId::mint(), SchemaVersionId::mint());
    let with = |version| {
        let mut changed = canonical.clone();
        changed.schema_version = Some(version);
        changed.canonical_bytes()
    };
    assert_ne!(with(one), canonical.canonical_bytes());
    assert_ne!(with(one), with(two));
}

/// The `ModifyProperty` owner reaches the encoding.
#[test]
fn the_modify_property_owner_reaches_the_encoding() {
    let property = PropertyDefinition::new(PropertyId::mint(), "note", ValueType::String);
    let encode = |owner| {
        GraphOperation::<CanonicalValue>::ModifyProperty(PropertyModification {
            owner,
            property: property.clone(),
        })
        .canonical_bytes()
    };
    assert_ne!(encode(Some(TypeId::mint())), encode(Some(TypeId::mint())));
    // The P1 shape, with no owner, encodes as P1 did: the variant, then the bare declaration. An
    // owner is written as a tagged `Some`, so the two shapes cannot share bytes.
    let mut p1 = Encoder::new();
    p1.variant(8);
    property.encode(&mut p1);
    assert_eq!(encode(None), p1.finish());
    assert_ne!(encode(Some(TypeId::mint()))[..10], encode(None)[..10]);
}

const TX: &str = "00000000-0000-4000-8000-0000000000a1";
const AGENT: &str = "00000000-0000-4000-8000-0000000000a2";
const TYPE: &str = "00000000-0000-4000-8000-0000000000a3";
const PROPERTY: &str = "00000000-0000-4000-8000-0000000000a4";
const VERSION: &str = "00000000-0000-4000-8000-0000000000a5";

fn document(extra: &str, operation: &str) -> String {
    format!(
        "format: ekr.transaction-document/1\ntransaction:\n  id: {TX}\n  proposer: {AGENT}\n  \
         operations:\n  - {operation}\n  evidence: []\n{extra}"
    )
}

/// `ekr.transaction-document/1` takes one optional `schema_version` key, and a document without
/// it reads as before.
#[test]
fn the_transaction_document_takes_one_optional_version_key() {
    let modify = format!(
        "!ModifyProperty {{owner: {TYPE}, property: {{id: {PROPERTY}, name: note, value_type: \
         {{value_kind: String}}}}}}"
    );
    let without = TransactionDocument::parse(document("", &modify).as_bytes()).unwrap();
    assert_eq!(without.transaction().schema_version, None);

    let with = TransactionDocument::parse(
        document(&format!("  schema_version: {VERSION}\n"), &modify).as_bytes(),
    )
    .unwrap();
    assert_eq!(
        with.transaction().schema_version,
        Some(VERSION.parse().unwrap())
    );
    let GraphOperation::ModifyProperty(modification) = &with.transaction().operations[0] else {
        panic!("a ModifyProperty")
    };
    assert_eq!(modification.owner, Some(TYPE.parse::<TypeId>().unwrap()));
    assert_eq!(modification.property.id, PROPERTY.parse().unwrap());

    // The ownerless P1 shape still reads, as a modification with no owner: v1 stores retain it
    // (correction 1; `base_era_v1_replay.rs`), and v2 refuses it at validation, not at parse.
    let ownerless = format!(
        "!ModifyProperty {{id: {PROPERTY}, name: note, value_type: {{value_kind: String}}}}"
    );
    let p1 = TransactionDocument::parse(document("", &ownerless).as_bytes()).unwrap();
    let GraphOperation::ModifyProperty(modification) = &p1.transaction().operations[0] else {
        panic!("a ModifyProperty")
    };
    assert_eq!(modification.owner, None);
    // And a key that is not the version is still refused.
    assert!(TransactionDocument::parse(
        document(&format!("  schema_versions: {VERSION}\n"), &modify).as_bytes()
    )
    .is_err());
}

/// A data transaction written before the field existed still parses, with no version.
#[test]
fn a_data_document_without_the_key_reads_as_before() {
    let node = format!(
        "!CreateNode {{id: {PROPERTY}, root_id: {TYPE}, type_id: {TYPE}, canonical_name: x, \
         properties: {{}}}}"
    );
    let parsed = TransactionDocument::parse(document("", &node).as_bytes()).unwrap();
    assert_eq!(parsed.transaction().schema_version, None);
}
