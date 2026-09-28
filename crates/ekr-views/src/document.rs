//! The pure half: a [`LoadedRevision`] to the bytes of `ekr.graph-projection/1`.
//!
//! Every type here is one of `views.yaml`'s, its fields in the order the specification declares
//! them, because serde writes a struct's fields in declaration order — rule (2). An `Option` is
//! skipped when `None` and written when `Some`; a list or map is always written — rule (4).
//! Every array is built sorted where rule (1) sorts it and in store order where it keeps it;
//! every map is a [`BTreeMap`] over text, which ascends by that text. `serde_json::to_vec` writes
//! no insignificant whitespace, an integer as an integer and a string one way — rule (3).

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{EdgeId, NodeId, PropertyId, TypeId};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalDependency, CanonicalValue, Edge,
    EvidenceKind, Object, Predicate, Subject,
};
use ekr_ontology::{Ontology, PropertyDefinition};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{GraphProjected, LoadedRevision, ProjectError, Rendered, FORMAT};

/// Ascending by the text of each id, which rule (1) orders by.
pub(crate) fn sorted<T: ToString>(ids: impl IntoIterator<Item = T>) -> Vec<String> {
    let mut texts: Vec<String> = ids.into_iter().map(|id| id.to_string()).collect();
    texts.sort();
    texts.dedup();
    texts
}

#[derive(Serialize)]
struct GraphProjectionV1 {
    meta: ProjectionMeta,
    ontology: ProjectedOntology,
    nodes: Vec<ProjectedNode>,
    edges: Vec<ProjectedEdge>,
    evidence: Vec<ProjectedEvidence>,
    schema: ProjectedSchema,
}

#[derive(Serialize)]
struct ProjectionMeta {
    format: &'static str,
    revision: u64,
    node_count: u64,
    edge_count: u64,
    assertion_count: u64,
    evidence_count: u64,
}

#[derive(Serialize)]
#[serde(tag = "kind", content = "value")]
pub(crate) enum ProjectedValue {
    String(String),
    Boolean(bool),
    Integer(i64),
    Decimal(String),
    Timestamp(i64),
    Duration(i64),
    NodeRef(String),
    Enum(String),
    List(Vec<ProjectedValue>),
    Record(BTreeMap<String, ProjectedValue>),
}

impl From<&CanonicalValue> for ProjectedValue {
    fn from(value: &CanonicalValue) -> Self {
        match value {
            CanonicalValue::String(text) => Self::String(text.clone()),
            CanonicalValue::Boolean(truth) => Self::Boolean(*truth),
            CanonicalValue::Integer(number) => Self::Integer(*number),
            CanonicalValue::Decimal(text) => Self::Decimal(text.clone()),
            CanonicalValue::Timestamp(at) => Self::Timestamp(at.millis()),
            CanonicalValue::Duration(length) => Self::Duration(*length),
            CanonicalValue::NodeRef(node) => Self::NodeRef(node.id().to_string()),
            CanonicalValue::Enum(variant) => Self::Enum(variant.clone()),
            CanonicalValue::List(items) => Self::List(items.iter().map(Self::from).collect()),
            CanonicalValue::Record(fields) => Self::Record(
                fields
                    .iter()
                    .map(|(name, value)| (name.clone(), Self::from(value)))
                    .collect(),
            ),
        }
    }
}

/// `ekr.graph.AssessmentProjection`: exactly the payload its kind carries.
#[derive(Serialize)]
struct ProjectedAssessment {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    completed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    validators: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    issues: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    competing_assertions: Option<Vec<String>>,
}

#[derive(Serialize)]
struct ProjectedLifecycle {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    at_revision: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    by: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    effective_from: Option<i64>,
}

#[derive(Serialize)]
pub(crate) struct ProjectedAssertion {
    id: String,
    predicate_kind: &'static str,
    predicate: String,
    object_kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    object_value: Option<ProjectedValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    object_ref: Option<String>,
    assessment: ProjectedAssessment,
    lifecycle: ProjectedLifecycle,
    #[serde(skip_serializing_if = "Option::is_none")]
    valid_from: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    valid_to: Option<i64>,
    recorded_from: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    recorded_to: Option<i64>,
    evidence: Vec<String>,
}

#[derive(Serialize, PartialEq, Eq)]
struct ProjectedProperty {
    id: String,
    name: String,
    value_kind: String,
}

#[derive(Serialize)]
struct ProjectedNodeType {
    id: String,
    name: String,
    properties: Vec<String>,
    assertions: Vec<ProjectedAssertion>,
}

#[derive(Serialize)]
struct ProjectedEdgeType {
    id: String,
    name: String,
    source_types: Vec<String>,
    target_types: Vec<String>,
    properties: Vec<String>,
    assertions: Vec<ProjectedAssertion>,
}

#[derive(Serialize)]
pub(crate) struct ProjectedOntology {
    node_types: Vec<ProjectedNodeType>,
    edge_types: Vec<ProjectedEdgeType>,
    properties: Vec<ProjectedProperty>,
}

#[derive(Serialize)]
struct ProjectedNode {
    id: String,
    name: String,
    #[serde(rename = "type")]
    type_id: String,
    aliases: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<String>,
    props: BTreeMap<String, Vec<ProjectedValue>>,
    assertions: Vec<ProjectedAssertion>,
}

#[derive(Serialize)]
pub(crate) struct ProjectedEdge {
    pub(crate) id: String,
    pub(crate) source: String,
    pub(crate) target: String,
    #[serde(rename = "type")]
    pub(crate) type_id: String,
    pub(crate) props: BTreeMap<String, Vec<ProjectedValue>>,
    pub(crate) assertions: Vec<ProjectedAssertion>,
}

impl ProjectedEdge {
    /// `edge` with the assertions `about` it, which rule (1) orders by id.
    pub(crate) fn of(edge: &Edge, about: Option<&Vec<&Assertion>>) -> Self {
        Self {
            id: edge.id.to_string(),
            source: edge.source.id().to_string(),
            target: edge.target.id().to_string(),
            type_id: edge.type_id.to_string(),
            props: props(&edge.properties),
            assertions: listed_under(about),
        }
    }
}

/// The projected assertions `claims`, ascending by id.
pub(crate) fn listed_under(claims: Option<&Vec<&Assertion>>) -> Vec<ProjectedAssertion> {
    let mut projected: Vec<ProjectedAssertion> = claims
        .into_iter()
        .flatten()
        .map(|claim| assertion(claim))
        .collect();
    projected.sort_by(|a, b| a.id.cmp(&b.id));
    projected
}

#[derive(Serialize)]
struct ProjectedEvidence {
    id: String,
    kind: &'static str,
    locator: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    section: Option<String>,
    content_hash: String,
    observed_at: i64,
    confidence_bp: u16,
    retained: bool,
}

#[derive(Serialize)]
struct ProjectedSchemaVersion {
    number: u64,
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent: Option<String>,
    revision: u64,
    added: Vec<String>,
}

#[derive(Serialize)]
struct ProjectedRevision {
    number: u64,
    committed_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    transaction_id: Option<String>,
    schema_version: String,
}

#[derive(Serialize)]
struct ProjectedSchema {
    versions: Vec<ProjectedSchemaVersion>,
    revisions: Vec<ProjectedRevision>,
}

/// The `ontology.properties` entry for one declaration: all of it the format carries.
fn projected_property(property: &PropertyDefinition) -> ProjectedProperty {
    ProjectedProperty {
        id: property.id.to_string(),
        name: property.name.clone(),
        value_kind: property.value_type.kind().to_string(),
    }
}

const fn evidence_kind(kind: EvidenceKind) -> &'static str {
    match kind {
        EvidenceKind::Url => "Url",
        EvidenceKind::Document => "Document",
        EvidenceKind::DatabaseRecord => "DatabaseRecord",
        EvidenceKind::GraphAssertion => "GraphAssertion",
        EvidenceKind::Observation => "Observation",
        EvidenceKind::HumanStatement => "HumanStatement",
    }
}

fn assessment(assessment: &Assessment) -> ProjectedAssessment {
    let mut projected = ProjectedAssessment {
        kind: assessment.name(),
        completed: None,
        required: None,
        validators: None,
        issues: None,
        competing_assertions: None,
    };
    match assessment {
        Assessment::Proposed => {}
        Assessment::Validating {
            completed,
            required,
        } => {
            projected.completed = Some(*completed);
            projected.required = Some(*required);
        }
        Assessment::Accepted { validators } => projected.validators = Some(sorted(validators)),
        Assessment::Rejected { issues } => {
            projected.issues = Some(issues.iter().map(ToString::to_string).collect());
        }
        Assessment::Disputed {
            competing_assertions,
        } => {
            projected.competing_assertions = Some(
                competing_assertions
                    .iter()
                    .map(|competitor| competitor.id().to_string())
                    .collect(),
            );
        }
    }
    projected
}

fn lifecycle(lifecycle: &AssertionLifecycle) -> ProjectedLifecycle {
    let mut projected = ProjectedLifecycle {
        kind: lifecycle.name(),
        at_revision: None,
        reason: None,
        by: None,
        effective_from: None,
    };
    match lifecycle {
        AssertionLifecycle::Active => {}
        AssertionLifecycle::Retracted {
            at_revision,
            reason,
        } => {
            projected.at_revision = Some(at_revision.get());
            projected.reason = Some(reason.as_str().to_owned());
        }
        AssertionLifecycle::Superseded {
            by,
            at_revision,
            effective_from,
        } => {
            projected.at_revision = Some(at_revision.get());
            projected.by = Some(by.id().to_string());
            projected.effective_from = Some(effective_from.millis());
        }
    }
    projected
}

pub(crate) fn assertion(assertion: &Assertion) -> ProjectedAssertion {
    let (predicate_kind, predicate) = match assertion.predicate {
        Predicate::Property(property) => ("Property", property.to_string()),
        Predicate::Relation(type_id) => ("Relation", type_id.to_string()),
    };
    let (object_kind, object_value, object_ref) = match &assertion.object {
        Object::Value(value) => ("Value", Some(ProjectedValue::from(value)), None),
        Object::Node(node) => ("Node", None, Some(node.id().to_string())),
        Object::Type(type_id) => ("Type", None, Some(type_id.to_string())),
    };
    ProjectedAssertion {
        id: assertion.id.to_string(),
        predicate_kind,
        predicate,
        object_kind,
        object_value,
        object_ref,
        assessment: self::assessment(&assertion.assessment),
        lifecycle: self::lifecycle(&assertion.lifecycle),
        valid_from: assertion.valid_time.from.map(|at| at.millis()),
        valid_to: assertion.valid_time.to.map(|at| at.millis()),
        recorded_from: assertion.transaction_time.recorded_from.millis(),
        recorded_to: assertion.transaction_time.recorded_to.map(|at| at.millis()),
        evidence: sorted(assertion.evidence.iter().map(CanonicalDependency::id)),
    }
}

pub(crate) fn props(
    properties: &BTreeMap<PropertyId, Vec<CanonicalValue>>,
) -> BTreeMap<String, Vec<ProjectedValue>> {
    properties
        .iter()
        .map(|(property, values)| {
            (
                property.to_string(),
                values.iter().map(ProjectedValue::from).collect(),
            )
        })
        .collect()
}

/// Every type, edge-type and property id `ontology` declares.
fn declared(ontology: &Ontology) -> BTreeSet<String> {
    let document = ontology.to_document();
    let mut ids = BTreeSet::new();
    for node_type in &document.node_types {
        ids.insert(node_type.id.to_string());
        ids.extend(node_type.properties.keys().map(ToString::to_string));
    }
    for edge_type in &document.edge_types {
        ids.insert(edge_type.id.to_string());
        ids.extend(edge_type.properties.keys().map(ToString::to_string));
    }
    ids
}

pub(crate) fn render(loaded: &LoadedRevision) -> Result<Rendered, ProjectError> {
    let graph = &loaded.graph;

    // An assertion is listed under the node, edge or type that is its subject. Each kind has its
    // own buckets, keyed by its own id type: a node and an edge may hold one UUID, and each lists
    // only the assertions about itself.
    let mut by_node: BTreeMap<NodeId, Vec<&Assertion>> = BTreeMap::new();
    let mut by_edge: BTreeMap<EdgeId, Vec<&Assertion>> = BTreeMap::new();
    let mut by_type: BTreeMap<TypeId, Vec<&Assertion>> = BTreeMap::new();
    let mut listed = 0_u64;
    for claim in graph.assertions.values() {
        let held = match claim.subject {
            Subject::Node(node) if graph.nodes.contains_key(&node.id()) => {
                by_node.entry(node.id()).or_default().push(claim);
                true
            }
            Subject::Edge(edge) if graph.edges.contains_key(&edge.id()) => {
                by_edge.entry(edge.id()).or_default().push(claim);
                true
            }
            Subject::Type(type_id)
                if graph.ontology.node_type(type_id).is_some()
                    || graph.ontology.edge_type(type_id).is_some() =>
            {
                by_type.entry(type_id).or_default().push(claim);
                true
            }
            _ => false,
        };
        if !held {
            return Err(ProjectError::Inconsistent(format!(
                "assertion {} is about a subject revision {} does not hold",
                claim.id, graph.revision
            )));
        }
        listed += 1;
    }
    let ProjectedOntology {
        node_types,
        edge_types,
        properties,
    } = project_ontology(&graph.ontology, &by_type)?;

    let mut nodes: Vec<ProjectedNode> = graph
        .nodes
        .values()
        .map(|node| ProjectedNode {
            assertions: listed_under(by_node.get(&node.id)),
            id: node.id.to_string(),
            name: node.canonical_name.clone(),
            type_id: node.type_id.to_string(),
            aliases: node.aliases.clone(),
            state: node.type_state.clone(),
            props: props(&node.properties),
        })
        .collect();
    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    let mut edges: Vec<ProjectedEdge> = graph
        .edges
        .values()
        .map(|edge| ProjectedEdge {
            assertions: listed_under(by_edge.get(&edge.id)),
            id: edge.id.to_string(),
            source: edge.source.id().to_string(),
            target: edge.target.id().to_string(),
            type_id: edge.type_id.to_string(),
            props: props(&edge.properties),
        })
        .collect();
    edges.sort_by(|a, b| a.id.cmp(&b.id));
    let mut evidence: Vec<ProjectedEvidence> = graph
        .evidence
        .values()
        .map(|record| ProjectedEvidence {
            id: record.id.to_string(),
            kind: evidence_kind(record.source.kind()),
            locator: record.source.locator(),
            section: record.source.section().map(str::to_owned),
            content_hash: record.content_hash.to_string(),
            observed_at: record.observed_at.millis(),
            confidence_bp: record.confidence.basis_points(),
            retained: loaded.retained.contains(&record.content_hash),
        })
        .collect();
    evidence.sort_by(|a, b| a.id.cmp(&b.id));

    let mut versions = Vec::new();
    for (first, schema) in loaded.schemas.values() {
        if *first > graph.revision {
            continue;
        }
        let version = schema.version();
        let mut added = declared(schema);
        if let Some(parent) = version.parent {
            let (_, parent) = loaded.schemas.get(&parent).ok_or_else(|| {
                ProjectError::Inconsistent(format!(
                    "schema version {} names parent {parent}, which no listed revision is \
                     valid against",
                    version.id
                ))
            })?;
            let inherited = declared(parent);
            added.retain(|id| !inherited.contains(id));
        }
        versions.push(ProjectedSchemaVersion {
            number: version.number,
            id: version.id.to_string(),
            parent: version.parent.map(|parent| parent.to_string()),
            revision: first.get(),
            added: added.into_iter().collect(),
        });
    }
    versions.sort_by_key(|version| version.number);
    let revisions: Vec<ProjectedRevision> = loaded
        .revisions
        .iter()
        .filter(|entry| entry.number <= graph.revision)
        .map(|entry| ProjectedRevision {
            number: entry.number.get(),
            committed_at: entry.committed_at.millis(),
            transaction_id: entry.transaction_id.map(|id| id.to_string()),
            schema_version: entry.schema_version.to_string(),
        })
        .collect();

    let edge_assertions = edges.iter().map(|edge| edge.assertions.len() as u64).sum();
    let every_assertion = nodes
        .iter()
        .flat_map(|node| &node.assertions)
        .chain(edges.iter().flat_map(|edge| &edge.assertions))
        .chain(node_types.iter().flat_map(|declared| &declared.assertions))
        .chain(edge_types.iter().flat_map(|declared| &declared.assertions));
    let (mut assertion_count, mut retracted_assertions) = (0_u64, 0_u64);
    for projected in every_assertion {
        assertion_count += 1;
        if projected.lifecycle.kind == "Retracted" {
            retracted_assertions += 1;
        }
    }
    if assertion_count != listed {
        return Err(ProjectError::Inconsistent(format!(
            "{listed} assertions were placed and {assertion_count} listed"
        )));
    }
    let document = GraphProjectionV1 {
        meta: ProjectionMeta {
            format: FORMAT,
            revision: graph.revision.get(),
            node_count: nodes.len() as u64,
            edge_count: edges.len() as u64,
            assertion_count,
            evidence_count: evidence.len() as u64,
        },
        ontology: ProjectedOntology {
            node_types,
            edge_types,
            properties,
        },
        nodes,
        edges,
        evidence,
        schema: ProjectedSchema {
            versions,
            revisions,
        },
    };
    let bytes = serde_json::to_vec(&document)
        .map_err(|error| ProjectError::Inconsistent(format!("encoding the document: {error}")))?;
    let summary = GraphProjected {
        revision: document.meta.revision,
        nodes: document.meta.node_count,
        edges: document.meta.edge_count,
        assertions: document.meta.assertion_count,
        evidence: document.meta.evidence_count,
        schema_versions: document.schema.versions.len() as u64,
        revisions: document.schema.revisions.len() as u64,
        transactions: document
            .schema
            .revisions
            .iter()
            .filter(|entry| entry.transaction_id.is_some())
            .count() as u64,
        node_types: document.ontology.node_types.len() as u64,
        edge_types: document.ontology.edge_types.len() as u64,
        properties: document.ontology.properties.len() as u64,
        edge_assertions,
        retracted_assertions,
        retained_evidence: document
            .evidence
            .iter()
            .filter(|record| record.retained)
            .count() as u64,
        projection_hash: hex::encode(Sha256::digest(&bytes)),
    };
    Ok(Rendered { bytes, summary })
}

/// `ontology` as `ekr.graph-projection/1` projects it, with the type assertions `by_type`.
///
/// # Errors
///
/// [`ProjectError::Inconsistent`] for a property id two types declare with a different name or
/// value kind.
pub(crate) fn project_ontology(
    ontology: &Ontology,
    by_type: &BTreeMap<TypeId, Vec<&Assertion>>,
) -> Result<ProjectedOntology, ProjectError> {
    let ontology = ontology.to_document();
    let mut node_types: Vec<ProjectedNodeType> = ontology
        .node_types
        .iter()
        .map(|declared| ProjectedNodeType {
            id: declared.id.to_string(),
            name: declared.name.clone(),
            properties: sorted(declared.properties.keys()),
            assertions: listed_under(by_type.get(&declared.id)),
        })
        .collect();
    node_types.sort_by(|a, b| a.id.cmp(&b.id));
    let mut edge_types: Vec<ProjectedEdgeType> = ontology
        .edge_types
        .iter()
        .map(|declared| ProjectedEdgeType {
            id: declared.id.to_string(),
            name: declared.name.clone(),
            source_types: sorted(&declared.source_types),
            target_types: sorted(&declared.target_types),
            properties: sorted(declared.properties.keys()),
            assertions: listed_under(by_type.get(&declared.id)),
        })
        .collect();
    edge_types.sort_by(|a, b| a.id.cmp(&b.id));
    // `ontology.properties` holds one entry per property id, carrying its name and value kind and
    // nothing else of the definition. Each declaring type's entry is built exactly as it is
    // projected, and the entries are compared: two types whose declarations of one id agree on
    // name and value kind project it once, whatever else of their definitions differs, because
    // the format carries nothing else. Entries that differ in either cannot both be projected, and
    // keeping one would tell a reader of the other type the wrong name or kind; the render refuses
    // instead, until the format carries each type's own definition
    // (task:projection-carries-per-type-property-definitions).
    let mut definitions: BTreeMap<String, (ProjectedProperty, TypeId)> = BTreeMap::new();
    for (owner, property) in ontology
        .node_types
        .iter()
        .flat_map(|declared| {
            declared
                .properties
                .values()
                .map(move |property| (declared.id, property))
        })
        .chain(ontology.edge_types.iter().flat_map(|declared| {
            declared
                .properties
                .values()
                .map(move |property| (declared.id, property))
        }))
    {
        let entry = projected_property(property);
        let (held, first) = definitions
            .entry(entry.id.clone())
            .or_insert_with(|| (projected_property(property), owner));
        if *held != entry {
            return Err(ProjectError::Inconsistent(format!(
                "property {} is declared by type {first} as {:?} of kind {} and by type {owner} \
                 as {:?} of kind {}, and ekr.graph-projection/1 carries one name and value kind \
                 per property id (task:projection-carries-per-type-property-definitions)",
                entry.id, held.name, held.value_kind, entry.name, entry.value_kind
            )));
        }
    }
    let properties: Vec<ProjectedProperty> = definitions
        .into_values()
        .map(|(property, _)| property)
        .collect();
    Ok(ProjectedOntology {
        node_types,
        edge_types,
        properties,
    })
}
