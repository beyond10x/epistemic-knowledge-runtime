//! Pure unordered application, reachable only with the kernel's private validated capability.
use crate::{GraphOperation, ValidatedTransaction};
use ekr_core::{AgentId, ContentHash, Timestamp};
use ekr_graph::{
    AssertionLifecycle, Assessment, CanonicalGraph, CanonicalRef, Edge, Node, Root, TransactionTime,
};
use ekr_store::{evidence_root, knowledge_root, AdmittedRevision, StoreError};
use std::collections::BTreeSet;

pub(crate) fn apply(
    prior: &AdmittedRevision,
    validated: &ValidatedTransaction,
    validators: &BTreeSet<AgentId>,
    at: Timestamp,
) -> Result<(CanonicalGraph, Root), StoreError> {
    if at < prior.committed_at {
        return Err(StoreError::Document("commit-time-precedes-head".into()));
    }
    let mut graph = prior.graph.clone();
    let tx = validated.transaction();
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
                    by: change.by,
                    at_revision: graph.revision,
                    effective_from: change.effective_from,
                };
            }
            GraphOperation::DefineNodeType(_)
            | GraphOperation::DefineEdgeType(_)
            | GraphOperation::ModifyProperty(_)
            | GraphOperation::MergeEntity(_) => {
                return Err(StoreError::Document("unsupported-operation".into()));
            }
            GraphOperation::CreateNode(_)
            | GraphOperation::CreateEdge(_)
            | GraphOperation::AddAssertion(_) => {}
        }
    }
    let root = Root {
        revision: graph.revision,
        parent: Some(ContentHash::of(&prior.root)),
        ontology_root: ContentHash::of(&graph.ontology),
        knowledge_root: knowledge_root(&graph),
        evidence_root: evidence_root(&graph),
        agent_root: prior.root.agent_root,
        transaction: ContentHash::of(tx),
    };
    Ok((graph, root))
}
