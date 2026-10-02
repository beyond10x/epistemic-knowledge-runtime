//! Pure unordered application, reachable only with the kernel's private validated capability.
use crate::replay::Revision;
use crate::{GraphOperation, ValidatedTransaction};
use ekr_core::{AgentId, ContentHash, Timestamp};
use ekr_graph::{
    AssertionLifecycle, Assessment, AttachedEvidence, CanonicalGraph, CanonicalRef, Edge, Node,
    Root, TransactionTime,
};
use ekr_store::{evidence_root, knowledge_root, StoreError};
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::sync::{Arc, Weak};

/// The root the last [`apply`] on this thread computed, with every input it is a function of.
///
/// A commit applies its transaction to the head when the kernel decides it, and the store then
/// admits the staged candidate by replaying it through the same kernel, which applies the same
/// transaction to the same head graph again. The root is a pure function of these inputs, so that
/// second application takes the root the kernel already computed rather than hashing the whole
/// graph a second time (`story:commit-hashes-the-graph-once`). The root is still the kernel's.
///
/// The graph is the same pure function of the same inputs, so the decision also leaves the graph
/// it applied here ([`decide`]), and that second application takes it instead of cloning the
/// head graph and applying the transaction again (`task:commit-applies-once`). The graph is
/// still the kernel's: this module computed it, from the inputs the key names.
///
/// The prior graph is identified by its allocation. The weak reference keeps that allocation
/// from being reused while it is held, so an equal address is the same graph; it does not keep
/// the graph's contents alive.
struct Applied {
    prior: Weak<CanonicalGraph>,
    prior_root: Root,
    prior_committed_at: Timestamp,
    validators: BTreeSet<AgentId>,
    at: Timestamp,
    /// The result; its `transaction` is the content address of the transaction applied.
    root: Root,
    /// The graph the decision applied, until the application that admits it takes it.
    graph: Option<CanonicalGraph>,
}
impl Applied {
    fn holds(
        &self,
        prior: &Revision,
        graph: &Arc<CanonicalGraph>,
        transaction: ContentHash,
        validators: &BTreeSet<AgentId>,
        at: Timestamp,
    ) -> bool {
        std::ptr::eq(self.prior.as_ptr(), Arc::as_ptr(graph))
            && self.prior_root == prior.root
            && self.prior_committed_at == prior.committed_at
            && self.root.transaction == transaction
            && self.validators == *validators
            && self.at == at
    }
}
thread_local! {
    static APPLIED: RefCell<Option<Applied>> = const { RefCell::new(None) };
    static GRAPHS_APPLIED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// How many times the calling thread has cloned a prior revision's graph and applied a
/// transaction to it.
///
/// Test instrumentation, as `ekr_store::knowledge_roots_hashed` is: it lets a test show that a
/// commit clones and applies the head graph once. Counted per thread because every store and
/// kernel call runs on its caller's thread, and tests in one binary run on several.
#[doc(hidden)]
#[must_use]
pub fn graphs_applied() -> u64 {
    GRAPHS_APPLIED.with(std::cell::Cell::get)
}

/// Whether the calling thread holds a graph a commit decision applied and no application has
/// taken yet.
///
/// Test instrumentation, as [`graphs_applied`] is: it lets a test show that a graph a commit
/// decided and did not take is released when the command ends.
#[doc(hidden)]
#[must_use]
pub fn decided_graph_held() -> bool {
    APPLIED.with(|applied| {
        applied
            .borrow()
            .as_ref()
            .is_some_and(|applied| applied.graph.is_some())
    })
}

/// Releases the graph a decision left for the application that admits it, when that application
/// did not take it: dropped at the end of a commit command, so a graph is held only while its
/// publication is in flight. The remembered root stays.
pub(crate) struct ReleaseDecided;
impl Drop for ReleaseDecided {
    fn drop(&mut self) {
        let _ = APPLIED.try_with(|applied| {
            if let Ok(mut applied) = applied.try_borrow_mut() {
                if let Some(applied) = applied.as_mut() {
                    applied.graph = None;
                }
            }
        });
    }
}

/// The root of `validated` applied to `prior` at `at`, for a commit the kernel is deciding. The
/// graph is left for the application that admits the publication ([`apply`] with the same
/// inputs), which takes it rather than applying the transaction again.
pub(crate) fn decide(
    prior: &Revision,
    validated: &ValidatedTransaction,
    validators: &BTreeSet<AgentId>,
    at: Timestamp,
) -> Result<Root, StoreError> {
    let (graph, root) = apply(prior, validated, validators, at)?;
    // The entry either held these inputs already or was just written for them.
    APPLIED.with(|applied| {
        if let Some(applied) = applied.borrow_mut().as_mut() {
            applied.graph = Some(graph);
        }
    });
    Ok(root)
}

/// The graph and root of `validated` applied to `prior` at `at`: the ones the last application on
/// this thread computed when it had exactly these inputs, and otherwise computed now.
pub(crate) fn apply(
    prior: &Revision,
    validated: &ValidatedTransaction,
    validators: &BTreeSet<AgentId>,
    at: Timestamp,
) -> Result<(CanonicalGraph, Root), StoreError> {
    if at < prior.committed_at {
        return Err(StoreError::Document("commit-time-precedes-head".into()));
    }
    let held = prior
        .graph
        .as_ref()
        .ok_or_else(|| crate::replay::refuse(crate::replay::GRAPH_NOT_HELD))?;
    let tx = validated.transaction();
    let transaction = ContentHash::of(tx);
    let reused = APPLIED.with(|applied| {
        applied
            .borrow_mut()
            .as_mut()
            .filter(|applied| applied.holds(prior, held, transaction, validators, at))
            .map(|applied| (applied.root, applied.graph.take()))
    });
    let (graph, root) = match reused {
        Some((root, Some(graph))) => (graph, root),
        Some((root, None)) => (applied_graph(prior, held, tx, validators, at)?, root),
        None => {
            let graph = applied_graph(prior, held, tx, validators, at)?;
            let root = Root {
                revision: graph.revision,
                parent: Some(ContentHash::of(&prior.root)),
                ontology_root: ContentHash::of(&graph.ontology),
                knowledge_root: knowledge_root(&graph),
                evidence_root: evidence_root(&graph),
                agent_root: prior.root.agent_root,
                transaction,
            };
            APPLIED.with(|applied| {
                *applied.borrow_mut() = Some(Applied {
                    prior: Arc::downgrade(held),
                    prior_root: prior.root,
                    prior_committed_at: prior.committed_at,
                    validators: validators.clone(),
                    at,
                    root,
                    graph: None,
                });
            });
            (graph, root)
        }
    };
    Ok((graph, root))
}

/// The graph `tx` makes of `prior`'s graph, `held`.
fn applied_graph(
    prior: &Revision,
    held: &CanonicalGraph,
    tx: &crate::GraphTransaction<ekr_graph::CanonicalValue>,
    validators: &BTreeSet<AgentId>,
    at: Timestamp,
) -> Result<CanonicalGraph, StoreError> {
    GRAPHS_APPLIED.with(|count| count.set(count.get() + 1));
    let mut graph = held.clone();
    graph.revision = prior
        .root
        .revision
        .next()
        .ok_or_else(|| StoreError::Document("revision-overflow".into()))?;
    for op in &tx.operations {
        match op {
            GraphOperation::CreateNode(draft) => {
                let mut node = Node::new(
                    draft.id,
                    draft.root_id,
                    draft.type_id,
                    draft.canonical_name.clone(),
                );
                node.aliases.clone_from(&draft.aliases);
                node.properties = draft
                    .properties
                    .iter()
                    .filter(|(_, values)| !values.is_empty())
                    .map(|(id, values)| (*id, values.clone()))
                    .collect();
                node.type_state = graph
                    .ontology
                    .node_type(draft.type_id)
                    .and_then(|ty| ty.lifecycle.as_ref())
                    .map(|life| life.initial.clone());
                graph.nodes.insert(node.id, node);
            }
            GraphOperation::CreateEdge(draft) => {
                let mut edge = Edge::new(
                    draft.id,
                    draft.root_id,
                    draft.type_id,
                    CanonicalRef::new(draft.source),
                    CanonicalRef::new(draft.target),
                );
                edge.properties = draft
                    .properties
                    .iter()
                    .filter(|(_, values)| !values.is_empty())
                    .map(|(id, values)| (*id, values.clone()))
                    .collect();
                graph.edges.insert(edge.id, edge);
            }
            GraphOperation::AddAssertion(proposed) => {
                let mut assertion = (**proposed).clone();
                assertion.assessment = Assessment::Accepted {
                    validators: validators.clone(),
                };
                assertion.transaction_time = TransactionTime::since(at);
                graph.assertions.insert(assertion.id, assertion);
            }
            // Recorded beside the assertion, which is not changed; validation held it current and
            // the evidence new to it (design § 103.3).
            GraphOperation::AttachEvidence(attachment) => {
                if !graph.assertions.contains_key(&attachment.assertion) {
                    return Err(StoreError::Document("admitted-assertion-missing".into()));
                }
                let revision = graph.revision;
                graph
                    .attachments
                    .entry(attachment.assertion)
                    .or_default()
                    .insert(AttachedEvidence {
                        evidence: CanonicalRef::new(attachment.evidence),
                        revision,
                    });
            }
            // The entry joins retained evidence; its payload is published beside the commit
            // receipt as a Provenance object (`crate::commands`), never inside a record.
            GraphOperation::AddEvidence(addition) => {
                graph
                    .evidence
                    .insert(addition.evidence.id, addition.evidence.clone());
            }
            _ => {}
        }
    }
    for op in &tx.operations {
        match op {
            GraphOperation::UpdateProperty(change) => {
                let node = graph
                    .nodes
                    .get_mut(&change.node)
                    .ok_or_else(|| StoreError::Document("admitted-node-missing".into()))?;
                if change.values.is_empty() {
                    node.properties.remove(&change.property);
                } else {
                    node.properties
                        .insert(change.property, change.values.clone());
                }
            }
            GraphOperation::DeleteEdge(id) => {
                graph.edges.remove(id);
            }
            // After every create, so a node the same transaction creates gains it too.
            GraphOperation::AddAlias(addition) => {
                graph
                    .nodes
                    .get_mut(&addition.node)
                    .ok_or_else(|| StoreError::Document("admitted-node-missing".into()))?
                    .aliases
                    .push(addition.alias.clone());
            }
            GraphOperation::Invoke {
                node, operation, ..
            } => {
                let held = graph
                    .nodes
                    .get_mut(node)
                    .ok_or_else(|| StoreError::Document("admitted-node-missing".into()))?;
                let declared = graph
                    .ontology
                    .node_type(held.type_id)
                    .ok_or_else(|| StoreError::Document("admitted-type-missing".into()))?;
                let op = declared
                    .operations
                    .get(operation)
                    .ok_or_else(|| StoreError::Document("admitted-operation-missing".into()))?;
                if let Some(life) = &declared.lifecycle {
                    held.type_state = Some(
                        life.transition(held.type_state.as_deref().unwrap_or(&life.initial), op)
                            .map_err(|e| StoreError::Document(e.to_string()))?,
                    );
                }
            }
            GraphOperation::RetractAssertion(change) => {
                let held = graph
                    .assertions
                    .get_mut(&change.assertion)
                    .ok_or_else(|| StoreError::Document("admitted-assertion-missing".into()))?;
                held.transaction_time =
                    TransactionTime::new(held.transaction_time.recorded_from, Some(at))
                        .ok_or_else(|| {
                            StoreError::Document("commit-time-precedes-assertion".into())
                        })?;
                held.lifecycle = AssertionLifecycle::Retracted {
                    at_revision: graph.revision,
                    reason: change.reason.clone(),
                };
            }
            GraphOperation::SupersedeAssertion(change) => {
                let held = graph
                    .assertions
                    .get_mut(&change.assertion)
                    .ok_or_else(|| StoreError::Document("admitted-assertion-missing".into()))?;
                held.transaction_time =
                    TransactionTime::new(held.transaction_time.recorded_from, Some(at))
                        .ok_or_else(|| {
                            StoreError::Document("commit-time-precedes-assertion".into())
                        })?;
                held.valid_time.to = Some(change.effective_from);
                held.lifecycle = AssertionLifecycle::Superseded {
                    by: CanonicalRef::new(change.by),
                    at_revision: graph.revision,
                    effective_from: change.effective_from,
                };
            }
            GraphOperation::MergeEntity(_) => {
                return Err(StoreError::Document("unsupported-operation".into()));
            }
            // Applied together below: a schema change is one new version, not three.
            GraphOperation::DefineNodeType(_)
            | GraphOperation::DefineEdgeType(_)
            | GraphOperation::ModifyProperty(_)
            | GraphOperation::WidenEdgeType(_)
            | GraphOperation::CreateNode(_)
            | GraphOperation::CreateEdge(_)
            | GraphOperation::AddAssertion(_)
            | GraphOperation::AttachEvidence(_)
            | GraphOperation::AddEvidence(_) => {}
        }
    }
    // `ekr.p2-apply/1`: a schema change replaces the ontology with the version it names, derived
    // from the prior one with `number + 1`, `parent` = the prior version, and the commit time as
    // its `created_at`. `GraphRoot.schema_version_id` stays the seed's: it is the root's identity,
    // and the ontology root is what carries the version (wave p5-01, decision 4). Validation has
    // already evolved the same changes over the same ontology, so a refusal here is unreachable.
    if let Some(version) = tx.schema_version {
        let changes = crate::validate::schema::changes(tx)
            .ok_or_else(|| StoreError::Document("admitted-mixed-schema-transaction".into()))?;
        graph.ontology = graph
            .ontology
            .evolve(version, at, &changes)
            .map_err(|error| StoreError::Document(error.code().into()))?;
    }
    Ok(graph)
}
