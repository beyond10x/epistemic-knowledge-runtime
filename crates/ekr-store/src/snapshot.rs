//! Serializable graph data. Only the kernel admits a document as canonical state.
//! The store delegates every seed replay to its injected authority.

use std::collections::BTreeMap;
use std::fmt;

use ekr_core::{
    AssertionId, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId, RevisionNumber,
    SchemaVersionId,
};
use ekr_graph::{
    Assertion, CanonicalGraph, CanonicalValue, Edge, Evidence, GraphRoot, InadmissibleValue, Node,
    Object, Space, Subject,
};
use ekr_ontology::Value;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::StoreError;

/// A graph written down: `ekr.store.Snapshot`'s materialised fold, as bytes.
///
/// Holds the **transient** instantiation of every graph type, which is what makes the crossing a
/// place rather than a habit. See the module documentation for why that is the shape.
#[derive(Clone, Debug, PartialEq)]
pub struct GraphDocument {
    /// The root the state hangs off.
    pub root: GraphRoot,
    /// Its position in the revision lineage.
    pub revision: RevisionNumber,
    /// Its nodes, by id.
    pub nodes: BTreeMap<NodeId, Node<Value>>,
    /// Its edges, by id.
    pub edges: BTreeMap<EdgeId, Edge<Value>>,
    /// Its assertions, by id.
    pub assertions: BTreeMap<AssertionId, Assertion<Value>>,
    /// The retained evidence its assertions rest on, by id.
    ///
    /// Not generic and not widened: [`Evidence`] carries no value at all, so there is no transient
    /// instantiation of it and nothing for the crossing to refuse.
    pub evidence: BTreeMap<EvidenceId, Evidence>,
}

// The remote derive preserves the useful in-memory graph shape while requiring a complete,
// explicitly versioned envelope at every serialization boundary, including nested seed input.
#[derive(Serialize, Deserialize)]
#[serde(remote = "GraphDocument", deny_unknown_fields)]
struct GraphFields {
    root: GraphRoot,
    revision: RevisionNumber,
    #[serde(deserialize_with = "ekr_core::decode::unique_map")]
    nodes: BTreeMap<NodeId, Node<Value>>,
    #[serde(deserialize_with = "ekr_core::decode::unique_map")]
    edges: BTreeMap<EdgeId, Edge<Value>>,
    #[serde(deserialize_with = "ekr_core::decode::unique_map")]
    assertions: BTreeMap<AssertionId, Assertion<Value>>,
    #[serde(deserialize_with = "ekr_core::decode::unique_map")]
    evidence: BTreeMap<EvidenceId, Evidence>,
}

#[derive(Serialize, Deserialize)]
enum GraphFormat {
    #[serde(rename = "ekr.graph-document/2")]
    Current,
}

#[derive(Serialize)]
struct GraphEnvelopeRef<'a> {
    format: GraphFormat,
    #[serde(with = "GraphFields")]
    graph: &'a GraphDocument,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GraphEnvelope {
    format: GraphFormat,
    #[serde(with = "GraphFields")]
    graph: GraphDocument,
}

impl Serialize for GraphDocument {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        GraphEnvelopeRef {
            format: GraphFormat::Current,
            graph: self,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for GraphDocument {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let GraphEnvelope {
            format: GraphFormat::Current,
            graph,
        } = GraphEnvelope::deserialize(deserializer)?;
        Ok(graph)
    }
}

impl GraphDocument {
    /// The document a canonical graph writes.
    ///
    /// Total and lossless in this direction: every [`CanonicalValue`] is a [`Value`], because the
    /// canonical value is the same kinds minus the one that cannot be encoded. The ontology is
    /// dropped, which is the one thing a document does not carry.
    #[must_use]
    pub fn of(graph: &CanonicalGraph) -> Self {
        Self {
            root: graph.root,
            revision: graph.revision,
            nodes: graph
                .nodes
                .iter()
                .map(|(id, node)| (*id, widen_node(node)))
                .collect(),
            edges: graph
                .edges
                .iter()
                .map(|(id, edge)| (*id, widen_edge(edge)))
                .collect(),
            assertions: graph
                .assertions
                .iter()
                .map(|(id, assertion)| (*id, widen_assertion(assertion)))
                .collect(),
            evidence: graph.evidence.clone(),
        }
    }

    /// The document, as the bytes a [`StoredObject`](crate::StoredObject) holds.
    ///
    /// # Errors
    ///
    /// [`StoreError::Document`] when the document cannot be serialised.
    pub fn to_bytes(&self) -> Result<Vec<u8>, StoreError> {
        serde_json::to_vec(self).map_err(|error| StoreError::Document(error.to_string()))
    }

    /// The document those bytes carry.
    ///
    /// Reads into this type; kernel admission is a separate operation.
    ///
    /// # Errors
    ///
    /// [`StoreError::Document`] when the bytes are not a document.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        serde_json::from_slice(bytes).map_err(|error| StoreError::Document(error.to_string()))
    }
}

/// One record of a document, named by kind and id.
///
/// What a refusal points at. Its own type rather than a formatted string, because a caller that
/// has to parse a message to find out which node was refused is a caller that cannot act on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entity {
    /// A node.
    Node(NodeId),
    /// An edge.
    Edge(EdgeId),
    /// An assertion.
    Assertion(AssertionId),
    /// A piece of evidence.
    Evidence(EvidenceId),
}

impl fmt::Display for Entity {
    /// The kind as the domain spells it, then the id.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Node(id) => write!(f, "node {id}"),
            Self::Edge(id) => write!(f, "edge {id}"),
            Self::Assertion(id) => write!(f, "assertion {id}"),
            Self::Evidence(id) => write!(f, "evidence {id}"),
        }
    }
}

/// A document that does not cross into canonical state, and what in it did not.
///
/// Three kinds of refusal, and the module's table says which field earns which: the document
/// belongs to another space or another schema version; a record is filed under the wrong key or
/// carries another root's id; or a value is one canonical state does not admit.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum MembraneError {
    /// The document's root declares a space that is not canonical.
    ///
    /// `ekr.graph.Space` is on the root, and this is the one place a document's own answer to
    /// "which space wrote this" is read. AGENTS.md invariant 2 is about the reference direction
    /// inside Rust; this is the same boundary where the bytes are.
    #[error("the document's root declares {declared:?} space, which canonical state is not")]
    Space {
        /// The space the document's root declares.
        declared: Space,
    },

    /// The document was written against one schema version and opened against another.
    ///
    /// A `CanonicalGraph` says twice which schema it is typed by — `root.schema_version_id` and
    /// `ontology` — and the two agreeing is what makes "valid against its schema" a statement
    /// about one schema rather than two.
    #[error(
        "the document was written against schema version {written_against} and this store was \
         opened with {opened_with}"
    )]
    SchemaVersion {
        /// The version the document's root names.
        written_against: SchemaVersionId,
        /// The version the caller's ontology is.
        opened_with: SchemaVersionId,
    },

    /// The document's root names itself as the root it was derived from.
    ///
    /// `GraphRoot.parent` is "the root it was derived from, if any", and a root derived from itself
    /// is a cycle of length one rather than a lineage. This crossing cannot *resolve* a parent —
    /// nothing here knows which other roots exist — but it can read this one, and a document that
    /// disagrees with itself is refused where it is read.
    #[error("root {root_id} names itself as the root it was derived from")]
    SelfParentedRoot {
        /// The root that is its own parent.
        root_id: GraphRootId,
    },

    /// A record is filed in the document under a key that is not its own id.
    #[error("{key} is filed under a key that is not its own id: the record there is {found}")]
    Misfiled {
        /// The key it was filed under.
        key: Entity,
        /// The record found there.
        found: Entity,
    },

    /// A record belongs to a graph root that is not the document's.
    #[error("{entity} belongs to root {root_id}, and this document hangs off {document_root}")]
    Misrooted {
        /// The record.
        entity: Entity,
        /// The root it says it belongs to.
        root_id: GraphRootId,
        /// The root the document hangs off.
        document_root: GraphRootId,
    },

    /// A node property canonical state does not admit.
    #[error("node {node_id} property {property}: {cause}")]
    Node {
        /// The node.
        node_id: NodeId,
        /// The property it sat on.
        property: PropertyId,
        /// What was wrong with it, and where inside it.
        cause: InadmissibleValue,
    },
    /// An edge property canonical state does not admit.
    #[error("edge {edge_id} property {property}: {cause}")]
    Edge {
        /// The edge.
        edge_id: EdgeId,
        /// The property it sat on.
        property: PropertyId,
        /// What was wrong with it, and where inside it.
        cause: InadmissibleValue,
    },
    /// An assertion's object canonical state does not admit.
    #[error("assertion {assertion_id} object: {cause}")]
    Assertion {
        /// The assertion.
        assertion_id: AssertionId,
        /// What was wrong with it, and where inside it.
        cause: InadmissibleValue,
    },
}

/// A canonical node, as a candidate's shape. Total: every canonical value is a value.
fn widen_node(node: &Node<CanonicalValue>) -> Node<Value> {
    Node {
        id: node.id,
        root_id: node.root_id,
        type_id: node.type_id,
        canonical_name: node.canonical_name.clone(),
        aliases: node.aliases.clone(),
        type_state: node.type_state.clone(),
        properties: node
            .properties
            .iter()
            .map(|(id, value)| (*id, value.iter().cloned().map(Value::from).collect()))
            .collect(),
    }
}

/// A canonical edge, as a candidate's shape.
///
/// Its two ends lose their type on the way out and are minted back on the way in: a document
/// carries ids, which is the serde boundary
/// `architecture-decision-record:0008-canonical-state-references-are-typed` states rather than
/// hides.
fn widen_edge(edge: &Edge<CanonicalValue>) -> Edge<Value> {
    Edge {
        id: edge.id,
        root_id: edge.root_id,
        type_id: edge.type_id,
        source: edge.source.node(),
        target: edge.target.node(),
        properties: edge
            .properties
            .iter()
            .map(|(id, value)| (*id, value.iter().cloned().map(Value::from).collect()))
            .collect(),
    }
}

/// A canonical assertion, as a candidate's shape.
fn widen_assertion(assertion: &Assertion<CanonicalValue>) -> Assertion<Value> {
    Assertion {
        id: assertion.id,
        root_id: assertion.root_id,
        subject: match assertion.subject {
            Subject::Node(node) => Subject::Node(node.node()),
            Subject::Edge(edge) => Subject::Edge(edge),
            Subject::Type(type_id) => Subject::Type(type_id),
        },
        predicate: assertion.predicate,
        object: match &assertion.object {
            Object::Value(value) => Object::Value(Value::from(value.clone())),
            Object::Node(node) => Object::Node(node.node()),
            Object::Type(type_id) => Object::Type(*type_id),
        },
        evidence: assertion.evidence.clone(),
        proposed_by: assertion.proposed_by,
        assessment: assertion.assessment.clone(),
        lifecycle: assertion.lifecycle.clone(),
        valid_time: assertion.valid_time,
        transaction_time: assertion.transaction_time,
    }
}
