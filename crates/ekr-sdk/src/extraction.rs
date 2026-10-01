//! Applying an extraction document to a store (`story:extraction-verb-shares-the-sdk-path`).
//!
//! [`apply`] is the one routine that turns an [`ExtractionDocument`] into committed
//! transactions. `ekr apply-extraction` runs it in process, over a transport that answers each
//! request through the session's own dispatch; a consumer runs it over a child `ekr session`
//! ([`crate::session::ProcessSession`]). One algorithm, two transports: both write the same store.
//!
//! In order, over `transport`:
//!
//! 1. `ekr snapshot` for the graph root new nodes belong to and the evidence the store holds, and
//!    `ekr ontology` for the store's types by name;
//! 2. the document's ontology, as [`Ontology::ensure`] finds it missing, committed as one schema
//!    change. If it is rejected, nothing more is applied: what the document names by it does not
//!    exist;
//! 3. every named thing — each entity, then each fact's subject and object, in document order —
//!    resolved through one [`Resolver`]: a thing the store holds is used, a thing it does not is
//!    created as a node of its type named by its first alias, and a thing the store answers with
//!    several nodes is [ambiguous](ExtractionReport::ambiguous): nothing is chosen;
//! 4. every fact about things that resolved, as one `!AddAssertion` each — `!Property` with the
//!    value, `!Relation` with the object node — committed through a [`Batcher`] with the
//!    document's evidence: each item a fact cites is added with the first fact citing it, unless
//!    the store already holds its id. An item no fact cites is not added.
//!
//! The only requests that write are `propose`, `validate` and `commit`; the rest are `snapshot`,
//! `ontology`, `head` and `resolve`. Nothing here checks the document against the store first:
//! `ekr apply-extraction` runs the engine's reader before it calls this, and over a child session
//! the kernel's validators refuse what the store cannot take, each as a
//! [rejected](ExtractionReport::rejected) part of the document.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AgentId, EvidenceId, NodeId, TransactionId};
use serde::Serialize;

use crate::batch::{BatchError, BatchReport, Batcher, CallError, Rejection};
use crate::document::{
    Assertion, ExtractedFact, ExtractedReference, ExtractionDocument, Object, Ontology,
    OntologyError, Operation, Predicate, Subject, TypedReference, ValidationProfile,
};
use crate::evidence::EvidenceSet;
use crate::read::{ReadError, Reader};
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
    /// The thing, as the document names it.
    pub reference: ExtractedReference,
    /// Every node it matched, in id order.
    pub candidates: Vec<NodeId>,
}

/// Why [`apply`] stopped before its end. What it committed until then stays committed.
#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
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

/// What a named thing came to.
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

/// Apply `document` to the store `transport` answers for, proposing as `operator`, the host
/// operator.
///
/// # Errors
/// [`ApplyError`] when a request gets no answer it can act on, or the document names a type or
/// property no declaration gives.
pub fn apply<T: Transport + ?Sized>(
    transport: &mut T,
    document: &ExtractionDocument,
    operator: AgentId,
) -> Result<ExtractionReport, ApplyError> {
    let snapshot = Reader::new(&mut *transport).snapshot(None, None)?;
    let root = snapshot.graph.graph.root.id;
    let held: BTreeSet<EvidenceId> = snapshot.graph.graph.evidence.keys().copied().collect();
    let store = Ontology::read(&ontology(transport)?)?;

    let mut report = ExtractionReport::default();
    let batcher = Batcher::new(operator);
    // The kernel decides whether the store's profile takes a schema change: under v1 it is
    // rejected (`unsupported-operation`), and reported as the ontology's rejection.
    let change = store.ensure(&document.ontology, ValidationProfile::V2)?;
    let ontology = if change.is_empty() {
        store
    } else {
        let schema = batcher.commit(transport, &[change.operations().to_vec()])?;
        let refused = !schema.rejected.is_empty() || schema.refused.is_some();
        report.absorb(schema, |_| "ontology".to_owned());
        if refused {
            return Ok(report);
        }
        change.ontology().clone()
    };

    // Every named thing, where it first appears.
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

    let mut resolver = Resolver::new(root, operator);
    let mut things: BTreeMap<Key, Thing> = BTreeMap::new();
    let mut created: BTreeMap<NodeId, (Key, String)> = BTreeMap::new();
    for (site, reference) in &sites {
        let key = key_of(reference);
        if things.contains_key(&key) {
            continue;
        }
        let type_id = ontology
            .node_type(&reference.node_type)
            .ok_or_else(|| undeclared("node type", &reference.node_type))?;
        let typed = TypedReference::new(type_id, reference.aliases.iter().cloned());
        let thing = match resolver.resolve(transport, &typed)? {
            Resolution::Resolved(node) => Thing::Node(node),
            Resolution::Queued(node) => {
                created
                    .entry(node)
                    .or_insert_with(|| (key.clone(), site.clone()));
                Thing::Queued(node)
            }
            Resolution::Ambiguous(candidates) => {
                report.ambiguous.push(AmbiguousExtraction {
                    reference: (*reference).clone(),
                    candidates,
                });
                Thing::Ambiguous
            }
        };
        things.insert(key, thing);
    }

    let flushed = resolver.flush(transport)?;
    let mut missing: BTreeSet<NodeId> = BTreeSet::new();
    for rejected in &flushed.report.rejected {
        if let Operation::CreateNode(node) = &rejected.operation {
            missing.insert(node.id);
        }
    }
    if flushed.report.refused.is_some() {
        missing.extend(created.keys().copied());
    }
    let label = |node: Option<NodeId>| {
        node.and_then(|node| created.get(&node))
            .map_or_else(|| "entities".to_owned(), |(_, site)| site.clone())
    };
    let nodes: BTreeMap<usize, NodeId> = flushed
        .report
        .rejected
        .iter()
        .filter_map(|rejected| match &rejected.operation {
            Operation::CreateNode(node) => Some((rejected.group, node.id)),
            _ => None,
        })
        .collect();
    report.absorb(flushed.report, |group| label(nodes.get(&group).copied()));
    for thing in things.values_mut() {
        if let Thing::Queued(node) = *thing {
            *thing = match flushed.replaced.get(&node) {
                Some(Resolution::Resolved(other)) => Thing::Node(*other),
                Some(Resolution::Ambiguous(_)) => Thing::Ambiguous,
                Some(Resolution::Queued(_)) | None if missing.contains(&node) => Thing::Missing,
                Some(Resolution::Queued(_)) | None => Thing::Node(node),
            };
        }
    }
    for (node, (key, _)) in &created {
        if let Some(Resolution::Ambiguous(candidates)) = flushed.replaced.get(node) {
            report.ambiguous.push(AmbiguousExtraction {
                reference: ExtractedReference::new(key.0.clone(), key.1.iter().cloned()),
                candidates: candidates.clone(),
            });
        }
    }

    let mut groups: Vec<Vec<Operation>> = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    for (at, fact) in document.facts.iter().enumerate() {
        let node = |reference: &ExtractedReference| match things.get(&key_of(reference)) {
            Some(Thing::Node(node)) => Ok(*node),
            Some(Thing::Ambiguous) => Err(None),
            _ => Err(Some(format!(
                "the {} {:?} it names was not created",
                reference.node_type, reference.aliases
            ))),
        };
        let assertion = match fact {
            ExtractedFact::Property(fact) => {
                let property = ontology
                    .property(&fact.subject.node_type, &fact.property)
                    .ok_or_else(|| {
                        undeclared(
                            "property",
                            &format!("{}.{}", fact.subject.node_type, fact.property),
                        )
                    })?;
                node(&fact.subject).map(|subject| {
                    Assertion::new(
                        root,
                        Subject::Node(subject),
                        Predicate::Property(property),
                        Object::Value(fact.value.clone()),
                        operator,
                    )
                })
            }
            ExtractedFact::Relation(fact) => {
                let relation = ontology
                    .edge_type(&fact.relation)
                    .ok_or_else(|| undeclared("edge type", &fact.relation))?;
                node(&fact.subject).and_then(|subject| {
                    node(&fact.object).map(|object| {
                        Assertion::new(
                            root,
                            Subject::Node(subject),
                            Predicate::Relation(relation),
                            Object::Node(object),
                            operator,
                        )
                    })
                })
            }
        };
        match assertion {
            Ok(assertion) => {
                let cited = fact
                    .evidence()
                    .iter()
                    .fold(assertion, |assertion, id| assertion.citing(*id));
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

    let mut evidence = EvidenceSet::new(operator);
    for item in &document.evidence {
        let id = evidence.hold(item.clone());
        if held.contains(&id) {
            evidence.mark_committed(id);
        }
    }
    let facts = batcher.commit_with_evidence(transport, &groups, &mut evidence)?;
    report.absorb(facts, |group| labels[group].clone());
    Ok(report)
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
