//! Applying an extraction document to a store (`story:extraction-verb-shares-the-sdk-path`).
//!
//! [`apply`] is the one routine that turns an [`ExtractionDocument`] into committed
//! transactions. `ekr apply-extraction` runs it in process, over a transport that answers each
//! request through the session's own dispatch; a consumer runs it over a child `ekr session`
//! ([`crate::session::ProcessSession`]). One algorithm, two transports: both write the same store.
//!
//! First it reads `ekr snapshot`, for the graph root new nodes belong to, the evidence and the
//! assertions the store holds, and `ekr ontology`, for the store's types by name. Then, before it
//! writes anything, it refuses — by the engine reader's code, as [`ApplyError::Refused`] — a
//! document [`ExtractionDocument::check`] refuses (two evidence items under one id, a payload that
//! does not hash to its entry); a named thing with no alias but the empty string
//! (`reference-without-identity`); and a named thing whose node type is abstract or has a subtype
//! once the document's ontology is applied (`reference-type-has-subtypes`), which `ekr resolve`
//! would refuse. Every type, property and relation the document names is looked up then too, so
//! a name no declaration gives is [`ApplyError::Undeclared`] before anything is written.
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
//!    value, `!Relation` with the object node — committed through a [`Batcher`] with the
//!    document's evidence: each item a fact cites is added with the first fact citing it, unless
//!    the store already holds its id. An item no fact cites is not added. A fact the store already
//!    asserts — an active assertion with the same subject, predicate, object, valid time and
//!    evidence — or that an earlier fact of the document says too is [held](ExtractionReport::held)
//!    and not asserted again. So a document applied a second time adds no node, evidence entry or
//!    assertion.
//!
//! The only requests that write are `propose`, `validate` and `commit`; the rest are `snapshot`,
//! `ontology`, `head` and `resolve`. Once something has committed, the routine does not end in an
//! error: a request that then gets no answer it can act on stops it, and the report names why
//! ([`ExtractionReport::stopped`]) beside what committed until then.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AgentId, EvidenceId, NodeId, Timestamp, TransactionId, TypeId};
use serde::Serialize;

use crate::batch::{BatchError, BatchReport, Batcher, CallError, Rejection};
use crate::document::{
    extraction_codes as code, Assertion, ExtractedFact, ExtractedReference, ExtractionDocument,
    ExtractionRefusal, Object, Ontology, OntologyError, Operation, Predicate, PropertyId,
    SchemaChange, Subject, TypedReference, ValidationProfile,
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
    /// Every fact not asserted because the store already asserts it, or an earlier fact of the
    /// document says the same, by its item: `facts[<index>]`.
    pub held: Vec<String>,
    /// Why applying stopped before the end of the document, after something had committed;
    /// `None` when it went to its end.
    pub stopped: Option<String>,
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
    /// `ekr propose` as `<code>: <reason>`, a document limit, or a named thing it rests on that
    /// was not created.
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

/// Everything [`apply`] looks up before it writes.
#[derive(Debug)]
struct Plan {
    groups: Vec<Group>,
    /// Each fact's subject group and what it says, in document order.
    facts: Vec<(usize, Said)>,
}

impl Plan {
    /// The groups and lookups of `document` against `ontology`, the store's once the document's
    /// ontology is applied. Refuses what the module documentation lists, before any write.
    fn of(document: &ExtractionDocument, ontology: &Ontology) -> Result<Self, ApplyError> {
        let mut sites: Vec<(String, &ExtractedReference)> = document
            .entities
            .iter()
            .enumerate()
            .map(|(at, entity)| (format!("entities[{at}]"), entity))
            .collect();
        for (at, fact) in document.facts.iter().enumerate() {
            let subject = match fact {
                ExtractedFact::Property(fact) => &fact.subject,
                ExtractedFact::Relation(fact) => &fact.subject,
            };
            sites.push((format!("facts[{at}].subject"), subject));
            if let ExtractedFact::Relation(fact) = fact {
                sites.push((format!("facts[{at}].object"), &fact.object));
            }
        }

        // Every distinct key, where it first appears, and the first alias of each of its sites.
        let mut keys: Vec<(Key, TypeId, String, BTreeSet<String>)> = Vec::new();
        let mut index: BTreeMap<Key, usize> = BTreeMap::new();
        for (site, reference) in &sites {
            let key = key_of(reference);
            if key.1.is_empty() {
                return Err(ExtractionRefusal::new(code::WITHOUT_IDENTITY, site.clone()).into());
            }
            let Some(type_id) = ontology.node_type(&key.0) else {
                return Err(undeclared("node type", &key.0));
            };
            if ontology.has_subtypes(&key.0) {
                return Err(ExtractionRefusal::new(
                    code::HAS_SUBTYPES,
                    format!("{site}: {}", key.0),
                )
                .into());
            }
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

        let mut facts = Vec::new();
        for fact in &document.facts {
            match fact {
                ExtractedFact::Property(fact) => {
                    let property = ontology
                        .property(&fact.subject.node_type, &fact.property)
                        .ok_or_else(|| {
                            undeclared(
                                "property",
                                &format!("{}.{}", fact.subject.node_type, fact.property),
                            )
                        })?;
                    facts.push((of_key[&key_of(&fact.subject)], Said::Property(property)));
                }
                ExtractedFact::Relation(fact) => {
                    let relation = ontology
                        .edge_type(&fact.relation)
                        .ok_or_else(|| undeclared("edge type", &fact.relation))?;
                    facts.push((
                        of_key[&key_of(&fact.subject)],
                        Said::Relation(relation, of_key[&key_of(&fact.object)]),
                    ));
                }
            }
        }
        Ok(Self { groups, facts })
    }
}

/// The root of `at`'s set, halving the path on the way.
fn root_of(parent: &mut [usize], mut at: usize) -> usize {
    while parent[at] != at {
        parent[at] = parent[parent[at]];
        at = parent[at];
    }
    at
}

/// An active assertion as compared for [`ExtractionReport::held`]: its subject node, predicate,
/// object as JSON, valid time and evidence.
type Claim = (
    NodeId,
    Predicate,
    String,
    Option<Timestamp>,
    Option<Timestamp>,
    BTreeSet<EvidenceId>,
);

/// What `assertion` claims.
fn claim(assertion: &Assertion) -> Option<Claim> {
    let Subject::Node(subject) = assertion.subject else {
        return None;
    };
    Some((
        subject,
        assertion.predicate,
        serde_json::to_value(&assertion.object).ok()?.to_string(),
        assertion.valid_time.from,
        assertion.valid_time.to,
        assertion.evidence.clone(),
    ))
}

/// Apply `document` to the store `transport` answers for, proposing as `operator`, the host
/// operator.
///
/// # Errors
/// [`ApplyError::Refused`] or [`ApplyError::Undeclared`] for a document refused before any write;
/// any other [`ApplyError`] when a request gets no answer it can act on before anything
/// committed. Once something has committed, such a request ends the run with
/// [`ExtractionReport::stopped`] instead.
pub fn apply<T: Transport + ?Sized>(
    transport: &mut T,
    document: &ExtractionDocument,
    operator: AgentId,
) -> Result<ExtractionReport, ApplyError> {
    document.check()?;
    let snapshot = Reader::new(&mut *transport).snapshot(None, None)?;
    let graph = &snapshot.graph.graph;
    let store = Ontology::read(&ontology(transport)?)?;
    // The kernel decides whether the store's profile takes a schema change: under v1 it is
    // rejected (`unsupported-operation`), and reported as the ontology's rejection.
    let change = store.ensure(&document.ontology, ValidationProfile::V2)?;
    let plan = Plan::of(
        document,
        if change.is_empty() {
            &store
        } else {
            change.ontology()
        },
    )?;
    let held: BTreeSet<EvidenceId> = graph.evidence.keys().copied().collect();
    let asserted: BTreeSet<Claim> = graph
        .assertions
        .values()
        .filter(|assertion| assertion.lifecycle == serde_json::json!("Active"))
        .filter_map(|assertion| {
            let SnapshotSubject::Node(subject) = assertion.subject else {
                return None;
            };
            let predicate = match assertion.predicate {
                SnapshotPredicate::Property(property) => Predicate::Property(property),
                SnapshotPredicate::Relation(relation) => Predicate::Relation(relation),
                SnapshotPredicate::Other => return None,
            };
            Some((
                subject,
                predicate,
                assertion.object.to_string(),
                assertion.valid_time.from,
                assertion.valid_time.to,
                assertion.evidence.iter().copied().collect(),
            ))
        })
        .collect();

    let mut report = ExtractionReport::default();
    let run = Run {
        document,
        plan: &plan,
        root: graph.root.id,
        operator,
        held,
        asserted,
    };
    if let Err(error) = run.go(transport, &change, &mut report) {
        if report.committed.is_empty() {
            return Err(error);
        }
        report.stopped = Some(error.to_string());
    }
    Ok(report)
}

/// One application, once its plan is made.
struct Run<'a> {
    document: &'a ExtractionDocument,
    plan: &'a Plan,
    root: ekr_core::GraphRootId,
    operator: AgentId,
    /// The evidence ids the store holds.
    held: BTreeSet<EvidenceId>,
    /// The claims of the store's active assertions.
    asserted: BTreeSet<Claim>,
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
            let schema = batcher.commit(transport, &[change.operations().to_vec()])?;
            let refused = !schema.rejected.is_empty() || schema.refused.is_some();
            report.absorb(schema, |_| "ontology".to_owned());
            if refused {
                return Ok(());
            }
        }

        let things = self.resolve(transport, report)?;

        let mut groups: Vec<Vec<Operation>> = Vec::new();
        let mut labels: Vec<String> = Vec::new();
        for (at, (fact, (subject, said))) in
            self.document.facts.iter().zip(&self.plan.facts).enumerate()
        {
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
                    if claim(&cited).is_some_and(|claim| !self.asserted.insert(claim)) {
                        report.held.push(format!("facts[{at}]"));
                        continue;
                    }
                    groups.push(vec![cited.into()]);
                    labels.push(format!("facts[{at}]"));
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
        let facts = batcher.commit_with_evidence(transport, &groups, &mut evidence)?;
        report.absorb(facts, |group| labels[group].clone());
        Ok(())
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
            things.push(match resolver.resolve(transport, &typed)? {
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

        let flushed = resolver.flush(transport)?;
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

fn undeclared(kind: &'static str, name: &str) -> ApplyError {
    ApplyError::Undeclared {
        kind,
        name: name.to_owned(),
    }
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
