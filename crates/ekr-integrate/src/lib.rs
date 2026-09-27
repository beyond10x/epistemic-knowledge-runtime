//! Integration: typed references resolved against one canonical snapshot.
//!
//! Implements the resolver of the `ekr.integrate` domain, `systems/ekr/domains/integrate.yaml`
//! (design § 10, § 45, amendment A10 in `docs/predecessors.md` § 2). A [`TypedReference`] names a
//! node by its type and the aliases it is known by; [`resolve`] answers it against one
//! [`GraphSnapshot`] with a [`ResolutionOutcome`], and writes nothing.
//!
//! # The rule
//!
//! A candidate is a node of the snapshot's canonical root whose `type_id` equals the reference's
//! `type_id` exactly and whose `aliases` hold one of the reference's aliases, compared byte for
//! byte. `canonical_name` is never compared: AGENTS.md invariant 3, names are not identities.
//!
//! | Candidates | Outcome |
//! |---|---|
//! | one | [`ResolutionOutcome::Resolved`] |
//! | none | [`ResolutionOutcome::ProposeNew`], carrying the type and the aliases; the caller mints the id |
//! | more than one | [`ResolutionOutcome::Ambiguous`], every candidate in id order; none is chosen |
//!
//! Refused before any node is read ([`ResolutionRefusalCode`]):
//!
//! * `reference-without-identity` — the reference holds no alias other than the empty string,
//!   which identifies nothing: an empty alias is never compared and never proposed;
//! * `reference-type-undeclared` — the snapshot's ontology declares no node type with the
//!   reference's `type_id`, so no node of it could be admitted;
//! * `reference-type-has-subtypes` — the reference's type is abstract or has a declared descendant.
//!   Whether such a reference should match its descendants' nodes is
//!   `decision-blocker:typed-reference-subtype-matching`; until it is answered, it is refused.
//!
//! The checks run in that order; the first that applies is the answer.
//!
//! Identifying keys (`decision-blocker:typed-reference-identifying-keys`) are not a field of
//! `ekr.integrate.TypedReference`, so a reference carrying them is not representable here.
//!
//! # Deterministic
//!
//! The outcome depends on the set of the reference's aliases, not their order or repetition: the
//! aliases a [`ResolutionOutcome::ProposeNew`] or a [`ResolutionRefusal`] carries are sorted
//! byte-wise and deduplicated. A refusal echoes the reference as given, empty alias included; a
//! proposal carries only the aliases that identify. Candidates are read from the snapshot's
//! id-ordered node map. Two permutations of one input give byte-identical outcomes.
//!
//! # No writer
//!
//! AGENTS.md invariant 1: this crate reads a snapshot and holds no writer. A new node is a
//! proposal, committed only through a transaction the kernel validates.
//!
//! ```
//! use std::collections::BTreeMap;
//!
//! use ekr_core::{GraphRootId, NodeId, RevisionNumber, SchemaVersionId, Timestamp, TypeId};
//! use ekr_graph::{CanonicalGraph, GraphRoot, GraphSnapshot, Node, Space};
//! use ekr_integrate::{resolve, ResolutionOutcome, ResolvedReference, TypedReference};
//! use ekr_ontology::{NodeType, Ontology, OntologyDocument, SchemaVersion};
//!
//! let (root_id, schema) = (GraphRootId::mint(), SchemaVersionId::mint());
//! let (person, organisation) = (TypeId::mint(), TypeId::mint());
//! let mut someone = Node::new(NodeId::mint(), root_id, person, "Meridian");
//! someone.aliases = vec!["meridian".to_owned()];
//! let mut company = Node::new(NodeId::mint(), root_id, organisation, "Meridian");
//! company.aliases = vec!["meridian".to_owned()];
//! let someone_id = someone.id;
//!
//! let graph = CanonicalGraph {
//!     root: GraphRoot {
//!         id: root_id,
//!         space: Space::Canonical,
//!         schema_version_id: schema,
//!         parent: None,
//!         created_at: Timestamp::EPOCH,
//!     },
//!     revision: RevisionNumber::new(1),
//!     ontology: Ontology::load(OntologyDocument {
//!         version: SchemaVersion::seed(schema, Timestamp::EPOCH),
//!         node_types: vec![
//!             NodeType::new(person, "person"),
//!             NodeType::new(organisation, "organisation"),
//!         ],
//!         edge_types: Vec::new(),
//!     })?,
//!     nodes: [(someone.id, someone), (company.id, company)].into_iter().collect(),
//!     edges: BTreeMap::new(),
//!     assertions: BTreeMap::new(),
//!     evidence: BTreeMap::new(),
//! };
//!
//! // One name, two types: the reference to a person finds the person and nothing else.
//! let reference = TypedReference { type_id: person, aliases: vec!["meridian".to_owned()] };
//! assert_eq!(
//!     resolve(GraphSnapshot::of(&graph), &reference),
//!     ResolutionOutcome::Resolved(ResolvedReference { node_id: someone_id }),
//! );
//! # Ok::<(), ekr_ontology::OntologyError>(())
//! ```

use ekr_core::{NodeId, TypeId};
use ekr_graph::GraphSnapshot;
use serde::{Deserialize, Serialize};

/// A reference to a node by its type and the names it is known by, before it is resolved:
/// `ekr.integrate.TypedReference` (`integrate.yaml`, lines 32–38).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedReference {
    /// The node type the referenced node is an instance of, exactly.
    pub type_id: TypeId,
    /// The names the referenced node is known by, each compared byte for byte.
    pub aliases: Vec<String>,
}

/// The one canonical node a reference resolved to: `ekr.integrate.ResolvedReference`
/// (`integrate.yaml`, lines 42–46).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedReference {
    /// The node.
    pub node_id: NodeId,
}

/// Every candidate a reference matched, in id order, with none chosen:
/// `ekr.integrate.AmbiguousReference` (`integrate.yaml`, lines 49–53).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AmbiguousReference {
    /// The candidates, ascending by id.
    pub candidates: Vec<NodeId>,
}

/// Why a reference cannot be resolved at all: `ekr.integrate.ResolutionRefusalCode`
/// (`integrate.yaml`, lines 56–61).
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ResolutionRefusalCode {
    /// The reference holds no alias but the empty string, so nothing identifies the node it means.
    #[serde(rename = "reference-without-identity")]
    ReferenceWithoutIdentity,
    /// The reference's type is abstract or has a declared descendant.
    #[serde(rename = "reference-type-has-subtypes")]
    ReferenceTypeHasSubtypes,
    /// The reference's type is not a node type the snapshot's ontology declares.
    #[serde(rename = "reference-type-undeclared")]
    ReferenceTypeUndeclared,
}

impl ResolutionRefusalCode {
    /// The code as the domain spells it.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::ReferenceWithoutIdentity => "reference-without-identity",
            Self::ReferenceTypeHasSubtypes => "reference-type-has-subtypes",
            Self::ReferenceTypeUndeclared => "reference-type-undeclared",
        }
    }
}

/// A refused reference and why: `ekr.integrate.ResolutionRefusal` (`integrate.yaml`, lines 63–69).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolutionRefusal {
    /// Why.
    pub code: ResolutionRefusalCode,
    /// The reference, its aliases sorted and deduplicated.
    pub reference: TypedReference,
}

/// What resolving one typed reference answers: `ekr.integrate.ResolutionOutcome`
/// (`integrate.yaml`, lines 73–80), tagged by `kind`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ResolutionOutcome {
    /// Exactly one candidate.
    Resolved(ResolvedReference),
    /// No candidate: a new node of this type with these aliases is proposed. Nothing is written.
    ProposeNew(TypedReference),
    /// More than one candidate.
    Ambiguous(AmbiguousReference),
    /// The reference cannot be resolved.
    Refused(ResolutionRefusal),
}

/// Resolves `reference` against `snapshot`. Reads only; see the [crate] documentation for the rule.
#[must_use]
pub fn resolve(snapshot: GraphSnapshot<'_>, reference: &TypedReference) -> ResolutionOutcome {
    let mut aliases = reference.aliases.clone();
    aliases.sort_unstable();
    aliases.dedup();
    let reference = TypedReference {
        type_id: reference.type_id,
        aliases,
    };
    let graph = snapshot.graph();

    let refused = |code| {
        ResolutionOutcome::Refused(ResolutionRefusal {
            code,
            reference: reference.clone(),
        })
    };
    let identifying = TypedReference {
        type_id: reference.type_id,
        aliases: reference
            .aliases
            .iter()
            .filter(|alias| !alias.is_empty())
            .cloned()
            .collect(),
    };
    if identifying.aliases.is_empty() {
        return refused(ResolutionRefusalCode::ReferenceWithoutIdentity);
    }
    let Some(declared) = graph.ontology.node_type(reference.type_id) else {
        return refused(ResolutionRefusalCode::ReferenceTypeUndeclared);
    };
    let is_abstract = declared.abstract_type;
    let has_descendant = graph
        .ontology
        .to_document()
        .node_types
        .iter()
        .any(|declared| declared.parents.contains(&reference.type_id));
    if is_abstract || has_descendant {
        return refused(ResolutionRefusalCode::ReferenceTypeHasSubtypes);
    }

    let candidates: Vec<NodeId> = graph
        .nodes
        .values()
        .filter(|node| node.root_id == graph.root.id && node.type_id == reference.type_id)
        .filter(|node| {
            node.aliases
                .iter()
                .any(|alias| identifying.aliases.binary_search(alias).is_ok())
        })
        .map(|node| node.id)
        .collect();
    match candidates.as_slice() {
        [] => ResolutionOutcome::ProposeNew(identifying),
        [node_id] => ResolutionOutcome::Resolved(ResolvedReference { node_id: *node_id }),
        _ => ResolutionOutcome::Ambiguous(AmbiguousReference { candidates }),
    }
}
