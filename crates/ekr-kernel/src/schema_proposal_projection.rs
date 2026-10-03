//! Field projections between generated transport and generated semantic contracts.
use super::{incubation_projection as p, schema_proposals::error};
use ekr_core::contract_data as w;
use ekr_core::contracts::{
    graph as g, integrate as m, kernel as k, ontology as o, primitives::Uuid,
};
use ekr_store::StoreError;

fn correction(value: &w::EkrKernelClaimCorrection) -> Result<k::ClaimCorrection, StoreError> {
    crate::human_review::correction_from_document(value).map_err(|e| error(e.reason))
}
fn addition(
    value: &w::EkrIntegrateAdditiveSchemaOperation,
) -> Result<m::AdditiveSchemaOperation, StoreError> {
    Ok(match value {
        w::EkrIntegrateAdditiveSchemaOperation::V0(v) => {
            m::AdditiveSchemaOperation::AddOptionalProperty(m::OptionalPropertyAddition {
                owner_type: v.value.owner_type.clone(),
                property: p::property(&v.value.property)?,
            })
        }
        w::EkrIntegrateAdditiveSchemaOperation::V1(v) => {
            m::AdditiveSchemaOperation::DefineRelation(m::EdgeTypeSpec {
                name: v.value.name.clone(),
                source_types: v.value.source_types.clone(),
                target_types: v.value.target_types.clone(),
                cardinality: p::cardinality(&v.value.cardinality),
                properties: v
                    .value
                    .properties
                    .iter()
                    .map(|v| p::property(v))
                    .collect::<Result<_, _>>()?,
            })
        }
        w::EkrIntegrateAdditiveSchemaOperation::V2(v) => {
            m::AdditiveSchemaOperation::DefineType(m::NodeTypeSpec {
                name: v.value.name.clone(),
                parents: v.value.parents.clone(),
                abstract_type: v.value.abstract_type,
                properties: v
                    .value
                    .properties
                    .iter()
                    .map(|v| p::property(v))
                    .collect::<Result<_, _>>()?,
            })
        }
    })
}
fn mapping(value: &w::EkrIntegrateKnowledgeMapping) -> Result<m::KnowledgeMapping, StoreError> {
    Ok(m::KnowledgeMapping {
        source: p::version(&value.source)?,
        source_item: value.source_item.clone(),
        source_type: value.source_type.clone(),
        target_type: value.target_type.clone(),
        target_member: value.target_member.clone(),
        value: match value.value.as_ref() {
            w::EkrIntegrateMappingValue::V0(v) => {
                m::MappingValue::Constant(p::typed_value(&v.value)?)
            }
            w::EkrIntegrateMappingValue::V1(v) => {
                m::MappingValue::CopyField(m::DeclaredSourceField {
                    declaration: v.value.declaration.clone(),
                    field: v.value.field.clone(),
                })
            }
            w::EkrIntegrateMappingValue::V2(v) => {
                m::MappingValue::CopyRelation(m::DeclaredSourceRelation {
                    declaration: v.value.declaration.clone(),
                    relation: v.value.relation.clone(),
                })
            }
        },
    })
}
pub(super) fn document(
    value: &w::EkrIntegrateSchemaProposalDocument,
) -> Result<m::SchemaProposalDocument, StoreError> {
    Ok(m::SchemaProposalDocument {
        observations: value
            .observations
            .iter()
            .map(|v| g::ObservationId(Uuid(v.0.clone())))
            .collect(),
        proposal_id: m::SchemaProposalId(Uuid(value.proposal_id.0.clone())),
        base_schema: o::SchemaVersionId(Uuid(value.base_schema.0.clone())),
        sources: value
            .sources
            .iter()
            .map(|v| {
                Ok(m::ProposalSource {
                    version: p::version(&v.version)?,
                    items: v.items.clone(),
                })
            })
            .collect::<Result<_, StoreError>>()?,
        evidence: value
            .evidence
            .iter()
            .map(|v| g::EvidenceId(Uuid(v.0.clone())))
            .collect(),
        additions: value
            .additions
            .iter()
            .map(|v| addition(v))
            .collect::<Result<_, _>>()?,
        mappings: value
            .mappings
            .iter()
            .map(|v| mapping(v))
            .collect::<Result<_, _>>()?,
        corrections: value
            .corrections
            .iter()
            .map(|v| correction(v))
            .collect::<Result<_, _>>()?,
        explanation: value.explanation.clone(),
    })
}
pub(super) fn preview(
    value: &w::EkrIntegrateMappingPreview,
) -> Result<m::MappingPreview, StoreError> {
    Ok(m::MappingPreview {
        source: p::version(&value.source)?,
        item: value.item.clone(),
        mapping: mapping(&value.mapping)?,
        blockers: value.blockers.clone(),
        corrections: value
            .corrections
            .iter()
            .map(|v| correction(v))
            .collect::<Result<_, _>>()?,
    })
}
pub(super) fn submitted(
    value: &w::EkrIntegrateSchemaProposalRead,
) -> Result<m::SubmitSchemaProposalResult, StoreError> {
    Ok(m::SubmitSchemaProposalResult {
        proposal_id: m::SchemaProposalId(Uuid(value.proposal.proposal_id.0.clone())),
        proposal_digest: k::ContentHash(value.proposal_digest.0.clone()),
        preview: value
            .preview
            .iter()
            .map(|v| preview(v))
            .collect::<Result<_, _>>()?,
    })
}
pub(super) fn shown(
    value: &w::EkrIntegrateSchemaProposalRead,
) -> Result<m::ShowSchemaProposalResult, StoreError> {
    // Review and application persistence are separate handlers; never silently drop their history.
    if !value.receipts.is_empty() {
        return Err(error("unsupported application history projection"));
    }
    Ok(m::ShowSchemaProposalResult {
        proposal: document(&value.proposal)?,
        proposal_digest: k::ContentHash(value.proposal_digest.0.clone()),
        preview: value
            .preview
            .iter()
            .map(|v| preview(v))
            .collect::<Result<_, _>>()?,
        reviews: value
            .reviews
            .iter()
            .map(|v| review(v))
            .collect::<Result<_, _>>()?,
        receipts: Vec::new(),
        basis: k::ReviewBasis {
            observed_revision: k::RevisionNumber(
                value
                    .basis
                    .observed_revision
                    .0
                    .as_i64()
                    .ok_or_else(|| error("revision exceeds generated command range"))?,
            ),
            evidence_digest: k::ContentHash(value.basis.evidence_digest.0.clone()),
            options_digest: k::ContentHash(value.basis.options_digest.0.clone()),
            effects_digest: k::ContentHash(value.basis.effects_digest.0.clone()),
        },
        expected_previous_decision: match &value.expected_previous_decision {
            w::EssPresence::Absent => None,
            w::EssPresence::Present(hash) => Some(k::ContentHash(hash.0.clone())),
        },
    })
}

pub(super) fn review(
    value: &w::EkrIntegrateProposalReviewSnapshot,
) -> Result<m::ProposalReviewSnapshot, StoreError> {
    Ok(m::ProposalReviewSnapshot {
        review_id: m::ProposalReviewId(Uuid(value.review_id.0.clone())),
        proposal_id: m::SchemaProposalId(Uuid(value.proposal_id.0.clone())),
        proposal_digest: k::ContentHash(value.proposal_digest.0.clone()),
        human_proof_digest: k::ContentHash(value.human_proof_digest.0.clone()),
        basis: crate::human_review::basis_from_document(&value.basis)
            .map_err(|e| error(e.reason))?,
        decision: match *value.decision {
            w::EkrIntegrateReviewDecision::V0 => m::ReviewDecision::Approved,
            w::EkrIntegrateReviewDecision::V1 => m::ReviewDecision::Rejected,
        },
        operator: k::TrustedOperatorIdentity {
            actor: k::AgentId(Uuid(value.operator.actor.0.clone())),
            authentication_subject: value.operator.authentication_subject.clone(),
        },
        evidence_id: g::EvidenceId(Uuid(value.evidence_id.0.clone())),
        recorded_at: ekr_core::contracts::primitives::Timestamp(
            crate::incubation_document::timestamp_text(&value.recorded_at)?,
        ),
    })
}
