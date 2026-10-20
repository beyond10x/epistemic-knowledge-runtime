//! Applying an extraction document to a store (`story:extraction-verb-shares-the-sdk-path`).
//!
//! [`apply`] and [`apply_with`] are the one routine that turns an [`ExtractionDocument`] into
//! committed transactions. `ekr apply-extraction` runs it in process, over a transport that
//! answers each request through the session's own dispatch; a consumer runs it over a child
//! `ekr session` ([`crate::session::ProcessSession`]). One algorithm, two transports: both write
//! the same store.
//!
//! First it reads `ekr snapshot`, for the graph root new nodes belong to, the evidence and the
//! assertions the store holds, and `ekr ontology`, for the store's types by name. Then, before it
//! writes anything, it refuses — by the engine reader's code, as [`ApplyError::Refused`] — a
//! document [`ExtractionDocument::check_facts`] refuses (two evidence items under one id, a payload
//! that does not hash to its entry); an entity with no alias but the empty string
//! (`reference-without-identity`); and an entity whose node type is abstract or has a subtype
//! once the document's ontology is applied (`reference-type-has-subtypes`), which `ekr resolve`
//! would refuse. A document [`Ontology::ensure`] refuses to plan — a subtype redeclaring a
//! property an ancestor declares otherwise — is [`ApplyError::Ontology`], before any write too; the
//! engine's reader refuses it first, as `extraction-property-conflict`. Every type, property and
//! relation the document names is looked up then too, so an entity's type no declaration gives is
//! [`ApplyError::Undeclared`] before anything is written.
//!
//! A bad fact is skipped, and the rest of the document applies (`story:extraction-partial-apply`):
//! one citing an id the document's evidence does not carry (`fact-evidence-unlisted`), one whose
//! subject or object a named thing above would refuse, one naming a property or edge type no
//! declaration gives (`extraction-property-undeclared`, `extraction-type-undeclared`), a relation
//! between node types its edge type does not connect, subtypes conforming
//! (`extraction-relation-ends`), and one the caller's own reader refused
//! ([`ApplyOptions::refused`]). A property and a relation's ends are checked against the type of
//! the node the key resolves to, which a supersession record can make a node of another type
//! (`story:supersede-node-review-fixes`, D6 and Q4): such a key takes the kept node's properties
//! and edge ends, a fact the kept type does not admit is skipped before anything only it names is
//! created, and its refusal names the supersession of each end that caused it. A key is checked
//! alone and again as the group it is resolved with, and a fact the group's type does not admit
//! is skipped the same way. So is a `!Property` fact marked `replaces` that neither the store —
//! on the nodes `ekr resolve` would answer its subject with — nor an earlier fact of the document
//! holds an active value of its property for ([`REPLACES_NOTHING`]). It is listed under
//! [`ExtractionReport::rejected`] with its refusal, and nothing only it names is resolved, created
//! or added. [`ApplyOptions::strict`] refuses the whole document for it instead, as [`apply`] did
//! before — a fact a kept node's type does not admit included, whether its key alone or its group
//! lands there — but a replacement with nothing to replace, which depends on the store, is skipped
//! under it too.
//!
//! In order, over `transport`:
//!
//! 1. the document's ontology, as [`Ontology::ensure`] finds it missing, committed as one schema
//!    change. If it is rejected, nothing more is applied: what the document names by it does not
//!    exist;
//! 2. every named thing — each entity, then each fact's subject and object. Named things of one
//!    node type that share an alias are one thing, whichever order the document lists them in:
//!    each such group is resolved once through one [`Resolver`], as a typed reference carrying
//!    every alias of its members. A group the store holds is used; one it does not is created as
//!    a node of its type named by the least, in byte order, of its members' first aliases; one
//!    the store answers with several nodes is [ambiguous](ExtractionReport::ambiguous): nothing
//!    is chosen;
//! 3. every fact about things that resolved, as one `!AddAssertion` each — `!Property` with the
//!    value, `!Relation` with the object node — valid from the earliest `observed_at` of the
//!    evidence items it cites, with no end (`story:extraction-valid-time`), and committed through
//!    a [`Batcher`] with the document's evidence: each item a fact cites is added with the first
//!    fact citing it, unless the store already holds its id. An item no fact cites is not added.
//!    A `!Relation` fact also writes a `!CreateEdge` of its edge type from the subject to the
//!    object node, in the same group as its assertion, so the reads that walk the graph's edges
//!    see it (`story:extracted-relations-visible-to-graph-reads`); none when the store, or an
//!    earlier fact that committed, already joins the two by an edge of that type. An edge the
//!    kernel refuses — its endpoint types, its type's cardinality — refuses the fact, assertion
//!    and all. A `!Property` fact marked `replaces` also supersedes, in its group, every active
//!    assertion of its subject and property — the store's, or an earlier committed fact's — from
//!    its own valid time on (`story:extraction-supersession`); with none left to supersede, as
//!    when the earlier fact that set one was rejected, it is refused alone, as
//!    [`REPLACES_NOTHING`].
//!
//!    Facts are committed in rounds. What the run records as done — the active values, the edges
//!    and the claims — changes only when a fact's group commits, so a fact that depends on an
//!    earlier fact of the round waits for the next one, and is planned once that fact is known
//!    to have committed or not: a fact repeating its claim and citing no evidence it does not
//!    (so it could be held by it), a relation repeating one whose edge it proposes, and a fact on
//!    the same subject and property where either replaces. Any other corroboration of a claim
//!    commits in the same round. A fact repeating a rejected one is tried on its own and
//!    reported on its own result.
//!
//!    A fact is [held](ExtractionReport::held), not asserted, when an assertion making its claim
//!    — the same subject, predicate and object — covers it: it cites every evidence item the fact
//!    cites, holds from no later than the fact does (an unbounded start, as this routine wrote one
//!    before facts had a valid time, covering every start) and ends where the fact does. It is,
//!    in that order, one the store holds active, one an earlier fact of the document committed,
//!    or one the store holds retracted or superseded, which asserting it again from the same
//!    evidence would undo. So a document applied a second time adds no node, evidence entry or
//!    assertion, and neither does a fact citing part of an earlier claim's evidence. A held fact
//!    marked `replaces` still supersedes every other active value of its subject and property,
//!    by the active assertion that covers it, from that assertion's start; when that assertion
//!    holds from an unbounded past, the fact is asserted anew instead and supersedes every active
//!    value. Such a fact is reported held once its supersession commits, and only as rejected if
//!    that is rejected. A fact citing evidence no such assertion cited is asserted.
//!
//! The only requests that write are `propose`, `validate` and `commit`; the rest are `snapshot`,
//! `ontology`, `head` and `resolve`. Once something has committed, the routine does not end in an
//! error: a request that then gets no answer it can act on stops it, and the report names why
//! ([`ExtractionReport::stopped`]) beside every transaction committed until then, the ones the
//! batch or flush that stopped committed included.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AgentId, AssertionId, EvidenceId, NodeId, Timestamp, TransactionId, TypeId};
use serde::Serialize;

use crate::batch::{BatchError, BatchReport, Batcher, CallError, Rejection};
use crate::document::{
    extraction_codes as code, Assertion, EdgeDraft, ExtractedFact, ExtractedReference,
    ExtractionDocument, ExtractionRefusal, Object, Ontology, OntologyError, Operation, Predicate,
    PropertyId, SchemaChange, Subject, Supersession, TemporalRange, TypedReference,
    ValidationProfile,
};
use crate::evidence::EvidenceSet;
use crate::read::{ReadError, Reader, SnapshotPredicate, SnapshotSubject};
use crate::reply::{Answer, Outcome};
use crate::resolve::{Resolution, Resolver};
use crate::transport::{Request, Transport};

/// What applying one extraction document did: `ekr.integrate.ExtractionReport`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct ExtractionReport {
    /// Every transaction committed, in the order it was committed: the schema change, the new
    /// nodes, then the facts.
    pub committed: Vec<CommittedExtraction>,
    /// Every part of the document that was not applied, in the order it was tried.
    pub rejected: Vec<RejectedExtraction>,
    /// Every named thing the store answers with more than one node, once each.
    pub ambiguous: Vec<AmbiguousExtraction>,
    /// Every fact not asserted, and why: the store holds an assertion of the same claim citing all
    /// of its evidence, active, retracted or superseded, or an earlier fact of the document says
    /// the same from the same evidence.
    pub held: Vec<HeldExtraction>,
    /// Why applying stopped before the end of the document, after something had committed;
    /// `None` when it went to its end.
    pub stopped: Option<String>,
}

/// A fact not asserted: `ekr.integrate.HeldExtraction`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct HeldExtraction {
    /// The fact: `facts[<index>]`.
    pub item: String,
    /// Why.
    pub reason: HeldReason,
}

/// Why a fact is not asserted: `ekr.integrate.HeldReason`. In each case the assertion making its
/// claim — the same subject, predicate, object and valid time — cites every evidence item the
/// fact cites; a fact citing evidence no such assertion cited is asserted.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum HeldReason {
    /// The store holds it, active.
    Asserted,
    /// An earlier fact of the document says it.
    Repeated,
    /// The store held it and it was retracted: asserting it again would undo that decision.
    Retracted,
    /// The store held it and it was superseded: asserting it again would undo that decision.
    Superseded,
}

/// One committed transaction: `ekr.integrate.CommittedExtraction`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct CommittedExtraction {
    /// Its id.
    pub transaction_id: TransactionId,
    /// The revision it published.
    pub revision: u64,
}

/// One part of the document that was not applied: `ekr.integrate.RejectedExtraction`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct RejectedExtraction {
    /// What of the document: `ontology`, `entities[<index>]`, `facts[<index>]` or
    /// `facts[<index>].subject` / `.object` for the first place a named thing appears.
    pub item: String,
    /// The transaction that validation rejected, holding that part alone; `None` when no
    /// transaction was validated.
    pub transaction_id: Option<TransactionId>,
    /// The validators' issues; empty when no transaction was validated.
    pub issues: Vec<ExtractionIssue>,
    /// Why it was not tried or not proposed, when no validator answered: a refusal of
    /// `ekr propose` as `<code>: <reason>`, a document limit, a named thing it rests on that
    /// was not created, a fact skipped by the engine reader's code as `<code>: <name>`, or a
    /// replacement with no active assertion to replace ([`REPLACES_NOTHING`]).
    pub refusal: Option<String>,
}

/// One validator issue, as `ekr validate` printed it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ExtractionIssue {
    /// The validator that raised it.
    pub validator: String,
    /// Its stable code.
    pub code: String,
    /// What was wrong.
    pub message: String,
}

/// A named thing the store answers with more than one node: `ekr.integrate.AmbiguousExtraction`.
/// No fact about it is applied.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct AmbiguousExtraction {
    /// The thing: its node type and every alias of the named things it groups, the name first.
    pub reference: ExtractedReference,
    /// Every node it matched, in id order.
    pub candidates: Vec<NodeId>,
}

/// Why [`apply`] wrote nothing, or stopped before anything committed.
#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    /// The document is one the engine's reader refuses, by its code; nothing was written.
    #[error(transparent)]
    Refused(#[from] ExtractionRefusal),
    /// `ekr snapshot` did not answer with a snapshot.
    #[error(transparent)]
    Read(#[from] ReadError),
    /// `ekr ontology` did not read, or the document's ontology is not one a schema change makes.
    #[error(transparent)]
    Ontology(#[from] OntologyError),
    /// The document names a type or property the store does not declare, nor the document.
    #[error("the store declares no {kind} named {name:?}, and the document does not either")]
    Undeclared {
        /// `node type`, `edge type` or `property`.
        kind: &'static str,
        /// The name, `Type.property` for a property.
        name: String,
    },
    /// A request got no answer the SDK can act on.
    #[error(transparent)]
    Call(#[from] CallError),
    /// A batch stopped before its end.
    #[error(transparent)]
    Batch(#[from] Box<BatchError>),
}

/// The code of a [`RejectedExtraction::refusal`] for a fact marked `replaces` whose subject and
/// property the store holds no active assertion of (`story:extraction-supersession`).
pub const REPLACES_NOTHING: &str = "replacement-without-active-assertion";

/// A named thing's cache key: its type's name and its distinct non-empty aliases, sorted.
type Key = (String, Vec<String>);

/// What a group of named things came to.
#[derive(Clone, Debug)]
enum Thing {
    /// A node the store holds, or one created for it.
    Node(NodeId),
    /// A node queued for creation, not yet committed.
    Queued(NodeId),
    /// More than one node; nothing chosen.
    Ambiguous,
    /// A node that was to be created and was not.
    Missing,
}

/// Named things of one node type that share an alias, transitively: one thing.
#[derive(Clone, Debug)]
struct Group {
    /// The node type's name.
    node_type: String,
    /// Its id once the document's ontology is applied.
    type_id: TypeId,
    /// The name first, then every other alias of its members, sorted and distinct.
    aliases: Vec<String>,
    /// Where a member first appears.
    site: String,
}

/// A fact's predicate and object, looked up before anything is written.
#[derive(Clone, Debug)]
enum Said {
    /// `!Property` of this property.
    Property(PropertyId),
    /// `!Relation` of this edge type, to the object's group.
    Relation(TypeId, usize),
}

/// What a fact names, looked up before its subject and object are grouped.
#[derive(Clone, Copy, Debug)]
enum Lookup {
    /// `!Property` of this property.
    Property(PropertyId),
    /// `!Relation` of this edge type.
    Relation(TypeId),
}

/// Why a reference or a fact cannot be planned: a refusal by the engine reader's code, or a name
/// no declaration gives.
#[derive(Debug)]
enum Defect {
    Refused(ExtractionRefusal),
    Undeclared { kind: &'static str, name: String },
}

impl Defect {
    /// The defect as a refusal of the whole document: [`ApplyError::Refused`] or
    /// [`ApplyError::Undeclared`].
    fn into_error(self) -> ApplyError {
        match self {
            Self::Refused(refusal) => refusal.into(),
            Self::Undeclared { kind, name } => ApplyError::Undeclared { kind, name },
        }
    }

    /// The defect as one fact's refusal, by the code the engine reader refuses it with: a property
    /// no declaration gives is `extraction-property-undeclared`, a node or edge type
    /// `extraction-type-undeclared`.
    fn into_refusal(self) -> ExtractionRefusal {
        match self {
            Self::Refused(refusal) => refusal,
            Self::Undeclared {
                kind: "property",
                name,
            } => ExtractionRefusal::new(code::PROPERTY_UNDECLARED, name),
            Self::Undeclared { name, .. } => ExtractionRefusal::new(code::TYPE_UNDECLARED, name),
        }
    }
}

/// The node type `reference`, at `site`, names: refused when it has no alias but the empty
/// string, its type is undeclared, or its type is abstract or has a subtype.
fn reference_type(
    site: &str,
    reference: &ExtractedReference,
    ontology: &Ontology,
) -> Result<TypeId, Defect> {
    let key = key_of(reference);
    if key.1.is_empty() {
        return Err(Defect::Refused(ExtractionRefusal::new(
            code::WITHOUT_IDENTITY,
            site,
        )));
    }
    let Some(type_id) = ontology.node_type(&key.0) else {
        return Err(Defect::Undeclared {
            kind: "node type",
            name: key.0,
        });
    };
    if ontology.has_subtypes(&key.0) {
        return Err(Defect::Refused(ExtractionRefusal::new(
            code::HAS_SUBTYPES,
            format!("{site}: {}", key.0),
        )));
    }
    Ok(type_id)
}

/// What `fact` names, looked up: its property, on the type of the node its subject's key
/// resolves to ([`KeptTypes`]), or its edge type.
fn lookup(fact: &ExtractedFact, ontology: &Ontology, kept: &KeptTypes) -> Result<Lookup, Defect> {
    match fact {
        ExtractedFact::Property(fact) => {
            let landed = kept.of(&fact.subject.node_type, &fact.subject.aliases);
            property_on(
                ontology,
                &fact.subject.node_type,
                landed.as_ref(),
                &fact.property,
            )
        }
        ExtractedFact::Relation(fact) => ontology
            .edge_type(&fact.relation)
            .map(Lookup::Relation)
            .ok_or_else(|| Defect::Undeclared {
                kind: "edge type",
                name: fact.relation.clone(),
            }),
    }
}

/// The property `property` of a node of the type named `node_type` — or of `landed`'s, the type
/// of the node its key resolves to through a supersession — and, where that type declares none,
/// `extraction-property-undeclared`, naming the supersession when `landed` is the cause.
fn property_on(
    ontology: &Ontology,
    node_type: &str,
    landed: Option<&Kept>,
    property: &str,
) -> Result<Lookup, Defect> {
    let typed = landed.map_or(node_type, |kept| kept.node_type.as_str());
    ontology
        .property(typed, property)
        .map(Lookup::Property)
        .ok_or_else(|| Defect::Undeclared {
            kind: "property",
            name: noted(format!("{typed}.{property}"), [landed]),
        })
}

/// `name`, followed by the note of each supersession in `causes`.
fn noted<'k>(mut name: String, causes: impl IntoIterator<Item = Option<&'k Kept>>) -> String {
    for kept in causes.into_iter().flatten() {
        name.push_str("; ");
        name.push_str(&kept.note);
    }
    name
}

/// The named things `fact`, at `facts[<at>]`, rests on, each with its site: its subject, and a
/// relation's object.
fn fact_sites(at: usize, fact: &ExtractedFact) -> Vec<(String, &ExtractedReference)> {
    match fact {
        ExtractedFact::Property(fact) => vec![(format!("facts[{at}].subject"), &fact.subject)],
        ExtractedFact::Relation(fact) => vec![
            (format!("facts[{at}].subject"), &fact.subject),
            (format!("facts[{at}].object"), &fact.object),
        ],
    }
}

/// One fact checked as the engine's reader checks it, in its order: its subject, then its
/// property or edge type, then a relation's object, then whether the relation's edge type
/// connects the two ([`relation_ends`]). A property and an end are checked against the type of
/// the node the key resolves to ([`KeptTypes`]), which is the key's own type unless a
/// supersession record resolves it to a node of another type.
fn check_fact(
    at: usize,
    fact: &ExtractedFact,
    ontology: &Ontology,
    kept: &KeptTypes,
) -> Result<Lookup, Defect> {
    let sites = fact_sites(at, fact);
    reference_type(&sites[0].0, sites[0].1, ontology)?;
    let found = lookup(fact, ontology, kept)?;
    for (site, reference) in &sites[1..] {
        reference_type(site, reference, ontology)?;
    }
    relation_ends(at, fact, ontology, kept)?;
    Ok(found)
}

/// A relation, at `facts[<at>]`, between node types its edge type does not connect, subtypes
/// conforming: `extraction-relation-ends`, as the engine's reader refuses it, so the SDK refuses it
/// before anything is written rather than leave it to the kernel after its named things exist.
/// Each end is typed by the node its key resolves to (`story:supersede-node-review-fixes`, D6
/// and Q4), and the refusal names a supersession for each end it resolved through that does not
/// connect.
fn relation_ends(
    at: usize,
    fact: &ExtractedFact,
    ontology: &Ontology,
    kept: &KeptTypes,
) -> Result<(), Defect> {
    let ExtractedFact::Relation(fact) = fact else {
        return Ok(());
    };
    let subject = kept.of(&fact.subject.node_type, &fact.subject.aliases);
    let object = kept.of(&fact.object.node_type, &fact.object.aliases);
    ends_connect(
        at,
        ontology,
        &fact.relation,
        (&fact.subject.node_type, subject.as_ref()),
        (&fact.object.node_type, object.as_ref()),
    )
}

/// Whether the edge type named `relation` connects the subject's type to the object's, each the
/// key's own (`.0`) unless it resolves through a supersession (`.1`): `extraction-relation-ends`
/// otherwise, naming the supersession of each end that does not connect.
fn ends_connect(
    at: usize,
    ontology: &Ontology,
    relation: &str,
    subject: (&str, Option<&Kept>),
    object: (&str, Option<&Kept>),
) -> Result<(), Defect> {
    let typed = |(own, landed): (&str, Option<&Kept>)| {
        landed.map_or(own.to_owned(), |kept| kept.node_type.clone())
    };
    let (source, target) = (typed(subject), typed(object));
    let starts = ontology.connects_end(relation, &source, true);
    let ends = ontology.connects_end(relation, &target, false);
    if starts && ends {
        return Ok(());
    }
    let causes = [subject.1.filter(|_| !starts), object.1.filter(|_| !ends)];
    Err(Defect::Refused(ExtractionRefusal::new(
        code::RELATION_ENDS,
        noted(format!("facts[{at}]: {source} {relation} {target}"), causes),
    )))
}

/// A named thing's key that resolves, through a supersession record, to a kept node of another
/// type (`story:supersede-node`): the type its facts are checked against, and what a refusal that
/// end caused says of the supersession (`story:supersede-node-review-fixes`, D6 and Q4).
#[derive(Clone, Debug, PartialEq, Eq)]
struct Kept {
    /// The kept node's type, by name.
    node_type: String,
    /// The supersession, as a refusal it caused names it: the superseded node, its type and the
    /// kept node.
    note: String,
}

/// The store's alias holders, read once from `ekr snapshot`, for [`KeptTypes::of`]: empty, and
/// answering nothing, for a store that holds no supersession record.
#[derive(Debug, Default)]
struct KeptTypes<'a> {
    /// The store's ontology, as `ekr ontology` printed it.
    store: Option<&'a Ontology>,
    /// The graph read.
    graph: Option<&'a crate::read::SnapshotGraph>,
    /// Each `(type, alias)` a node of the head holds.
    held: BTreeSet<(TypeId, &'a str)>,
    /// Each `(type, alias)` a superseded node holds, with that node and the node it is superseded
    /// by.
    superseded: BTreeMap<(TypeId, &'a str), BTreeSet<(NodeId, NodeId)>>,
}

impl<'a> KeptTypes<'a> {
    /// The holders of `graph`'s aliases under `store`'s types, read only where `graph` holds a
    /// supersession record.
    fn read(graph: &'a crate::read::SnapshotGraph, store: &'a Ontology) -> Self {
        if graph.superseded.is_empty() {
            return Self::default();
        }
        let mut kept = Self {
            store: Some(store),
            graph: Some(graph),
            ..Self::default()
        };
        for node in graph.nodes.values() {
            if node.root_id == graph.root.id {
                for alias in &node.aliases {
                    kept.held.insert((node.type_id, alias.as_str()));
                }
            }
        }
        for record in graph.superseded.values() {
            if record.node.root_id == graph.root.id {
                for alias in &record.node.aliases {
                    kept.superseded
                        .entry((record.node.type_id, alias.as_str()))
                        .or_default()
                        .insert((record.node.id, record.by));
                }
            }
        }
        kept
    }

    /// Where a typed reference of the store's type named `node_type` and `aliases` resolves, as
    /// `ekr resolve` answers it, to exactly one node, and that node is of another type because a
    /// supersession record holds the key: the kept node's type and the note naming the
    /// supersession. `None` otherwise — the key resolves to a node of its own type, to none, to
    /// several, or names a type the store does not hold yet.
    fn of(&self, node_type: &str, aliases: &[String]) -> Option<Kept> {
        let (store, graph) = (self.store?, self.graph?);
        let type_id = store.node_type(node_type)?;
        let identifying = || aliases.iter().filter(|alias| !alias.is_empty());
        if identifying().any(|alias| self.held.contains(&(type_id, alias.as_str()))) {
            return None;
        }
        let records: BTreeSet<(NodeId, NodeId)> = identifying()
            .filter_map(|alias| self.superseded.get(&(type_id, alias.as_str())))
            .flatten()
            .copied()
            .collect();
        let kept_by: BTreeSet<NodeId> = records.iter().map(|(_, by)| *by).collect();
        if kept_by.len() != 1 {
            return None;
        }
        let &(superseded, by) = records.first()?;
        let kept = graph.nodes.get(&by)?.type_id;
        if kept == type_id {
            return None;
        }
        Some(Kept {
            node_type: store.node_type_name(kept)?.to_owned(),
            note: format!(
                "the {node_type} known as {aliases:?} resolved through the supersession of node \
                 {superseded} (type {node_type} {type_id}) by node {by} (type {kept}), which is of \
                 another type"
            ),
        })
    }
}

/// How [`apply_with`] treats a fact it cannot apply (`story:extraction-partial-apply`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ApplyOptions {
    /// Refuse the whole document on its first bad fact, before anything is written, as
    /// [`apply`] did before partial application: [`ApplyError::Refused`] or
    /// [`ApplyError::Undeclared`]. When `false`, a bad fact is skipped and listed under
    /// [`ExtractionReport::rejected`], and the rest of the document applies.
    pub strict: bool,
    /// Facts a reader the caller ran refused, each under its index in `facts`, as the engine's
    /// reader names it (`ExtractionDocument::check_facts` of `ekr-integrate`). Each is skipped
    /// with that refusal; under [`Self::strict`] the first refuses the document.
    pub refused: BTreeMap<usize, ExtractionRefusal>,
}

impl ApplyOptions {
    /// Refuse the whole document on its first bad fact.
    #[must_use]
    pub const fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Skip each of `refused`, under its index in `facts`, with its refusal.
    #[must_use]
    pub fn refusing(mut self, refused: BTreeMap<usize, ExtractionRefusal>) -> Self {
        self.refused = refused;
        self
    }
}

/// Everything [`apply`] looks up before it writes.
#[derive(Debug)]
struct Plan {
    groups: Vec<Group>,
    /// Each fact's subject group and what it says, in document order; `None` for a fact
    /// skipped.
    facts: Vec<Option<(usize, Said)>>,
    /// Each fact skipped, under its index, and why.
    skipped: BTreeMap<usize, ExtractionRefusal>,
}

impl Plan {
    /// The groups and lookups of `document` against `ontology`, the store's once the document's
    /// ontology is applied. Refuses what the module documentation lists, before any write: under
    /// `options.strict` a bad fact too; otherwise a bad fact — one of `options.refused`, then one
    /// whose named things or lookup [`check_fact`] refuses, then one of `unlisted` — is skipped,
    /// and nothing only it names is grouped. So is a replacement with nothing to replace, under
    /// `options.strict` too: `holds` answers whether a store node of a type, holding one of the
    /// aliases, has an active assertion of a property — the nodes `ekr resolve` would answer with.
    ///
    /// A fact's property and ends are checked against the type of the node its key resolves to,
    /// `kept` answering where a supersession record resolves a key to a node of another type
    /// (`story:supersede-node-review-fixes`, D6 and Q4). A named thing is resolved as its group,
    /// every alias of the keys sharing one, so once the groups are made a fact whose group lands on
    /// another type than its key alone does is checked again against the group's, and skipped if
    /// that type refuses it — or, under `options.strict`, the document refused, as for a key —
    /// before anything is resolved, so no node is created for a fact that is then refused.
    fn of(
        document: &ExtractionDocument,
        ontology: &Ontology,
        options: &ApplyOptions,
        mut unlisted: BTreeMap<usize, ExtractionRefusal>,
        holds: &dyn Fn(TypeId, &[String], PropertyId) -> bool,
        kept: &KeptTypes,
    ) -> Result<Self, ApplyError> {
        let entities: Vec<(String, &ExtractedReference)> = document
            .entities
            .iter()
            .enumerate()
            .map(|(at, entity)| (format!("entities[{at}]"), entity))
            .collect();
        for (site, reference) in &entities {
            reference_type(site, reference, ontology).map_err(Defect::into_error)?;
        }
        if options.strict {
            if let Some(refusal) = options.refused.values().next() {
                return Err(refusal.clone().into());
            }
            // In the order a document was refused before partial application: every named
            // thing, then every lookup.
            for (at, fact) in document.facts.iter().enumerate() {
                for (site, reference) in fact_sites(at, fact) {
                    reference_type(&site, reference, ontology).map_err(Defect::into_error)?;
                }
            }
            for fact in &document.facts {
                lookup(fact, ontology, kept).map_err(Defect::into_error)?;
            }
            for (at, fact) in document.facts.iter().enumerate() {
                relation_ends(at, fact, ontology, kept).map_err(Defect::into_error)?;
            }
        }

        let mut skipped = BTreeMap::new();
        let mut lookups = Vec::new();
        for (at, fact) in document.facts.iter().enumerate() {
            let checked = match options.refused.get(&at) {
                Some(refusal) => Err(refusal.clone()),
                None => check_fact(at, fact, ontology, kept)
                    .map_err(Defect::into_refusal)
                    .and_then(|found| unlisted.remove(&at).map_or(Ok(found), Err)),
            };
            match checked {
                Ok(found) => lookups.push(Some(found)),
                Err(refusal) => {
                    skipped.insert(at, refusal);
                    lookups.push(None);
                }
            }
        }

        // A replacement neither the store nor an earlier fact of the document holds an active
        // value for is refused here, before anything is resolved, so nothing only it names is
        // created (`story:extraction-supersession`). Refusing one can split a group, so the
        // groups are made again until no further replacement is refused.
        loop {
            let mut sites = entities.clone();
            for (at, fact) in document.facts.iter().enumerate() {
                if !skipped.contains_key(&at) {
                    sites.extend(fact_sites(at, fact));
                }
            }
            let (groups, of_key) = grouped(&sites, ontology)?;
            let landed: Vec<Option<Kept>> = groups
                .iter()
                .map(|group| kept.of(&group.node_type, &group.aliases))
                .collect();
            let mut refused = BTreeMap::new();
            let mut facts: Vec<Option<(usize, Said)>> = Vec::with_capacity(document.facts.len());
            for (at, (fact, found)) in document.facts.iter().zip(&lookups).enumerate() {
                let planned = match (fact, found) {
                    _ if skipped.contains_key(&at) => Ok(None),
                    (_, None) => Ok(None),
                    (ExtractedFact::Property(fact), Some(Lookup::Property(property))) => {
                        let subject = of_key[&key_of(&fact.subject)];
                        regrouped_property(
                            fact,
                            *property,
                            landed[subject].as_ref(),
                            ontology,
                            kept,
                        )
                        .map(|property| Some((subject, Said::Property(property))))
                    }
                    (ExtractedFact::Relation(fact), Some(Lookup::Relation(relation))) => {
                        let (subject, object) = (
                            of_key[&key_of(&fact.subject)],
                            of_key[&key_of(&fact.object)],
                        );
                        regrouped_ends(
                            at,
                            fact,
                            (landed[subject].as_ref(), landed[object].as_ref()),
                            ontology,
                            kept,
                        )
                        .map(|()| Some((subject, Said::Relation(*relation, object))))
                    }
                    _ => unreachable!("a lookup is of its fact's own kind"),
                };
                match planned {
                    Ok(planned) => facts.push(planned),
                    // A fact its group lands on another type for is refused as one its key does
                    // is: under `strict`, the whole document.
                    Err(defect) if options.strict => return Err(defect.into_error()),
                    Err(defect) => {
                        refused.insert(at, defect.into_refusal());
                        facts.push(None);
                    }
                }
            }

            let mut set: BTreeSet<(usize, PropertyId)> = BTreeSet::new();
            for (at, (fact, planned)) in document.facts.iter().zip(&facts).enumerate() {
                let (ExtractedFact::Property(fact), Some((subject, Said::Property(property)))) =
                    (fact, planned)
                else {
                    continue;
                };
                let group = &groups[*subject];
                if fact.replaces
                    && !set.contains(&(*subject, *property))
                    && !holds(group.type_id, &group.aliases, *property)
                {
                    refused.insert(
                        at,
                        ExtractionRefusal::new(
                            REPLACES_NOTHING,
                            nothing_to_replace(at, &fact.subject.node_type, &fact.property, group),
                        ),
                    );
                } else {
                    set.insert((*subject, *property));
                }
            }
            if refused.is_empty() {
                return Ok(Self {
                    groups,
                    facts,
                    skipped,
                });
            }
            skipped.extend(refused);
        }
    }
}

/// A property fact's property once its subject is grouped: `property`, the one its key alone was
/// checked for, unless its group, `group` answering, resolves to a node of another type than the
/// key alone does — then the property that type carries, or `extraction-property-undeclared`.
fn regrouped_property(
    fact: &crate::document::PropertyFact,
    property: PropertyId,
    group: Option<&Kept>,
    ontology: &Ontology,
    kept: &KeptTypes,
) -> Result<PropertyId, Defect> {
    let alone = kept.of(&fact.subject.node_type, &fact.subject.aliases);
    if alone.as_ref().map(|kept| &kept.node_type) == group.map(|kept| &kept.node_type) {
        return Ok(property);
    }
    match property_on(ontology, &fact.subject.node_type, group, &fact.property)? {
        Lookup::Property(property) => Ok(property),
        Lookup::Relation(_) => unreachable!("a property is looked up as a property"),
    }
}

/// A relation fact's ends once its subject and object are grouped: unchanged, unless a group
/// resolves to a node of another type than its key alone does — then checked again against the
/// groups' types, `extraction-relation-ends` if its edge type does not connect them.
fn regrouped_ends(
    at: usize,
    fact: &crate::document::RelationFact,
    (subject, object): (Option<&Kept>, Option<&Kept>),
    ontology: &Ontology,
    kept: &KeptTypes,
) -> Result<(), Defect> {
    let same = |reference: &ExtractedReference, group: Option<&Kept>| {
        kept.of(&reference.node_type, &reference.aliases)
            .map(|kept| kept.node_type)
            .as_ref()
            == group.map(|kept| &kept.node_type)
    };
    if same(&fact.subject, subject) && same(&fact.object, object) {
        return Ok(());
    }
    ends_connect(
        at,
        ontology,
        &fact.relation,
        (&fact.subject.node_type, subject),
        (&fact.object.node_type, object),
    )
}

/// The name of a [`REPLACES_NOTHING`] refusal of the replacement at `facts[<at>]`.
fn nothing_to_replace(at: usize, node_type: &str, property: &str, group: &Group) -> String {
    format!(
        "facts[{at}]: the store holds no active {node_type}.{property} of {:?}",
        group.aliases
    )
}

/// The named things at `sites` grouped, and each key's group: keys of one type that share an
/// alias, transitively, are one thing.
fn grouped(
    sites: &[(String, &ExtractedReference)],
    ontology: &Ontology,
) -> Result<(Vec<Group>, BTreeMap<Key, usize>), ApplyError> {
    // Every distinct key, where it first appears, and the first alias of each of its sites.
    let mut keys: Vec<(Key, TypeId, String, BTreeSet<String>)> = Vec::new();
    let mut index: BTreeMap<Key, usize> = BTreeMap::new();
    for (site, reference) in sites {
        let key = key_of(reference);
        let type_id = reference_type(site, reference, ontology).map_err(Defect::into_error)?;
        let first = reference
            .aliases
            .iter()
            .find(|alias| !alias.is_empty())
            .cloned()
            .unwrap_or_default();
        let at = *index.entry(key.clone()).or_insert_with(|| {
            keys.push((key, type_id, site.clone(), BTreeSet::new()));
            keys.len() - 1
        });
        keys[at].3.insert(first);
    }

    // Keys of one type that share an alias are joined.
    let mut parent: Vec<usize> = (0..keys.len()).collect();
    let mut by_alias: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for (at, ((node_type, aliases), _, _, _)) in keys.iter().enumerate() {
        for alias in aliases {
            match by_alias.get(&(node_type.as_str(), alias.as_str())) {
                Some(&other) => {
                    let (left, right) = (root_of(&mut parent, at), root_of(&mut parent, other));
                    parent[left.max(right)] = left.min(right);
                }
                None => {
                    by_alias.insert((node_type.as_str(), alias.as_str()), at);
                }
            }
        }
    }

    let mut group_of_root: BTreeMap<usize, usize> = BTreeMap::new();
    let mut members: Vec<(BTreeSet<String>, BTreeSet<String>)> = Vec::new();
    let mut groups: Vec<Group> = Vec::new();
    let mut of_key = BTreeMap::new();
    for (at, ((node_type, aliases), type_id, site, firsts)) in keys.iter().enumerate() {
        let root = root_of(&mut parent, at);
        let group = *group_of_root.entry(root).or_insert_with(|| {
            groups.push(Group {
                node_type: node_type.clone(),
                type_id: *type_id,
                aliases: Vec::new(),
                site: site.clone(),
            });
            members.push((BTreeSet::new(), BTreeSet::new()));
            groups.len() - 1
        });
        members[group].0.extend(aliases.iter().cloned());
        members[group].1.extend(firsts.iter().cloned());
        of_key.insert((node_type.clone(), aliases.clone()), group);
    }
    for (group, (aliases, firsts)) in groups.iter_mut().zip(members) {
        let name = firsts.into_iter().next().unwrap_or_default();
        group.aliases = std::iter::once(name.clone())
            .chain(aliases.into_iter().filter(|alias| *alias != name))
            .collect();
    }
    Ok((groups, of_key))
}

/// The root of `at`'s set, halving the path on the way.
fn root_of(parent: &mut [usize], mut at: usize) -> usize {
    while parent[at] != at {
        parent[at] = parent[parent[at]];
        at = parent[at];
    }
    at
}

/// What an assertion claims, as compared for [`ExtractionReport::held`]: its subject node,
/// predicate and object as JSON. Its valid time is compared apart, in [`Made`].
type Claim = (NodeId, Predicate, String);

/// One assertion making a claim.
#[derive(Clone, Debug)]
struct Made {
    /// What became of it.
    reason: HeldReason,
    /// Where its valid time starts; `None` from an unbounded past, as extraction wrote one before
    /// facts had a valid time.
    from: Option<Timestamp>,
    /// Where its valid time ends: a supersession ends it where its replacement starts.
    to: Option<Timestamp>,
    /// The evidence it cites.
    evidence: BTreeSet<EvidenceId>,
    /// Its id.
    id: AssertionId,
}

/// What `assertion` claims, and the assertion as one making it.
fn claim(assertion: &Assertion, reason: HeldReason) -> Option<(Claim, Made)> {
    let Subject::Node(subject) = assertion.subject else {
        return None;
    };
    Some((
        (
            subject,
            assertion.predicate,
            serde_json::to_value(&assertion.object).ok()?.to_string(),
        ),
        Made {
            reason,
            from: assertion.valid_time.from,
            to: assertion.valid_time.to,
            evidence: assertion.evidence.clone(),
            id: assertion.id,
        },
    ))
}

/// Every claim the store holds, and every one this run committed, each with every assertion
/// making it.
type Claims = BTreeMap<Claim, Vec<Made>>;

/// Why `fact`, an assertion not yet made, is not asserted, given the assertions `made` making its
/// claim, and the one that covers it: an active assertion, then one an earlier fact of the
/// document committed, then a retracted one, then a superseded one, that cites every evidence
/// item the fact cites and holds from no later than the fact does — an unbounded start covering
/// every start. Each ends where the fact does, but a superseded one, whose end is where its
/// replacement starts and which the fact reaches. `None` when none covers it.
fn held_because<'a>(made: &'a [Made], fact: &Made) -> Option<&'a Made> {
    let ends = |covering: &Made| match covering.reason {
        HeldReason::Superseded => match (fact.to, covering.to) {
            (None, _) => true,
            (Some(to), Some(end)) => to >= end,
            (Some(_), None) => false,
        },
        _ => covering.to == fact.to,
    };
    let starts = |covering: &Made| match (covering.from, fact.from) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(start), Some(from)) => start <= from,
    };
    [
        HeldReason::Asserted,
        HeldReason::Repeated,
        HeldReason::Retracted,
        HeldReason::Superseded,
    ]
    .into_iter()
    .find_map(|wanted| {
        made.iter().find(|covering| {
            covering.reason == wanted
                && starts(covering)
                && ends(covering)
                && fact.evidence.is_subset(&covering.evidence)
        })
    })
}

/// Apply `document` to the store `transport` answers for, proposing as `operator`, the host
/// operator: [`apply_with`] under the default [`ApplyOptions`], which skips a bad fact and applies
/// the rest.
///
/// # Errors
/// As [`apply_with`].
pub fn apply<T: Transport + ?Sized>(
    transport: &mut T,
    document: &ExtractionDocument,
    operator: AgentId,
) -> Result<ExtractionReport, ApplyError> {
    apply_with(transport, document, operator, &ApplyOptions::default())
}

/// Apply `document` to the store `transport` answers for, proposing as `operator`, the host
/// operator, treating a bad fact as `options` says.
///
/// A fact [`ExtractionDocument::check_facts`] refuses (`fact-evidence-unlisted`), one whose
/// named things the module documentation lists as refused, one naming a property or edge type no
/// declaration gives, and one of [`ApplyOptions::refused`] is skipped: it is listed under
/// [`ExtractionReport::rejected`] as `facts[<index>]` with the refusal `<code>: <name>`, and
/// nothing only it names — a named thing, an evidence item — is resolved, created or added.
/// Under [`ApplyOptions::strict`] it refuses the whole document instead.
///
/// # Errors
/// [`ApplyError::Refused`] or [`ApplyError::Undeclared`] for a document refused before any write;
/// any other [`ApplyError`] when a request gets no answer it can act on before anything
/// committed. Once something has committed, such a request ends the run with
/// [`ExtractionReport::stopped`] instead.
pub fn apply_with<T: Transport + ?Sized>(
    transport: &mut T,
    document: &ExtractionDocument,
    operator: AgentId,
    options: &ApplyOptions,
) -> Result<ExtractionReport, ApplyError> {
    let unlisted = if options.strict {
        document.check()?;
        BTreeMap::new()
    } else {
        document.check_facts()?
    };
    let snapshot = Reader::new(&mut *transport).snapshot(None, None)?;
    let graph = &snapshot.graph.graph;
    let store = Ontology::read(&ontology(transport)?)?;
    // The kernel decides whether the store's profile takes a schema change: under v1 it is
    // rejected (`unsupported-operation`), and reported as the ontology's rejection.
    let change = store.ensure(&document.ontology, ValidationProfile::V2)?;
    let mut active: BTreeMap<(NodeId, PropertyId), Vec<AssertionId>> = BTreeMap::new();
    for assertion in graph.assertions.values() {
        if let (SnapshotSubject::Node(node), SnapshotPredicate::Property(property), true) = (
            assertion.subject,
            assertion.predicate,
            assertion.lifecycle == serde_json::json!("Active"),
        ) {
            active
                .entry((node, property))
                .or_default()
                .push(assertion.id);
        }
    }
    // The nodes `ekr resolve` answers a typed reference with: of its type, holding one of its
    // aliases; and for a superseded node that matches so, the node its record names
    // (`story:supersede-node`), as `ekr_integrate::resolve` answers.
    let holds = |type_id: TypeId, aliases: &[String], property: PropertyId| {
        let matches = |node: &&crate::read::SnapshotNode| {
            node.root_id == graph.root.id
                && node.type_id == type_id
                && node.aliases.iter().any(|alias| aliases.contains(alias))
        };
        let held = graph.nodes.values().filter(matches).map(|node| node.id);
        let kept = graph
            .superseded
            .values()
            .filter(|record| matches(&&record.node))
            .map(|record| record.by);
        held.chain(kept).any(|node| {
            active
                .get(&(node, property))
                .is_some_and(|held| !held.is_empty())
        })
    };
    let planning = if change.is_empty() {
        &store
    } else {
        change.ontology()
    };
    let kept = KeptTypes::read(graph, &store);
    let plan = Plan::of(document, planning, options, unlisted, &holds, &kept)?;
    let held: BTreeSet<EvidenceId> = graph.evidence.keys().copied().collect();
    let mut claims: Claims = BTreeMap::new();
    for assertion in graph.assertions.values() {
        let reason = if assertion.lifecycle == serde_json::json!("Active") {
            HeldReason::Asserted
        } else if assertion.lifecycle.get("Retracted").is_some() {
            HeldReason::Retracted
        } else if assertion.lifecycle.get("Superseded").is_some() {
            HeldReason::Superseded
        } else {
            continue;
        };
        let SnapshotSubject::Node(subject) = assertion.subject else {
            continue;
        };
        let predicate = match assertion.predicate {
            SnapshotPredicate::Property(property) => Predicate::Property(property),
            SnapshotPredicate::Relation(relation) => Predicate::Relation(relation),
            SnapshotPredicate::Other => continue,
        };
        claims
            .entry((subject, predicate, assertion.object.to_string()))
            .or_default()
            .push(Made {
                reason,
                from: assertion.valid_time.from,
                to: assertion.valid_time.to,
                evidence: assertion.evidence.iter().copied().collect(),
                id: assertion.id,
            });
    }

    let mut report = ExtractionReport::default();
    // A skipped fact was refused before anything was tried. The SDK's own refusals name a
    // supersession where one caused them; a refusal the caller's reader made is read here for one.
    report
        .rejected
        .extend(plan.skipped.iter().map(|(at, refusal)| RejectedExtraction {
            item: format!("facts[{at}]"),
            transaction_id: None,
            issues: Vec::new(),
            refusal: Some(match (options.refused.get(at), document.facts.get(*at)) {
                (Some(_), Some(fact)) => named_by_cause(fact, refusal, planning, &kept),
                _ => refusal.to_string(),
            }),
        }));
    let run = Run {
        document,
        plan: &plan,
        root: graph.root.id,
        operator,
        held,
        claims,
        observed: document
            .evidence
            .iter()
            .map(|item| (item.evidence.id, item.evidence.observed_at))
            .collect(),
        edges: graph
            .edges
            .values()
            .map(|edge| (edge.source, edge.type_id, edge.target))
            .collect(),
        active,
    };
    if let Err(error) = run.go(transport, &change, &mut report) {
        if report.committed.is_empty() {
            return Err(error);
        }
        report.stopped = Some(error.to_string());
    }
    Ok(report)
}

/// The refusal of a fact the caller's reader refused (`ApplyOptions::refused`), naming the
/// supersession of each end it resolved through that is its cause
/// (`story:supersede-node-review-fixes`, D6): a property undeclared, or a value its property does
/// not hold, on a subject whose key resolves through a supersession record to a node of another
/// type; and an end of a relation whose key resolves so, where the type of the node it lands on
/// does not connect — the end [`ends_connect`] names for the same fact, so `ekr apply-extraction`,
/// whose reader types each key as the SDK does, and the SDK give one refusal for one fact.
/// A refusal of anything else, and one no supersession reaches, is the reader's, unchanged. The
/// SDK's own refusals name their cause where they are made, and a refusal by validation names
/// none: the checks before it hold each end to the type of the node it lands on.
fn named_by_cause(
    fact: &ExtractedFact,
    refusal: &ExtractionRefusal,
    ontology: &Ontology,
    kept: &KeptTypes,
) -> String {
    let causes: Vec<Kept> = match fact {
        ExtractedFact::Property(fact)
            if [code::PROPERTY_UNDECLARED, VALUE_MISMATCH].contains(&refusal.code.as_str()) =>
        {
            kept.of(&fact.subject.node_type, &fact.subject.aliases)
                .into_iter()
                .collect()
        }
        ExtractedFact::Relation(fact) if refusal.code == code::RELATION_ENDS => {
            [(&fact.subject, true), (&fact.object, false)]
                .into_iter()
                .filter_map(|(reference, source)| {
                    let landed = kept.of(&reference.node_type, &reference.aliases)?;
                    (!ontology.connects_end(&fact.relation, &landed.node_type, source))
                        .then_some(landed)
                })
                .collect()
        }
        _ => Vec::new(),
    };
    noted(refusal.to_string(), causes.iter().map(Some))
}

/// The engine reader's code for a property fact's value its property's type does not hold.
const VALUE_MISMATCH: &str = "extraction-value-mismatch";

/// One application, once its plan is made.
struct Run<'a> {
    document: &'a ExtractionDocument,
    plan: &'a Plan,
    root: ekr_core::GraphRootId,
    operator: AgentId,
    /// The evidence ids the store holds.
    held: BTreeSet<EvidenceId>,
    /// The claims of the store's assertions, and of the facts committed so far.
    claims: Claims,
    /// When each evidence item of the document was observed.
    observed: BTreeMap<EvidenceId, Timestamp>,
    /// Each edge the store holds, and each one this run committed, as its source, type and
    /// target.
    edges: BTreeSet<(NodeId, TypeId, NodeId)>,
    /// The active assertions of each node and property: the store's, then as the facts committed
    /// so far leave them, a replacement in place of those it supersedes.
    active: BTreeMap<(NodeId, PropertyId), Vec<AssertionId>>,
}

/// How a fact's group changes the active assertions of its node and property once it commits.
#[derive(Debug)]
enum Change {
    /// One more is active.
    Add(AssertionId),
    /// These, and no other, are active.
    Set(Vec<AssertionId>),
}

/// What a fact's group changes in what the run records as done, applied only once the group
/// commits.
#[derive(Debug, Default)]
struct Effect {
    /// Its node and property, and how their active assertions change.
    active: Option<((NodeId, PropertyId), Change)>,
    /// The assertions it supersedes, each from when.
    superseded: Vec<(AssertionId, Timestamp)>,
    /// The claim it makes.
    claim: Option<(Claim, Made)>,
    /// The edge it creates.
    edge: Option<(NodeId, TypeId, NodeId)>,
    /// The fact held, whose report row waits for its group to commit: a held replacement whose
    /// supersession is rejected is reported only as rejected.
    held: Option<HeldExtraction>,
}

/// One round of facts, committed together.
#[derive(Debug, Default)]
struct Round {
    /// Each group, its fact's label and its effect, in order.
    groups: Vec<Vec<Operation>>,
    labels: Vec<String>,
    effects: Vec<Effect>,
    /// The claims its groups make, each with the evidence of every fact making it and whether
    /// that fact proposes an edge.
    claimed: BTreeMap<Claim, Vec<(BTreeSet<EvidenceId>, bool)>>,
    /// Each node and property its groups change, and whether one of them replaces.
    touched: BTreeMap<(NodeId, PropertyId), bool>,
}

impl Run<'_> {
    fn go<T: Transport + ?Sized>(
        mut self,
        transport: &mut T,
        change: &SchemaChange,
        report: &mut ExtractionReport,
    ) -> Result<(), ApplyError> {
        let batcher = Batcher::new(self.operator);
        if !change.is_empty() {
            let schema = batcher
                .commit(transport, &[change.operations().to_vec()])
                .map_err(|error| report.stopped_in(error, |_| "ontology".to_owned()))?;
            let refused = !schema.rejected.is_empty() || schema.refused.is_some();
            report.absorb(schema, |_| "ontology".to_owned());
            if refused {
                return Ok(());
            }
        }

        let things = self.resolve(transport, report)?;

        // Every fact about things that resolved, as the assertion it makes.
        let mut pending: Vec<(usize, Assertion)> = Vec::new();
        for (at, (fact, planned)) in self.document.facts.iter().zip(&self.plan.facts).enumerate() {
            // A skipped fact is reported already.
            let Some((subject, said)) = planned else {
                continue;
            };
            let node = |group: usize| match things[group] {
                Thing::Node(node) => Ok(node),
                Thing::Ambiguous => Err(None),
                Thing::Queued(_) | Thing::Missing => {
                    let group = &self.plan.groups[group];
                    Err(Some(format!(
                        "the {} {:?} it names was not created",
                        group.node_type, group.aliases
                    )))
                }
            };
            let assertion = match (fact, said) {
                (ExtractedFact::Property(fact), Said::Property(property)) => {
                    node(*subject).map(|subject| {
                        Assertion::new(
                            self.root,
                            Subject::Node(subject),
                            Predicate::Property(*property),
                            Object::Value(fact.value.clone()),
                            self.operator,
                        )
                    })
                }
                (ExtractedFact::Relation(_), Said::Relation(relation, object)) => node(*subject)
                    .and_then(|subject| {
                        node(*object).map(|object| {
                            Assertion::new(
                                self.root,
                                Subject::Node(subject),
                                Predicate::Relation(*relation),
                                Object::Node(object),
                                self.operator,
                            )
                        })
                    }),
                _ => unreachable!("the plan holds each fact's own kind"),
            };
            match assertion {
                Ok(assertion) => {
                    let cited = fact
                        .evidence()
                        .iter()
                        .fold(assertion, |assertion, id| assertion.citing(*id));
                    // Valid from the earliest observed time of the evidence it cites
                    // (`story:extraction-valid-time`); every id it cites is the document's.
                    let observed = fact
                        .evidence()
                        .iter()
                        .filter_map(|id| self.observed.get(id))
                        .min()
                        .copied();
                    pending.push((
                        at,
                        match observed {
                            Some(from) => cited.with_valid_time(TemporalRange::since(from)),
                            None => cited,
                        },
                    ));
                }
                // An ambiguous thing is reported once, under `ambiguous`.
                Err(None) => {}
                Err(Some(refusal)) => report.rejected.push(RejectedExtraction {
                    item: format!("facts[{at}]"),
                    transaction_id: None,
                    issues: Vec::new(),
                    refusal: Some(refusal),
                }),
            }
        }

        let mut evidence = EvidenceSet::new(self.operator);
        for item in &self.document.evidence {
            let id = evidence.hold(item.clone());
            if self.held.contains(&id) {
                evidence.mark_committed(id);
            }
        }

        // In rounds. What the run records as done changes only when a fact's group commits, so a
        // fact that depends on an earlier fact's change — the same claim, so also the same edge,
        // or the same node and property where either replaces — is planned in a later round,
        // once that fact's result is known: held if it committed, tried on its own if not.
        while !pending.is_empty() {
            let mut round = Round::default();
            let mut deferred = Vec::new();
            for (at, cited) in pending {
                if let Some(cited) = self.plan_fact(at, cited, &mut round, report) {
                    deferred.push((at, cited));
                }
            }
            let Round {
                groups,
                labels,
                effects,
                ..
            } = round;
            let batch = batcher
                .commit_with_evidence(transport, &groups, &mut evidence)
                .map_err(|error| report.stopped_in(error, |group| labels[group].clone()))?;
            let committed: BTreeSet<usize> = batch
                .committed
                .iter()
                .flat_map(|committed| committed.groups.iter().copied())
                .collect();
            let refused = batch
                .refused
                .as_ref()
                .map(|refused| refused.rejection.clone());
            report.absorb(batch, |group| labels[group].clone());
            for (group, effect) in effects.into_iter().enumerate() {
                if committed.contains(&group) {
                    report.held.extend(effect.held.clone());
                    self.record(effect);
                }
            }
            // A refusal of a transaction as a whole stops the run; a fact not yet tried is
            // refused with it, as a later batch's groups are.
            if let Some(refusal) = refused {
                for (at, _) in deferred {
                    report
                        .rejected
                        .push(rejection(format!("facts[{at}]"), refusal.clone()));
                }
                return Ok(());
            }
            pending = deferred;
        }
        Ok(())
    }

    /// Plans the fact at `facts[<at>]`, whose assertion is `cited`, into `round`: held, refused,
    /// or a group. Its assertion back when it depends on a change an earlier fact of the round
    /// makes, to be planned in the next.
    fn plan_fact(
        &self,
        at: usize,
        cited: Assertion,
        round: &mut Round,
        report: &mut ExtractionReport,
    ) -> Option<Assertion> {
        let fact = &self.document.facts[at];
        let replacing = match fact {
            ExtractedFact::Property(fact) if fact.replaces => Some(fact),
            _ => None,
        };
        let (key, made) = claim(&cited, HeldReason::Repeated).expect("a fact's subject is a node");
        let slot = match (cited.subject, cited.predicate) {
            (Subject::Node(node), Predicate::Property(property)) => Some((node, property)),
            _ => None,
        };
        // A fact waits for an earlier fact of the round only when that fact's result can change
        // how it is planned: it repeats the claim citing no evidence the earlier fact does not
        // (so it would be held by it), or it is a relation whose edge the earlier fact proposes,
        // or it and the earlier fact change one node and property where either replaces. Any
        // other corroboration of a claim is asserted whatever became of the earlier fact, and
        // commits in the same round.
        let depends = round.claimed.get(&key).is_some_and(|earlier| {
            earlier
                .iter()
                .any(|(evidence, edge)| *edge || made.evidence.is_subset(evidence))
        }) || slot.is_some_and(|slot| {
            round
                .touched
                .get(&slot)
                .is_some_and(|&replaced| replaced || replacing.is_some())
        });
        if depends {
            return Some(cited);
        }

        let item = format!("facts[{at}]");
        let active: Vec<AssertionId> = slot
            .and_then(|slot| self.active.get(&slot))
            .cloned()
            .unwrap_or_default();
        if let Some(covering) = self
            .claims
            .get(&key)
            .and_then(|made_by| held_because(made_by, &made))
        {
            // A replacement whose claim is already made still supersedes every other active value
            // of its node and property, by the active assertion that makes it
            // (`story:extraction-supersession`). One made from an unbounded past cannot replace
            // from a start: the fact is then asserted anew below, superseding every active value.
            let others: Vec<AssertionId> = active
                .iter()
                .copied()
                .filter(|id| *id != covering.id)
                .collect();
            let supersedes = replacing.is_some()
                && !others.is_empty()
                && matches!(covering.reason, HeldReason::Asserted | HeldReason::Repeated);
            match (supersedes, covering.from, slot) {
                (false, _, _) => {
                    report.held.push(HeldExtraction {
                        item,
                        reason: covering.reason,
                    });
                    return None;
                }
                (true, Some(from), Some(slot)) if active.contains(&covering.id) => {
                    // Held once the supersession commits; if it is rejected, the fact is
                    // reported only as rejected.
                    let held = HeldExtraction {
                        item: item.clone(),
                        reason: covering.reason,
                    };
                    let superseded: Vec<(AssertionId, Timestamp)> =
                        others.iter().map(|old| (*old, from)).collect();
                    round.groups.push(
                        superseded
                            .iter()
                            .map(|(old, from)| {
                                Operation::SupersedeAssertion(Supersession::new(
                                    *old,
                                    covering.id,
                                    *from,
                                ))
                            })
                            .collect(),
                    );
                    round.labels.push(item);
                    round.effects.push(Effect {
                        active: Some((slot, Change::Set(vec![covering.id]))),
                        superseded,
                        held: Some(held),
                        ..Effect::default()
                    });
                    round
                        .claimed
                        .entry(key)
                        .or_default()
                        .push((made.evidence, false));
                    round.touched.insert(slot, true);
                    return None;
                }
                _ => {}
            }
        }

        // A replacement supersedes every active assertion of its node and property from its own
        // valid time on, in its own group; with none left to replace — an earlier fact that
        // would have set one was rejected — it is refused alone.
        let mut effect = Effect::default();
        if let Some(slot) = slot {
            effect.active = Some((
                slot,
                if replacing.is_some() {
                    Change::Set(vec![cited.id])
                } else {
                    Change::Add(cited.id)
                },
            ));
        }
        if let (Some(replacing), Some(slot)) = (replacing, slot) {
            let refusal = match cited.valid_time.from {
                None => Some(format!("{}: {item}", code::WITHOUT_EVIDENCE)),
                Some(_) if active.is_empty() => {
                    let group = self.plan.facts[at].as_ref().map(|(subject, _)| *subject);
                    Some(format!(
                        "{REPLACES_NOTHING}: {}",
                        nothing_to_replace(
                            at,
                            &replacing.subject.node_type,
                            &replacing.property,
                            &self.plan.groups[group.expect("a planned fact has a subject")],
                        )
                    ))
                }
                Some(from) => {
                    effect.superseded = active.iter().map(|old| (*old, from)).collect();
                    None
                }
            };
            if let Some(refusal) = refusal {
                report.rejected.push(RejectedExtraction {
                    item,
                    transaction_id: None,
                    issues: Vec::new(),
                    refusal: Some(refusal),
                });
                return None;
            }
            round.touched.insert(slot, true);
        } else if let Some(slot) = slot {
            round.touched.entry(slot).or_insert(false);
        }

        // A relation is also an edge, for the reads that walk the graph
        // (`story:extracted-relations-visible-to-graph-reads`), in the assertion's group: an edge
        // the kernel refuses refuses the fact. One edge of a type joins two nodes once, whichever
        // fact or earlier writer committed it.
        effect.edge = match (cited.subject, cited.predicate, &cited.object) {
            (Subject::Node(source), Predicate::Relation(relation), Object::Node(target))
                if !self.edges.contains(&(source, relation, *target)) =>
            {
                Some((source, relation, *target))
            }
            _ => None,
        };
        let id = cited.id;
        let mut group = vec![cited.into()];
        group.extend(effect.edge.map(|(source, relation, target)| {
            Operation::CreateEdge(EdgeDraft::new(self.root, relation, source, target))
        }));
        group.extend(
            effect.superseded.iter().map(|(old, from)| {
                Operation::SupersedeAssertion(Supersession::new(*old, id, *from))
            }),
        );
        round.groups.push(group);
        round.labels.push(item);
        round
            .claimed
            .entry(key.clone())
            .or_default()
            .push((made.evidence.clone(), effect.edge.is_some()));
        effect.claim = Some((key, made));
        round.effects.push(effect);
        None
    }

    /// Records what a committed group changed.
    fn record(&mut self, effect: Effect) {
        match effect.active {
            Some((slot, Change::Add(id))) => self.active.entry(slot).or_default().push(id),
            Some((slot, Change::Set(ids))) => {
                self.active.insert(slot, ids);
            }
            None => {}
        }
        for (old, from) in &effect.superseded {
            for made in self.claims.values_mut().flatten() {
                if made.id == *old {
                    made.reason = HeldReason::Superseded;
                    made.to = Some(*from);
                }
            }
        }
        if let Some((key, made)) = effect.claim {
            self.claims.entry(key).or_default().push(made);
        }
        if let Some(edge) = effect.edge {
            self.edges.insert(edge);
        }
    }

    /// Every group resolved, and the ones the store holds none of created.
    fn resolve<T: Transport + ?Sized>(
        &self,
        transport: &mut T,
        report: &mut ExtractionReport,
    ) -> Result<Vec<Thing>, ApplyError> {
        let mut resolver = Resolver::new(self.root, self.operator);
        let mut things = Vec::new();
        let mut created: BTreeMap<NodeId, usize> = BTreeMap::new();
        for (at, group) in self.plan.groups.iter().enumerate() {
            let typed = TypedReference::new(group.type_id, group.aliases.iter().cloned());
            let resolved = resolver.resolve(transport, &typed).map_err(|error| {
                report.absorb(resolver.take_pending().report, |_| "entities".to_owned());
                ApplyError::from(error)
            })?;
            things.push(match resolved {
                Resolution::Resolved(node) => Thing::Node(node),
                Resolution::Queued(node) => {
                    created.insert(node, at);
                    Thing::Queued(node)
                }
                Resolution::Ambiguous(candidates) => {
                    report.ambiguous.push(ambiguous(group, candidates));
                    Thing::Ambiguous
                }
            });
        }

        let flushed = resolver.flush(transport).map_err(|error| {
            report.absorb(resolver.take_pending().report, |_| "entities".to_owned());
            ApplyError::from(error)
        })?;
        let mut missing: BTreeSet<NodeId> = flushed
            .report
            .rejected
            .iter()
            .filter_map(|rejected| match &rejected.operation {
                Operation::CreateNode(node) => Some(node.id),
                _ => None,
            })
            .collect();
        if flushed.report.refused.is_some() {
            missing.extend(created.keys().copied());
        }
        let nodes: BTreeMap<usize, NodeId> = flushed
            .report
            .rejected
            .iter()
            .filter_map(|rejected| match &rejected.operation {
                Operation::CreateNode(node) => Some((rejected.group, node.id)),
                _ => None,
            })
            .collect();
        let label = |group: usize| {
            nodes
                .get(&group)
                .and_then(|node| created.get(node))
                .map_or_else(
                    || "entities".to_owned(),
                    |&at| self.plan.groups[at].site.clone(),
                )
        };
        report.absorb(flushed.report, label);
        for thing in &mut things {
            if let Thing::Queued(node) = *thing {
                *thing = match flushed.replaced.get(&node) {
                    Some(Resolution::Resolved(other)) => Thing::Node(*other),
                    Some(Resolution::Ambiguous(_)) => Thing::Ambiguous,
                    Some(Resolution::Queued(_)) | None if missing.contains(&node) => Thing::Missing,
                    Some(Resolution::Queued(_)) | None => Thing::Node(node),
                };
            }
        }
        for (node, &at) in &created {
            if let Some(Resolution::Ambiguous(candidates)) = flushed.replaced.get(node) {
                report
                    .ambiguous
                    .push(ambiguous(&self.plan.groups[at], candidates.clone()));
            }
        }
        Ok(things)
    }
}

/// The report's row for a group the store answers with `candidates`.
fn ambiguous(group: &Group, candidates: Vec<NodeId>) -> AmbiguousExtraction {
    AmbiguousExtraction {
        reference: ExtractedReference::new(group.node_type.clone(), group.aliases.iter().cloned()),
        candidates,
    }
}

impl ExtractionReport {
    /// Adds what a batch committed and rejected before it stopped, and passes on why it stopped.
    fn stopped_in(&mut self, error: Box<BatchError>, item: impl Fn(usize) -> String) -> ApplyError {
        self.absorb(error.report.clone(), item);
        ApplyError::Batch(error)
    }

    /// Adds a batcher's report: its commits, and one rejection per refused group, named by
    /// `item`.
    fn absorb(&mut self, batch: BatchReport, item: impl Fn(usize) -> String) {
        self.committed.extend(
            batch
                .committed
                .into_iter()
                .map(|committed| CommittedExtraction {
                    transaction_id: committed.transaction,
                    revision: committed.revision,
                }),
        );
        for rejected in batch
            .rejected
            .into_iter()
            .filter(|rejected| rejected.index == 0)
        {
            self.rejected
                .push(rejection(item(rejected.group), rejected.rejection));
        }
        if let Some(refused) = batch.refused {
            for group in refused.groups {
                self.rejected
                    .push(rejection(item(group), refused.rejection.clone()));
            }
        }
    }
}

/// A group's rejection, as the report names it.
fn rejection(item: String, rejection: Rejection) -> RejectedExtraction {
    let (transaction_id, issues, refusal) = match rejection {
        Rejection::Rejected {
            transaction,
            issues,
        } => (
            Some(transaction),
            issues
                .into_iter()
                .map(|issue| ExtractionIssue {
                    validator: issue.validator,
                    code: issue.code,
                    message: issue.message,
                })
                .collect(),
            None,
        ),
        Rejection::Refused(refusal) => (
            None,
            Vec::new(),
            Some(format!("{}: {}", refusal.code, refusal.reason)),
        ),
        Rejection::Document(reason) => (None, Vec::new(), Some(reason)),
    };
    RejectedExtraction {
        item,
        transaction_id,
        issues,
        refusal,
    }
}

/// `reference`'s key: its type's name and its distinct non-empty aliases, sorted.
fn key_of(reference: &ExtractedReference) -> Key {
    let aliases: BTreeSet<&String> = reference
        .aliases
        .iter()
        .filter(|alias| !alias.is_empty())
        .collect();
    (
        reference.node_type.clone(),
        aliases.into_iter().cloned().collect(),
    )
}

/// What `ekr ontology` printed, as JSON text.
fn ontology<T: Transport + ?Sized>(transport: &mut T) -> Result<String, ApplyError> {
    let request = Request::new(["ontology"]);
    match transport
        .request(&request)
        .map_err(CallError::from)?
        .answer()
    {
        Answer::Outcome(Outcome::Other(document)) => {
            Ok(serde_json::to_string(&document).expect("a JSON value writes"))
        }
        answer => Err(CallError::Unanswered {
            verb: "ontology".to_owned(),
            answer,
        }
        .into()),
    }
}
