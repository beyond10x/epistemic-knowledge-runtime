//! Deterministic mapping material and immutable mapping coordinates; no allocation or writes.
use crate::{schema_proposals::Source, VerifiedRead};
use ekr_core::{contract_data as w, ContentHash, EvidenceId};
use ekr_graph::{CanonicalRef, CanonicalValue, Edge, Node, Object, Predicate, Subject};
use ekr_ontology::Ontology;
use ekr_store::StoreError;
use std::collections::BTreeMap;

#[derive(Debug, PartialEq)]
pub(crate) struct MappedClaim {
    pub subject: Subject<CanonicalRef<Node>, CanonicalRef<Edge>>,
    pub predicate: Predicate,
    pub object: Object<CanonicalValue>,
    pub evidence: Vec<EvidenceId>,
}
#[derive(Debug, PartialEq)]
pub(crate) enum MappingOutcome {
    Ready(MappedClaim),
    Blocked(Vec<String>),
}

/// Resolve against already authenticated source documents and the caller's checked ontology.
/// Source lookup and every selector/type/endpoint rule are the same implementation as preview.
/// This returns no authority and allocates no entity or assertion identity.
pub(crate) fn resolve(
    read: &VerifiedRead,
    mapping: &w::EkrIntegrateKnowledgeMapping,
    ontology: &Ontology,
    sources: &BTreeMap<String, Source>,
) -> Result<MappingOutcome, StoreError> {
    let nodes = crate::schema_proposal_mapping::mapping_nodes(read);
    match crate::schema_proposal_mapping::mapping_material(
        read, mapping, ontology, sources, &nodes, false,
    ) {
        Ok(material) => Ok(material.outcome),
        // Non-strict preview renders the same per-mapping document refusals as durable blockers.
        Err(StoreError::Document(reason)) => Ok(MappingOutcome::Blocked(vec![reason])),
        Err(error) => Err(error),
    }
}

/// Exact compact generated JSON, without a newline or a self-referential digest envelope.
pub(crate) fn payload(mapping: &w::EkrIntegrateKnowledgeMapping) -> Result<Vec<u8>, StoreError> {
    serde_json::to_vec(mapping).map_err(crate::schema_proposals::error)
}
pub(crate) fn digest(mapping: &w::EkrIntegrateKnowledgeMapping) -> Result<ContentHash, StoreError> {
    Ok(ContentHash::of_bytes(&payload(mapping)?))
}
/// A source item is qualified by its complete immutable document version and mapping content.
pub(crate) fn item(
    mapping: &w::EkrIntegrateKnowledgeMapping,
) -> Result<w::EkrIntegrateApplicationItemKey, StoreError> {
    crate::schema_proposals::fact_index(&mapping.source_item)?;
    Ok(w::EkrIntegrateApplicationItemKey {
        source: mapping.source.clone(),
        item: mapping.source_item.clone(),
        mapping_digest: Box::new(w::EkrKernelContentHash(digest(mapping)?.to_string())),
    })
}
#[cfg(test)]
mod tests;
