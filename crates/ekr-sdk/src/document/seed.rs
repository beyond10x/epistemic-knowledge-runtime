//! `ekr-seed/2`: the schema, the graph at revision 0 and its evidence bytes (`docs/cli.md`,
//! "The seed document").

use std::collections::BTreeMap;

use ekr_core::{
    AssertionId, ContentHash, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId, RevisionNumber,
    SchemaVersionId, Timestamp, TypeId,
};
use serde::{Deserialize, Serialize};

use super::graph::{Assertion, EdgeDraft, Evidence, NodeDraft};
use super::transaction::EvidenceAddition;
use super::value::{EdgeType, NodeType, Value};
use super::DocumentError;

/// The format every seed the SDK writes declares.
pub const SEED_FORMAT: &str = "ekr-seed/2";
/// The format of a seed's graph section.
pub const GRAPH_FORMAT: &str = "ekr.graph-document/2";

/// An `ekr-seed/2` document: what `ekr seed` reads.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedDocument {
    /// [`SEED_FORMAT`].
    pub format: String,
    /// The first version of the schema.
    pub ontology: OntologySection,
    /// The graph at revision 0.
    pub graph: GraphSection,
    /// Every evidence entry's bytes, by its content hash.
    pub evidence_payloads: BTreeMap<ContentHash, Vec<u8>>,
}

impl SeedDocument {
    /// The document as YAML, every variant written as a tag.
    ///
    /// # Errors
    /// [`DocumentError::Yaml`] when the writer refuses a value.
    pub fn to_yaml(&self) -> Result<String, DocumentError> {
        super::to_yaml(self)
    }
}

/// A seed's ontology section: the version and every type.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OntologySection {
    /// The schema version the seed declares.
    pub version: SchemaVersion,
    /// Its node types.
    pub node_types: Vec<NodeType>,
    /// Its edge types.
    pub edge_types: Vec<EdgeType>,
}

/// One schema version: at the seed, number 0 with no parent.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaVersion {
    /// Its id.
    pub id: SchemaVersionId,
    /// 0 at the seed.
    pub number: u64,
    /// `None` at the seed.
    pub parent: Option<SchemaVersionId>,
    /// When it was created.
    pub created_at: Timestamp,
}

impl SchemaVersion {
    /// A seed's version, under a freshly minted id.
    #[must_use]
    pub fn seed(created_at: Timestamp) -> Self {
        Self {
            id: SchemaVersionId::mint(),
            number: 0,
            parent: None,
            created_at,
        }
    }
}

/// A seed's graph section: the `ekr.graph-document/2` envelope.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphSection {
    /// [`GRAPH_FORMAT`].
    pub format: String,
    /// The graph.
    pub graph: SeedGraph,
}

/// The graph at revision 0, every map keyed by the id its entries carry.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedGraph {
    /// The graph root.
    pub root: GraphRoot,
    /// 0.
    pub revision: RevisionNumber,
    /// Its nodes.
    pub nodes: BTreeMap<NodeId, SeedNode>,
    /// Its edges.
    pub edges: BTreeMap<EdgeId, EdgeDraft>,
    /// Its assertions, each `Proposed` and `Active`.
    pub assertions: BTreeMap<AssertionId, Assertion>,
    /// Its evidence entries.
    pub evidence: BTreeMap<EvidenceId, Evidence>,
}

/// Which space a graph root is in; a seed's is canonical.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Space {
    /// `Canonical`.
    Canonical,
}

/// The root every seeded entity names as its `root_id`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GraphRoot {
    /// Its id.
    pub id: GraphRootId,
    /// `Canonical`.
    pub space: Space,
    /// The ontology's version id.
    pub schema_version_id: SchemaVersionId,
    /// `None` at the seed.
    pub parent: Option<GraphRootId>,
    /// When it was created.
    pub created_at: Timestamp,
}

/// A seeded node: a draft and its lifecycle state.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SeedNode {
    /// Its id.
    pub id: NodeId,
    /// The graph root.
    pub root_id: GraphRootId,
    /// A concrete node type.
    pub type_id: TypeId,
    /// The name a reader sees.
    pub canonical_name: String,
    /// The names a typed reference is matched against.
    pub aliases: Vec<String>,
    /// Its type's lifecycle `initial` state, or `None` for a type without one.
    pub type_state: Option<String>,
    /// Its values, by property id; each list non-empty.
    pub properties: BTreeMap<PropertyId, Vec<Value>>,
}

/// Builds a [`SeedDocument`] around one ontology section: every entity is filed under its id and
/// every evidence entry's bytes under their hash, computed here.
#[derive(Clone, Debug)]
pub struct SeedBuilder {
    ontology: OntologySection,
    root: GraphRoot,
    nodes: BTreeMap<NodeId, SeedNode>,
    edges: BTreeMap<EdgeId, EdgeDraft>,
    assertions: BTreeMap<AssertionId, Assertion>,
    evidence: BTreeMap<EvidenceId, Evidence>,
    payloads: BTreeMap<ContentHash, Vec<u8>>,
}

impl SeedBuilder {
    /// A seed of `ontology` with an empty graph under a freshly minted root created at
    /// `created_at`.
    #[must_use]
    pub fn new(ontology: OntologySection, created_at: Timestamp) -> Self {
        let root = GraphRoot {
            id: GraphRootId::mint(),
            space: Space::Canonical,
            schema_version_id: ontology.version.id,
            parent: None,
            created_at,
        };
        Self {
            ontology,
            root,
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            assertions: BTreeMap::new(),
            evidence: BTreeMap::new(),
            payloads: BTreeMap::new(),
        }
    }

    /// The root id every entity of this seed names.
    #[must_use]
    pub const fn root_id(&self) -> GraphRootId {
        self.root.id
    }

    /// This seed with one more node, in `type_state` (its type's `initial` state, or `None`).
    #[must_use]
    pub fn node(mut self, node: NodeDraft, type_state: Option<String>) -> Self {
        self.nodes.insert(
            node.id,
            SeedNode {
                id: node.id,
                root_id: node.root_id,
                type_id: node.type_id,
                canonical_name: node.canonical_name,
                aliases: node.aliases,
                type_state,
                properties: node.properties,
            },
        );
        self
    }

    /// This seed with one more edge.
    #[must_use]
    pub fn edge(mut self, edge: EdgeDraft) -> Self {
        self.edges.insert(edge.id, edge);
        self
    }

    /// This seed with one more assertion.
    #[must_use]
    pub fn assertion(mut self, assertion: Assertion) -> Self {
        self.assertions.insert(assertion.id, assertion);
        self
    }

    /// This seed with one more evidence entry, its bytes filed under their hash.
    #[must_use]
    pub fn evidence(mut self, addition: EvidenceAddition) -> Self {
        let EvidenceAddition { evidence, payload } = addition;
        self.payloads.insert(evidence.content_hash, payload);
        self.evidence.insert(evidence.id, evidence);
        self
    }

    /// The document.
    #[must_use]
    pub fn build(self) -> SeedDocument {
        SeedDocument {
            format: SEED_FORMAT.to_owned(),
            ontology: self.ontology,
            graph: GraphSection {
                format: GRAPH_FORMAT.to_owned(),
                graph: SeedGraph {
                    root: self.root,
                    revision: RevisionNumber::new(0),
                    nodes: self.nodes,
                    edges: self.edges,
                    assertions: self.assertions,
                    evidence: self.evidence,
                },
            },
            evidence_payloads: self.payloads,
        }
    }
}
