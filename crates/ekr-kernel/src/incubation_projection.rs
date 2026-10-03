//! Lossless field projection from generated wire records into generated command contracts.
use super::incubation_document::error;
use ekr_core::contract_data as w;
use ekr_core::contracts::{
    graph as g, integrate as i, kernel as k, ontology as o, primitives as p,
};
use ekr_store::StoreError;

pub(super) fn bytes(value: &str) -> Result<Vec<u8>, StoreError> {
    ekr_core::bytes::decode(value).map_err(error)
}
fn integer(value: &serde_json::Number) -> Result<i64, StoreError> {
    value
        .as_i64()
        .ok_or_else(|| error("integer exceeds generated command range"))
}
fn optional<T, R>(
    value: &w::EssPresence<T>,
    f: impl FnOnce(&T) -> Result<R, StoreError>,
) -> Result<Option<R>, StoreError> {
    match value {
        w::EssPresence::Absent => Ok(None),
        w::EssPresence::Present(value) => f(value).map(Some),
    }
}
fn copied<T: Clone>(value: &w::EssPresence<T>) -> Option<T> {
    match value {
        w::EssPresence::Absent => None,
        w::EssPresence::Present(value) => Some(value.clone()),
    }
}
fn uuid(value: &str) -> p::Uuid {
    p::Uuid(value.into())
}
fn hash(value: &w::EkrKernelContentHash) -> k::ContentHash {
    k::ContentHash(value.0.clone())
}
fn cardinality(value: &w::EkrOntologyCardinality) -> o::Cardinality {
    match value {
        w::EkrOntologyCardinality::V0 => o::Cardinality::Many,
        w::EkrOntologyCardinality::V1 => o::Cardinality::One,
    }
}
fn value_kind(value: &w::EkrOntologyValueKind) -> o::ValueKind {
    use w::EkrOntologyValueKind as W;
    match value {
        W::V0 => o::ValueKind::Boolean,
        W::V1 => o::ValueKind::Decimal,
        W::V2 => o::ValueKind::Duration,
        W::V3 => o::ValueKind::Enum,
        W::V4 => o::ValueKind::Float,
        W::V5 => o::ValueKind::Integer,
        W::V6 => o::ValueKind::List,
        W::V7 => o::ValueKind::NodeRef,
        W::V8 => o::ValueKind::Record,
        W::V9 => o::ValueKind::String,
        W::V10 => o::ValueKind::Timestamp,
    }
}
fn value_spec(value: &w::EkrIntegrateValueSpec) -> Result<i::ValueSpec, StoreError> {
    Ok(i::ValueSpec {
        value_kind: value_kind(&value.value_kind),
        allowed_types: copied(&value.allowed_types),
        variants: copied(&value.variants),
        element: optional(&value.element, |value| value_spec(value).map(Box::new))?,
        fields: optional(&value.fields, |fields| {
            fields
                .ess_extra
                .iter()
                .map(|(key, value)| Ok((key.clone(), Box::new(value_spec(value)?))))
                .collect::<Result<_, StoreError>>()
        })?,
    })
}
fn property(value: &w::EkrIntegratePropertySpec) -> Result<i::PropertySpec, StoreError> {
    Ok(i::PropertySpec {
        name: value.name.clone(),
        value: Box::new(value_spec(&value.value)?),
        cardinality: cardinality(&value.cardinality),
        required: value.required,
    })
}
fn schema(value: &w::EkrIntegrateOntologySpec) -> Result<i::OntologySpec, StoreError> {
    Ok(i::OntologySpec {
        node_types: value
            .node_types
            .iter()
            .map(|node| {
                Ok(i::NodeTypeSpec {
                    name: node.name.clone(),
                    parents: node.parents.clone(),
                    abstract_type: node.abstract_type,
                    properties: node
                        .properties
                        .iter()
                        .map(|value| property(value))
                        .collect::<Result<_, StoreError>>()?,
                })
            })
            .collect::<Result<_, StoreError>>()?,
        edge_types: value
            .edge_types
            .iter()
            .map(|edge| {
                Ok(i::EdgeTypeSpec {
                    name: edge.name.clone(),
                    source_types: edge.source_types.clone(),
                    target_types: edge.target_types.clone(),
                    cardinality: cardinality(&edge.cardinality),
                    properties: edge
                        .properties
                        .iter()
                        .map(|value| property(value))
                        .collect::<Result<_, StoreError>>()?,
                })
            })
            .collect::<Result<_, StoreError>>()?,
    })
}
fn reference(value: &w::EkrIntegrateExtractedReference) -> i::ExtractedReference {
    i::ExtractedReference {
        node_type: value.node_type.clone(),
        aliases: value.aliases.clone(),
    }
}
fn typed_value(value: &w::EkrGraphTypedValue) -> Result<g::TypedValue, StoreError> {
    use w::EkrGraphCanonicalValueKind as W;
    Ok(g::TypedValue {
        kind: match *value.kind {
            W::V0 => g::CanonicalValueKind::Boolean,
            W::V1 => g::CanonicalValueKind::Decimal,
            W::V2 => g::CanonicalValueKind::Duration,
            W::V3 => g::CanonicalValueKind::Enum,
            W::V4 => g::CanonicalValueKind::Integer,
            W::V5 => g::CanonicalValueKind::List,
            W::V6 => g::CanonicalValueKind::NodeRef,
            W::V7 => g::CanonicalValueKind::Record,
            W::V8 => g::CanonicalValueKind::String,
            W::V9 => g::CanonicalValueKind::Timestamp,
        },
        canonical_bytes: bytes(&value.canonical_bytes)?,
    })
}
fn fact(value: &w::EkrIntegrateExtractedFact) -> Result<i::ExtractedFact, StoreError> {
    Ok(match value {
        w::EkrIntegrateExtractedFact::V0(value) => {
            let value = &value.value;
            i::ExtractedFact::Property(i::PropertyFact {
                subject: reference(&value.subject),
                property: value.property.clone(),
                value: typed_value(&value.value)?,
                evidence: value
                    .evidence
                    .iter()
                    .map(|id| g::EvidenceId(uuid(&id.0)))
                    .collect(),
            })
        }
        w::EkrIntegrateExtractedFact::V1(value) => {
            let value = &value.value;
            i::ExtractedFact::Relation(i::RelationFact {
                subject: reference(&value.subject),
                relation: value.relation.clone(),
                object: reference(&value.object),
                evidence: value
                    .evidence
                    .iter()
                    .map(|id| g::EvidenceId(uuid(&id.0)))
                    .collect(),
            })
        }
    })
}
fn evidence(
    value: &w::EkrKernelEvidenceAdditionProjection,
) -> Result<k::EvidenceAdditionProjection, StoreError> {
    use w::EkrGraphEvidenceKind as W;
    let source = &value.evidence.source;
    Ok(k::EvidenceAdditionProjection {
        evidence: g::EvidenceRecord {
            id: g::EvidenceId(uuid(&value.evidence.id.0)),
            source: g::EvidenceSourceProjection {
                kind: match *source.kind {
                    W::V0 => g::EvidenceKind::DatabaseRecord,
                    W::V1 => g::EvidenceKind::Document,
                    W::V2 => g::EvidenceKind::GraphAssertion,
                    W::V3 => g::EvidenceKind::HumanStatement,
                    W::V4 => g::EvidenceKind::Observation,
                    W::V5 => g::EvidenceKind::Url,
                },
                url: copied(&source.url),
                document_id: copied(&source.document_id),
                section: copied(&source.section),
                database: copied(&source.database),
                table: copied(&source.table),
                key: copied(&source.key),
                identity: copied(&source.identity),
                assertion: optional(&source.assertion, |id| Ok(g::AssertionId(uuid(&id.0))))?,
                observation: optional(&source.observation, |id| Ok(g::ObservationId(uuid(&id.0))))?,
            },
            content_hash: hash(&value.evidence.content_hash),
            extracted_by: k::AgentId(uuid(&value.evidence.extracted_by.0)),
            observed_at: p::Timestamp(value.evidence.observed_at.clone()),
            confidence_bp: integer(&value.evidence.confidence_bp)?,
        },
        payload: bytes(&value.payload)?,
    })
}
pub(super) fn document(
    value: &w::EkrIntegrateInterpretationDocument,
) -> Result<i::InterpretationDocument, StoreError> {
    Ok(i::InterpretationDocument {
        version: i::InterpretationCoordinate {
            interpretation_id: i::InterpretationId(uuid(&value.version.interpretation_id.0)),
            version: integer(&value.version.version)?,
        },
        root_id: g::GraphRootId(uuid(&value.root_id.0)),
        observations: value
            .observations
            .iter()
            .map(|id| g::ObservationId(uuid(&id.0)))
            .collect(),
        local_schema: schema(&value.local_schema)?,
        entities: value
            .entities
            .iter()
            .map(|value| reference(value))
            .collect(),
        facts: value
            .facts
            .iter()
            .map(|value| fact(value))
            .collect::<Result<_, StoreError>>()?,
        evidence: value
            .evidence
            .iter()
            .map(|value| evidence(value))
            .collect::<Result<_, StoreError>>()?,
    })
}
pub(super) fn version(
    value: &w::EkrIntegrateInterpretationVersion,
) -> Result<i::InterpretationVersion, StoreError> {
    Ok(i::InterpretationVersion {
        interpretation_id: i::InterpretationId(uuid(&value.interpretation_id.0)),
        version: integer(&value.version)?,
        document_digest: hash(&value.document_digest),
    })
}
pub(super) fn wire_version(
    value: i::InterpretationVersion,
) -> w::EkrIntegrateInterpretationVersion {
    w::EkrIntegrateInterpretationVersion {
        interpretation_id: Box::new(w::EkrIntegrateInterpretationId(
            value.interpretation_id.0 .0,
        )),
        version: value.version.into(),
        document_digest: Box::new(w::EkrKernelContentHash(value.document_digest.0)),
    }
}
pub(super) fn receipt(
    value: &w::EkrIntegrateIncubationImportReceipt,
) -> Result<i::IncubationImportReceipt, StoreError> {
    use w::EkrIntegrateIncubationOutcome as W;
    Ok(i::IncubationImportReceipt {
        outcome: match *value.outcome {
            W::V0 => i::IncubationOutcome::AlreadyRetained,
            W::V1 => i::IncubationOutcome::Retained,
            W::V2 => i::IncubationOutcome::RetainedWithBlockers,
        },
        version: version(&value.version)?,
        already_retained: value.already_retained,
        blockers: value
            .blockers
            .iter()
            .map(|id| i::IntegrationBlockerId(uuid(&id.0)))
            .collect(),
    })
}
fn blocker(value: &w::EkrIntegrateIntegrationBlockerSnapshot) -> i::IntegrationBlockerSnapshot {
    use w::EkrIntegrateIntegrationBlockerKind as W;
    i::IntegrationBlockerSnapshot {
        blocker_id: i::IntegrationBlockerId(uuid(&value.blocker_id.0)),
        document_digest: hash(&value.document_digest),
        item: value.item.clone(),
        declaration: value.declaration.clone(),
        reason: value.reason.clone(),
        basis_digest: hash(&value.basis_digest),
        kind: match *value.kind {
            W::V0 => i::IntegrationBlockerKind::Contradiction,
            W::V1 => i::IntegrationBlockerKind::RejectedInterpretation,
            W::V2 => i::IntegrationBlockerKind::UnknownProperty,
            W::V3 => i::IntegrationBlockerKind::UnknownRelation,
            W::V4 => i::IntegrationBlockerKind::UnknownType,
            W::V5 => i::IntegrationBlockerKind::UnresolvedReference,
            W::V6 => i::IntegrationBlockerKind::ValueMismatch,
        },
    }
}
fn processing(
    value: &w::EkrIntegrateProcessingReceiptSnapshot,
) -> Result<i::ProcessingReceiptSnapshot, StoreError> {
    use w::EkrIntegrateProcessingDisposition as W;
    Ok(i::ProcessingReceiptSnapshot {
        receipt_id: i::ProcessingReceiptId(uuid(&value.receipt_id.0)),
        mapping_digest: optional(&value.mapping_digest, |value| Ok(hash(value)))?,
        document_digest: hash(&value.document_digest),
        item: value.item.clone(),
        disposition: match *value.disposition {
            W::V0 => i::ProcessingDisposition::AlreadyIntegrated,
            W::V1 => i::ProcessingDisposition::Integrated,
            W::V2 => i::ProcessingDisposition::Parked,
            W::V3 => i::ProcessingDisposition::Rejected,
        },
        transaction_id: optional(&value.transaction_id, |id| {
            Ok(k::TransactionId(uuid(&id.0)))
        })?,
        assertions: value
            .assertions
            .iter()
            .map(|id| g::AssertionId(uuid(&id.0)))
            .collect(),
        basis_digest: hash(&value.basis_digest),
    })
}
pub(super) fn read(
    value: &w::EkrIntegrateInterpretationRead,
) -> Result<i::ShowInterpretationResult, StoreError> {
    Ok(i::ShowInterpretationResult {
        document: document(&value.document)?,
        blockers: value.blockers.iter().map(|value| blocker(value)).collect(),
        receipts: value
            .receipts
            .iter()
            .map(|value| processing(value))
            .collect::<Result<_, StoreError>>()?,
    })
}
