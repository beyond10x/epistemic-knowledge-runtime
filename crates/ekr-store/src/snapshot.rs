//! The materialised fold at a revision, and the one place a document becomes canonical state.
//!
//! Design § 71: readers operate on immutable snapshots. A fold has to be written down somewhere to
//! survive a process, and what it is written down as is a [`GraphDocument`] — a
//! [`CanonicalGraph`] with its values widened to [`Value`] and its ontology left out.
//!
//! # The membrane stops here, and this is the door
//!
//! `task:the-membrane-stops-at-the-store-boundary`. [`ekr_graph::CanonicalValue`] serialises
//! *through* [`Value`], so a canonical node and an incubation-forest candidate write **identical
//! bytes**. The decision this crate takes is the smallest of the three that task names: **the
//! store never deserialises straight into a canonical type from a document it did not write.** A
//! document is read as the *transient* instantiation — `Node<Value>`, `Edge<Value>`,
//! `Assertion<Value>` — and [`GraphDocument::into_canonical`] is the single named crossing.
//!
//! # The document is self-describing, and the crossing reads it
//!
//! The task also said the bytes carry no marker, because `ekr.graph.Space` sits on `GraphRoot` and
//! not on a node. That is true **of a node** and false of the thing this crossing is handed: a
//! [`GraphDocument`] is a [`GraphRoot`] plus four maps, and the root carries both `space` and
//! `schema_version_id`. The first implementation copied the root through unread, so a document
//! declaring `Transient` became canonical state, and a store opened against another schema version
//! silently re-typed every record it folded. Found by the adversary of wave p1-05, pass 1; the task
//! is corrected and this module is where the correction lives.
//!
//! So the crossing refuses rather than merely converts. What it checks is **every field of a
//! document that can be read against another field of the same document, or against what the
//! caller supplied** — which is the class those two defects belong to, rather than the two of them:
//!
//! | field | read against | refusal |
//! |---|---|---|
//! | `root.space` | `Space::Canonical` | [`MembraneError::Space`] |
//! | `root.schema_version_id` | the caller's `ontology.version().id` | [`MembraneError::SchemaVersion`] |
//! | `root.parent` | `root.id` | [`MembraneError::SelfParentedRoot`] |
//! | each entity's `root_id` | `root.id` | [`MembraneError::Misrooted`] |
//! | each map key | the entity's own id | [`MembraneError::Misfiled`] |
//! | each value, at every depth | [`CanonicalValue::try_from`] | [`MembraneError::Node`] and its two siblings |
//!
//! `root.parent` was on the other list for one round, and the adversary of pass 2 moved it: it is a
//! `GraphRootId` and `root.id` is three lines above it in the same struct, so a root that is its own
//! parent is a document disagreeing with itself and the comparison is the one already made four
//! times below. Not "cannot be checked" — **cannot be resolved**, which is the narrower and true
//! claim: nothing here can say whether some *other* root exists, only that this one is not itself.
//!
//! # What this crossing does not do, and where it is done instead
//!
//! Two fields are genuinely copied through unread, and that is a bound rather than an omission:
//! `root.created_at` has nothing in the document to disagree with, and `revision` is the fold's to
//! set.
//!
//! Everything else a reader might expect here is **placed elsewhere on purpose**, and is design
//! § 20's list of deterministic validators, which AGENTS.md invariant 7 and the roadmap's crate map
//! both put in `ekr-kernel` above this crate:
//!
//! * an assertion citing evidence the document does not carry, an edge whose `source` or `target`
//!   is not among its nodes, an assertion whose subject or object is not — the **reference**
//!   validator;
//! * a node or edge whose `type_id` the ontology does not declare, a property key that type has no
//!   definition for, a value whose kind does not match the definition — the **type** validator;
//! * a `type_state` naming a state its type's lifecycle does not have — the **ontology-constraint**
//!   validator.
//!
//! The ontology is in hand at this crossing and every one of those is answerable from it. They are
//! still not answered here, because a store that type-checked would be a second validator with no
//! way to report an issue and no transaction to refuse — and two validators that can disagree are
//! worse than one. The store's job is that what it reads back is what some writer wrote.
//!
//! # First error wins, and the evidence check is the odd one out
//!
//! A document with two faults reports one of them, and which one is a function of the order below
//! rather than of severity. Three of the four maps are checked for filing as they are narrowed;
//! **evidence is checked after all three**, because it carries no values to narrow and so has no
//! loop of its own to sit in. So a document with a misfiled evidence *and* a float in a node
//! reports the float.
//!
//! Both refusals are true of that document and the caller has to fix both either way, so this is
//! not a defect — but it is not a property worth relying on, and a caller reading one refusal
//! should not conclude it is the only one. If it ever needs to be uniform, the answer is to check
//! all four filings before narrowing anything, not to move the evidence loop.
//!
//! # What is still not carried by the bytes
//!
//! A **node**, on its own, still is not. `ekr.graph.Space` is on the root, so a node lifted out of
//! a transient document and pasted into a canonical one is indistinguishable from a canonical node
//! that carries no float: the crossing sees a document whose root says `Canonical` and has nothing
//! to object to. `tests/membrane_boundary.rs`'s
//! `a_candidate_node_and_a_canonical_node_write_the_same_document` pins exactly that residual hole
//! and nothing wider. Closing it is the task's second option — a space recorded per entity — and it
//! is not taken here.
//!
//! # What is not in a document
//!
//! The **ontology**. `ekr_ontology::Ontology` is held only through `Ontology::load` and publishes
//! no way back to the `OntologyDocument` it was loaded from, so this crate cannot write one down.
//! [`GraphDocument::into_canonical`] therefore takes the ontology from its caller, and checks it
//! against the version the document says it was written against — the same crate boundary that
//! leaves `Root.ontology_root` a placeholder in P1
//! (`task:two-of-the-five-revision-sub-roots-are-placeholders`).

use std::collections::BTreeMap;
use std::fmt;

use ekr_core::{
    AssertionId, EdgeId, EvidenceId, GraphRootId, NodeId, PropertyId, RevisionNumber,
    SchemaVersionId,
};
use ekr_graph::{
    Assertion, CanonicalGraph, CanonicalRef, CanonicalValue, Edge, Evidence, GraphRoot,
    InadmissibleValue, Node, Object, Space, Subject,
};
use ekr_ontology::{Ontology, Value};
use serde::{Deserialize, Serialize};

use crate::StoreError;

/// A graph written down: `ekr.store.Snapshot`'s materialised fold, as bytes.
///
/// Holds the **transient** instantiation of every graph type, which is what makes the crossing a
/// place rather than a habit. See the module documentation for why that is the shape.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
    /// Reads into this type and **never** into a canonical one; [`Self::into_canonical`] is the
    /// crossing.
    ///
    /// # Errors
    ///
    /// [`StoreError::Document`] when the bytes are not a document.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StoreError> {
        serde_json::from_slice(bytes).map_err(|error| StoreError::Document(error.to_string()))
    }

    /// The one named crossing from a document into canonical state.
    ///
    /// Reads the document against itself and against `ontology` before converting anything: the
    /// table in this module's documentation is the whole of what is checked and the whole of what
    /// is not. Every value is then offered to [`CanonicalValue::try_from`], which refuses the one
    /// kind canonical state does not admit; a refusal names the entity and the property it sat on,
    /// because a caller holding a graph of a thousand nodes cannot act on "a float is in here
    /// somewhere".
    ///
    /// # Errors
    ///
    /// [`MembraneError`], naming the field that disagreed or the value that is inadmissible.
    pub fn into_canonical(self, ontology: Ontology) -> Result<CanonicalGraph, MembraneError> {
        // The root first, because a document that belongs to another space or another schema
        // version is one whose contents there is no point converting.
        if self.root.space != Space::Canonical {
            return Err(MembraneError::Space {
                declared: self.root.space,
            });
        }
        if self.root.schema_version_id != ontology.version().id {
            return Err(MembraneError::SchemaVersion {
                written_against: self.root.schema_version_id,
                opened_with: ontology.version().id,
            });
        }
        if self.root.parent == Some(self.root.id) {
            return Err(MembraneError::SelfParentedRoot {
                root_id: self.root.id,
            });
        }

        let root_id = self.root.id;
        let mut nodes = BTreeMap::new();
        for (key, node) in self.nodes {
            check_filing(
                Entity::Node(key),
                Entity::Node(node.id),
                node.root_id,
                root_id,
            )?;
            nodes.insert(key, narrow_node(node)?);
        }
        let mut edges = BTreeMap::new();
        for (key, edge) in self.edges {
            check_filing(
                Entity::Edge(key),
                Entity::Edge(edge.id),
                edge.root_id,
                root_id,
            )?;
            edges.insert(key, narrow_edge(edge)?);
        }
        let mut assertions = BTreeMap::new();
        for (key, assertion) in self.assertions {
            check_filing(
                Entity::Assertion(key),
                Entity::Assertion(assertion.id),
                assertion.root_id,
                root_id,
            )?;
            assertions.insert(key, narrow_assertion(assertion)?);
        }
        for (key, evidence) in &self.evidence {
            // Evidence carries no `root_id` — it is cited by assertions rather than owned by a
            // root — so only its filing is checkable here.
            if *key != evidence.id {
                return Err(MembraneError::Misfiled {
                    key: Entity::Evidence(*key),
                    found: Entity::Evidence(evidence.id),
                });
            }
        }

        Ok(CanonicalGraph {
            root: self.root,
            revision: self.revision,
            ontology,
            nodes,
            edges,
            assertions,
            evidence: self.evidence,
        })
    }
}

/// An entity is filed under its own id, and belongs to the root the document hangs off.
///
/// Two checks rather than one function per map: a map whose key disagrees with the entity's own id
/// makes `CanonicalGraph::resolve` answer with a record that is not the one asked for, and an
/// entity carrying another root's id is another root's state wearing this one's document.
fn check_filing(
    key: Entity,
    found: Entity,
    entity_root: GraphRootId,
    document_root: GraphRootId,
) -> Result<(), MembraneError> {
    if key != found {
        return Err(MembraneError::Misfiled { key, found });
    }
    if entity_root != document_root {
        return Err(MembraneError::Misrooted {
            entity: found,
            root_id: entity_root,
            document_root,
        });
    }
    Ok(())
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
            .map(|(id, value)| (*id, Value::from(value.clone())))
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
            .map(|(id, value)| (*id, Value::from(value.clone())))
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
        validation: assertion.validation.clone(),
        valid_time: assertion.valid_time,
        transaction_time: assertion.transaction_time,
    }
}

/// A candidate's node, as canonical state — or the refusal that says why it is not.
fn narrow_node(node: Node<Value>) -> Result<Node<CanonicalValue>, MembraneError> {
    let node_id = node.id;
    let mut properties = BTreeMap::new();
    for (property, value) in node.properties {
        let value = CanonicalValue::try_from(value).map_err(|cause| MembraneError::Node {
            node_id,
            property,
            cause,
        })?;
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
/// **A reference is minted here, and minting one is not resolving it.** The document carries two
/// node ids and this is where they become `CanonicalRef<Node>` — the one place in the workspace
/// where a canonical reference is made from bytes rather than from a node something already held.
/// The type holds inside Rust and stops here.
///
/// **Nothing on this path refuses a dangling one.** This crossing is reached from
/// [`RevisionLog::fold`](crate::RevisionLog::fold), which is below `ekr-kernel`, so the reference
/// validator that refuses an unresolvable id on the *transaction* path is not reachable from here
/// in any process — and a document naming an edge target it does not carry becomes canonical state
/// holding a reference to nothing. `tests/adversary_p1_06_reference_from_bytes.rs` measures exactly
/// that; `review-result:adversary-eventlog-store-pass-1`'s finding C is what closes it, and is not
/// this wave's. A second validator *here* is worse than one validator in the right place, which is
/// the section "What this crossing does not do" above.
/// `task:the-membrane-stops-at-the-store-boundary` records the same limit for the value direction.
fn narrow_edge(edge: Edge<Value>) -> Result<Edge<CanonicalValue>, MembraneError> {
    let edge_id = edge.id;
    let mut properties = BTreeMap::new();
    for (property, value) in edge.properties {
        let value = CanonicalValue::try_from(value).map_err(|cause| MembraneError::Edge {
            edge_id,
            property,
            cause,
        })?;
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
fn narrow_assertion(
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
            Subject::Edge(edge) => Subject::Edge(edge),
            Subject::Type(type_id) => Subject::Type(type_id),
        },
        predicate: assertion.predicate,
        object,
        evidence: assertion.evidence,
        proposed_by: assertion.proposed_by,
        validation: assertion.validation,
        valid_time: assertion.valid_time,
        transaction_time: assertion.transaction_time,
    })
}
