//! Kernel-owned bootstrap admission, without a preceding committed revision.
use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AgentId, ContentHash, GraphRootId, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::{
    Assertion, Assessment, CanonicalGraph, CanonicalRef, CanonicalValue, Edge, EvidenceSource,
    GraphSnapshot, Node, Object, Space, Subject,
};
use ekr_ontology::{Ontology, OntologyDocument, Value};
use ekr_store::{Entity, GraphDocument, MembraneError, StoreError};
use serde::{Deserialize, Serialize};

use crate::{AuthorityStateV1, EdgeDraft, GraphOperation, GraphTransaction, NodeDraft, Pipeline};

/// Independently authenticated execution identities, supplied by the host, never the seed input.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct BootstrapContext {
    /// The operator submitting the seed.
    pub operator: AgentId,
    /// The distinct agent performing deterministic validation.
    pub validator: AgentId,
}

/// Version two seed input. A graph format change must also change this version.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(deny_unknown_fields)]
pub struct SeedDocument {
    /// Must be `ekr-seed/2`.
    #[cfg_attr(feature = "schema", schemars(extend("const" = "ekr-seed/2")))]
    pub format: String,
    /// Complete ontology, retained rather than reconstructed from a schema identity.
    pub ontology: OntologyDocument,
    /// Proposed graph records, before the kernel attributes acceptance.
    #[cfg_attr(
        feature = "schema",
        schemars(schema_with = "crate::schema::graph_document")
    )]
    pub graph: GraphDocument,
    /// Exact retained HumanStatement bytes, keyed by their content address.
    #[serde(deserialize_with = "ekr_core::decode::unique_map")]
    pub evidence_payloads: BTreeMap<ContentHash, Vec<u8>>,
}

impl SeedDocument {
    /// Reads the versioned YAML input without discarding unknown semantic fields.
    ///
    /// # Errors
    /// Malformed input or an unsupported format.
    pub fn from_yaml(yaml: &str) -> Result<Self, SeedError> {
        let document: Self = serde_yaml_ng::from_str(yaml)
            .map_err(|error| SeedError::Invalid(format!("seed-decode: {error}")))?;
        document.check_version()?;
        Ok(document)
    }

    fn check_version(&self) -> Result<(), SeedError> {
        if self.format != "ekr-seed/2" {
            return invalid("unsupported-seed-format");
        }
        Ok(())
    }
}

/// A named bootstrap refusal.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SeedError {
    /// A deterministic admission rule failed.
    #[error("{0}")]
    Invalid(String),
    /// The provider could not initialize the lineage.
    #[error(transparent)]
    Store(#[from] StoreError),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SeedEnvelope {
    pub(crate) format: String,
    pub(crate) input: SeedDocument,
    pub(crate) context: BootstrapContext,
    pub(crate) authority: AuthorityStateV1,
    pub(crate) committed_at: Timestamp,
}

fn invalid<T>(code: &str) -> Result<T, SeedError> {
    Err(SeedError::Invalid(code.to_owned()))
}

/// `seed-evidence-payload-missing`, naming the entry's `content_hash`, every `evidence_payloads`
/// key no evidence entry names — where a half-applied correction left the payload — and the rule
/// that the two are one value.
fn payload_missing<T>(
    evidence: &ekr_graph::Evidence,
    entries: &BTreeMap<ekr_core::EvidenceId, ekr_graph::Evidence>,
    payloads: &BTreeMap<ContentHash, Vec<u8>>,
) -> Result<T, SeedError> {
    let named: BTreeSet<ContentHash> = entries.values().map(|e| e.content_hash).collect();
    let unnamed: Vec<String> = payloads
        .keys()
        .filter(|key| !named.contains(key))
        .map(ToString::to_string)
        .collect();
    let unnamed = if unnamed.is_empty() {
        "none".to_owned()
    } else {
        unnamed.join(", ")
    };
    Err(SeedError::Invalid(format!(
        "seed-evidence-payload-missing: evidence {} has content_hash {}, which is not a key of \
         evidence_payloads; an evidence entry's content_hash and its evidence_payloads key must \
         be the same value (`ekr hash` of the payload); keys no evidence entry names: {unnamed}",
        evidence.id, evidence.content_hash
    )))
}

/// `seed-evidence-payload-mismatch`, naming the content hash the payload's bytes have (expected)
/// and the one the seed wrote for them (found), so the author can correct the entry.
fn payload_mismatch<T>(expected: ContentHash, found: ContentHash) -> Result<T, SeedError> {
    Err(SeedError::Invalid(format!(
        "seed-evidence-payload-mismatch: expected {expected} \
         (sha256(\"ekr.payload.v1\" || the payload's bytes)), found {found}"
    )))
}

pub(crate) fn envelope(bytes: &[u8]) -> Result<SeedEnvelope, StoreError> {
    let shape: serde_json::Value = serde_json::from_slice(bytes)
        .map_err(|error| StoreError::InvalidSeed(format!("seed-decode: {error}")))?;
    if shape.get("format").is_none() && shape.get("root").is_some() {
        return Err(StoreError::SeedMigrationRequired);
    }
    let envelope: SeedEnvelope = serde_json::from_slice(bytes)
        .map_err(|error| StoreError::InvalidSeed(format!("seed-decode: {error}")))?;
    if envelope.format != "ekr-seed-envelope/2" {
        return Err(StoreError::InvalidSeed(
            "unsupported-seed-envelope".to_owned(),
        ));
    }
    Ok(envelope)
}

pub(crate) fn replay(
    bytes: &[u8],
    ontology: Option<&Ontology>,
    context: BootstrapContext,
    authority: &AuthorityStateV1,
) -> Result<CanonicalGraph, StoreError> {
    let envelope = envelope(bytes)?;
    authority.check(context)?;
    if envelope.context != context || envelope.authority != *authority {
        return Err(StoreError::AuthorityMismatch);
    }
    let graph = admitted_graph(&envelope.input, context, envelope.committed_at)
        .map_err(|error| StoreError::InvalidSeed(error.to_string()))?;
    if ontology.is_some_and(|expected| graph.ontology != *expected) {
        return Err(StoreError::InvalidSeed("seed-ontology-mismatch".to_owned()));
    }
    Ok(graph)
}

pub(crate) fn admitted_graph(
    input: &SeedDocument,
    context: BootstrapContext,
    committed_at: Timestamp,
) -> Result<CanonicalGraph, SeedError> {
    input.check_version()?;
    let ontology = Ontology::load(input.ontology.clone())
        .map_err(|error| SeedError::Invalid(format!("seed-ontology: {error}")))?;
    if ontology.version().number != 0 || ontology.version().parent.is_some() {
        return invalid("seed-ontology-lineage");
    }
    let document = &input.graph;
    if document.root.space != Space::Canonical {
        return invalid("seed-space");
    }
    if document.root.schema_version_id != ontology.version().id {
        return invalid("seed-schema-version");
    }
    if document.root.parent.is_some() || document.revision != RevisionNumber::SEED {
        return invalid("seed-root-lineage");
    }
    if context.operator == context.validator {
        return invalid("proposer-is-validator");
    }
    for (key, node) in &document.nodes {
        if node.properties.values().any(Vec::is_empty) {
            return invalid("seed-empty-property-values");
        }
        filing(
            Entity::Node(*key),
            Entity::Node(node.id),
            node.root_id,
            document.root.id,
        )?;
        if let Some(declared) = ontology.node_type(node.type_id) {
            match (&declared.lifecycle, &node.type_state) {
                (Some(lifecycle), Some(state)) if state == &lifecycle.initial => {}
                (None, None) => {}
                _ => return invalid("seed-initial-lifecycle"),
            }
        }
    }
    for (key, edge) in &document.edges {
        if edge.properties.values().any(Vec::is_empty) {
            return invalid("seed-empty-property-values");
        }
        filing(
            Entity::Edge(*key),
            Entity::Edge(edge.id),
            edge.root_id,
            document.root.id,
        )?;
    }
    for (key, assertion) in &document.assertions {
        filing(
            Entity::Assertion(*key),
            Entity::Assertion(assertion.id),
            assertion.root_id,
            document.root.id,
        )?;
        if assertion.proposed_by != context.operator {
            return invalid("seed-attribution-mismatch");
        }
    }
    for (key, evidence) in &document.evidence {
        if *key != evidence.id {
            return invalid("seed-misfiled-evidence");
        }
        if !matches!(evidence.source, EvidenceSource::HumanStatement { .. }) {
            return invalid("seed-unsupported-source");
        }
        if evidence.extracted_by != context.operator {
            return invalid("seed-attribution-mismatch");
        }
        let Some(payload) = input.evidence_payloads.get(&evidence.content_hash) else {
            return payload_missing(evidence, &document.evidence, &input.evidence_payloads);
        };
        let expected = ContentHash::of_bytes(payload);
        if expected != evidence.content_hash {
            return payload_mismatch(expected, evidence.content_hash);
        }
    }
    // An extra map entry is still retained input: verify every content address, cited or not.
    for (hash, payload) in &input.evidence_payloads {
        let expected = ContentHash::of_bytes(payload);
        if expected != *hash {
            return payload_mismatch(expected, *hash);
        }
    }

    // Explicit bootstrap validation context. This empty index is not a previous committed
    // revision: it supplies the seed's ontology and independently verified evidence to the
    // invariant validators. No ValidatedTransaction is sealed and no against-revision exists.
    let mut initial = CanonicalGraph {
        root: document.root,
        revision: RevisionNumber::SEED,
        ontology: ontology.clone(),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
    };
    initial.evidence.clone_from(&document.evidence);
    let operations = document
        .nodes
        .values()
        .map(|node| {
            GraphOperation::CreateNode(NodeDraft {
                id: node.id,
                root_id: node.root_id,
                type_id: node.type_id,
                canonical_name: node.canonical_name.clone(),
                properties: node.properties.clone(),
            })
        })
        .chain(document.edges.values().map(|edge| {
            GraphOperation::CreateEdge(EdgeDraft {
                id: edge.id,
                root_id: edge.root_id,
                type_id: edge.type_id,
                source: edge.source,
                target: edge.target,
                properties: edge.properties.clone(),
            })
        }))
        .chain(
            document
                .assertions
                .values()
                .cloned()
                .map(|a| GraphOperation::AddAssertion(Box::new(a))),
        )
        .collect();
    let proposal = GraphTransaction {
        // Diagnostic correlation only; this request is never persisted as a transaction.
        id: TransactionId::from_uuid(document.root.id.to_uuid()),
        proposer: context.operator,
        operations,
        evidence: document
            .assertions
            .values()
            .flat_map(|a| a.evidence.iter().copied())
            .collect(),
        schema_version: None,
    };
    Pipeline::deterministic(context.validator)
        .validate_bootstrap(&GraphSnapshot::of(&initial), &proposal)
        .map_err(|issues| {
            SeedError::Invalid(
                issues
                    .iter()
                    .map(|i| format!("{}: {}", i.code, i.message))
                    .collect::<Vec<_>>()
                    .join("; "),
            )
        })?;

    let narrow_error = |error: MembraneError| SeedError::Invalid(error.to_string());
    let nodes = document
        .nodes
        .iter()
        .map(|(id, node)| Ok((*id, narrow_node(node.clone())?)))
        .collect::<Result<_, MembraneError>>()
        .map_err(narrow_error)?;
    let edges = document
        .edges
        .iter()
        .map(|(id, edge)| Ok((*id, narrow_edge(edge.clone())?)))
        .collect::<Result<_, MembraneError>>()
        .map_err(narrow_error)?;
    let assertions = document
        .assertions
        .iter()
        .map(|(id, assertion)| {
            let mut admitted = narrow_assertion(assertion.clone())?;
            admitted.assessment = Assessment::Accepted {
                validators: BTreeSet::from([context.validator]),
            };
            admitted.transaction_time = ekr_graph::TransactionTime::since(committed_at);
            Ok((*id, admitted))
        })
        .collect::<Result<_, MembraneError>>()
        .map_err(narrow_error)?;
    Ok(CanonicalGraph {
        root: document.root,
        revision: RevisionNumber::SEED,
        ontology,
        nodes,
        edges,
        assertions,
        evidence: document.evidence.clone(),
    })
}

fn filing(
    key: Entity,
    found: Entity,
    root: GraphRootId,
    expected: GraphRootId,
) -> Result<(), SeedError> {
    if key != found {
        return invalid("seed-misfiled-entity");
    }
    if root != expected {
        return invalid("seed-misrooted-entity");
    }
    Ok(())
}

/// A candidate's node, as canonical state — or the refusal that says why it is not.
pub(crate) fn narrow_node(node: Node<Value>) -> Result<Node<CanonicalValue>, MembraneError> {
    let node_id = node.id;
    let mut properties = BTreeMap::new();
    for (property, value) in node.properties {
        let value = value
            .into_iter()
            .map(|value| {
                CanonicalValue::try_from(value).map_err(|cause| MembraneError::Node {
                    node_id,
                    property,
                    cause,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        properties.insert(property, value);
    }
    Ok(Node {
        id: node.id,
        root_id: node.root_id,
        type_id: node.type_id,
        canonical_name: node.canonical_name,
        aliases: node.aliases,
        type_state: node.type_state,
        properties,
    })
}

/// A candidate's edge, as canonical state — or the refusal that says why it is not.
///
/// References are narrowed only after deterministic bootstrap validation succeeds.
pub(crate) fn narrow_edge(edge: Edge<Value>) -> Result<Edge<CanonicalValue>, MembraneError> {
    let edge_id = edge.id;
    let mut properties = BTreeMap::new();
    for (property, value) in edge.properties {
        let value = value
            .into_iter()
            .map(|value| {
                CanonicalValue::try_from(value).map_err(|cause| MembraneError::Edge {
                    edge_id,
                    property,
                    cause,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        properties.insert(property, value);
    }
    Ok(Edge {
        id: edge.id,
        root_id: edge.root_id,
        type_id: edge.type_id,
        source: CanonicalRef::new(edge.source),
        target: CanonicalRef::new(edge.target),
        properties,
    })
}

/// A candidate's assertion, as canonical state — or the refusal that says why it is not.
pub(crate) fn narrow_assertion(
    assertion: Assertion<Value>,
) -> Result<Assertion<CanonicalValue>, MembraneError> {
    let assertion_id = assertion.id;
    let object = match assertion.object {
        Object::Value(value) => {
            Object::Value(CanonicalValue::try_from(value).map_err(|cause| {
                MembraneError::Assertion {
                    assertion_id,
                    cause,
                }
            })?)
        }
        Object::Node(node) => Object::Node(CanonicalRef::new(node)),
        Object::Type(type_id) => Object::Type(type_id),
    };
    Ok(Assertion {
        id: assertion.id,
        root_id: assertion.root_id,
        subject: match assertion.subject {
            Subject::Node(node) => Subject::Node(CanonicalRef::new(node)),
            Subject::Edge(edge) => Subject::Edge(CanonicalRef::new(edge)),
            Subject::Type(type_id) => Subject::Type(type_id),
        },
        predicate: assertion.predicate,
        object,
        evidence: assertion
            .evidence
            .into_iter()
            .map(CanonicalRef::new)
            .collect(),
        proposed_by: assertion.proposed_by,
        assessment: assertion.assessment.map_assertions(CanonicalRef::new),
        lifecycle: assertion.lifecycle.map_assertions(CanonicalRef::new),
        valid_time: assertion.valid_time,
        transaction_time: assertion.transaction_time,
    })
}
