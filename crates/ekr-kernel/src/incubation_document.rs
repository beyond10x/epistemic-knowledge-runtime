//! Adapts generated interpretation contracts to the existing local shape validator.
use ekr_core::contract_data::*;
use ekr_core::Timestamp;
use ekr_integrate::extraction as old;
use ekr_store::StoreError;

pub(super) fn error(detail: impl std::fmt::Display) -> StoreError {
    StoreError::Document(format!("incubation-document: {detail}"))
}

pub(super) fn timestamp(value: &str) -> Result<Timestamp, StoreError> {
    let parsed = time::OffsetDateTime::parse(value, &time::format_description::well_known::Rfc3339)
        .map_err(error)?;
    let nanos = parsed.unix_timestamp_nanos();
    if nanos % 1_000_000 != 0 {
        return Err(error("timestamp exceeds millisecond precision"));
    }
    Ok(Timestamp::from_millis(
        (nanos / 1_000_000).try_into().map_err(error)?,
    ))
}

fn required<T>(value: &EssPresence<T>) -> Result<&T, StoreError> {
    match value {
        EssPresence::Present(v) => Ok(v),
        EssPresence::Absent => Err(error("missing value type parameter")),
    }
}

fn value_spec(value: &EkrIntegrateValueSpec) -> Result<old::ValueSpec, StoreError> {
    use EkrOntologyValueKind as K;
    let expected = match *value.value_kind {
        K::V3 => 1,
        K::V6 => 2,
        K::V7 => 4,
        K::V8 => 8,
        _ => 0,
    };
    let present = u8::from(matches!(value.variants, EssPresence::Present(_)))
        | (u8::from(matches!(value.element, EssPresence::Present(_))) << 1)
        | (u8::from(matches!(value.allowed_types, EssPresence::Present(_))) << 2)
        | (u8::from(matches!(value.fields, EssPresence::Present(_))) << 3);
    if present != expected {
        return Err(error("value type has missing or unrelated parameters"));
    }
    Ok(match *value.value_kind {
        K::V0 => old::ValueSpec::Boolean,
        K::V1 => old::ValueSpec::Decimal,
        K::V2 => old::ValueSpec::Duration,
        K::V3 => old::ValueSpec::Enum {
            variants: required(&value.variants)?.clone(),
        },
        K::V4 => old::ValueSpec::Float,
        K::V5 => old::ValueSpec::Integer,
        K::V6 => old::ValueSpec::List(Box::new(value_spec(required(&value.element)?)?)),
        K::V7 => old::ValueSpec::NodeRef {
            allowed_types: required(&value.allowed_types)?.clone(),
        },
        K::V8 => old::ValueSpec::Record(
            required(&value.fields)?
                .ess_extra
                .iter()
                .map(|(k, v)| Ok((k.clone(), value_spec(v)?)))
                .collect::<Result<_, StoreError>>()?,
        ),
        K::V9 => old::ValueSpec::String,
        K::V10 => old::ValueSpec::Timestamp,
    })
}

fn property(input: &EkrIntegratePropertySpec) -> Result<old::PropertySpec, StoreError> {
    Ok(old::PropertySpec {
        name: input.name.clone(),
        value: value_spec(&input.value)?,
        cardinality: serde_json::from_value(
            serde_json::to_value(&input.cardinality).map_err(error)?,
        )
        .map_err(error)?,
        required: input.required,
    })
}

fn reference(input: &EkrIntegrateExtractedReference) -> old::ExtractedReference {
    old::ExtractedReference {
        node_type: input.node_type.clone(),
        aliases: input.aliases.clone(),
    }
}

/// Conversion checks every generated projection field. Source admission belongs to the caller.
pub(super) fn project(
    input: &EkrIntegrateInterpretationDocument,
) -> Result<old::ExtractionDocument, StoreError> {
    let ontology = old::OntologySpec {
        node_types: input
            .local_schema
            .node_types
            .iter()
            .map(|t| {
                Ok(old::NodeTypeSpec {
                    name: t.name.clone(),
                    parents: t.parents.clone(),
                    abstract_type: t.abstract_type,
                    properties: t
                        .properties
                        .iter()
                        .map(|p| property(p))
                        .collect::<Result<_, _>>()?,
                })
            })
            .collect::<Result<_, StoreError>>()?,
        edge_types: input
            .local_schema
            .edge_types
            .iter()
            .map(|t| {
                Ok(old::EdgeTypeSpec {
                    name: t.name.clone(),
                    source_types: t.source_types.clone(),
                    target_types: t.target_types.clone(),
                    cardinality: serde_json::from_value(
                        serde_json::to_value(&t.cardinality).map_err(error)?,
                    )
                    .map_err(error)?,
                    properties: t
                        .properties
                        .iter()
                        .map(|p| property(p))
                        .collect::<Result<_, _>>()?,
                })
            })
            .collect::<Result<_, StoreError>>()?,
    };
    let facts = input
        .facts
        .iter()
        .map(|fact| {
            Ok(match fact.as_ref() {
                EkrIntegrateExtractedFact::V0(p) => {
                    old::ExtractedFact::Property(old::PropertyFact {
                        subject: reference(&p.value.subject),
                        property: p.value.property.clone(),
                        value: super::incubation_value::decode(&p.value.value)?,
                        evidence: p
                            .value
                            .evidence
                            .iter()
                            .map(|e| e.0.parse().map_err(error))
                            .collect::<Result<_, _>>()?,
                    })
                }
                EkrIntegrateExtractedFact::V1(r) => {
                    old::ExtractedFact::Relation(old::RelationFact {
                        subject: reference(&r.value.subject),
                        relation: r.value.relation.clone(),
                        object: reference(&r.value.object),
                        evidence: r
                            .value
                            .evidence
                            .iter()
                            .map(|e| e.0.parse().map_err(error))
                            .collect::<Result<_, _>>()?,
                    })
                }
            })
        })
        .collect::<Result<_, StoreError>>()?;
    let evidence = input
        .evidence
        .iter()
        .map(|item| {
            let e = &item.evidence;
            let source = &e.source;
            // Refuse locator ambiguity, including irrelevant fields hidden beside the chosen kind.
            let encoded = serde_json::to_value(source).map_err(error)?;
            let keys: Vec<_> = encoded
                .as_object()
                .ok_or_else(|| error("evidence source"))?
                .keys()
                .map(String::as_str)
                .collect();
            let source = match *source.kind {
                EkrGraphEvidenceKind::V4 if keys == ["kind", "observation"] => {
                    ekr_graph::EvidenceSource::Observation(
                        required(&source.observation)?.0.parse().map_err(error)?,
                    )
                }
                EkrGraphEvidenceKind::V3 if keys == ["kind"] || keys == ["identity", "kind"] => {
                    ekr_graph::EvidenceSource::HumanStatement {
                        identity: match &source.identity {
                            EssPresence::Absent => None,
                            EssPresence::Present(id) => Some(id.clone()),
                        },
                    }
                }
                _ => return Err(error("inadmissible evidence source")),
            };
            let bp = e
                .confidence_bp
                .as_u64()
                .and_then(|n| u16::try_from(n).ok())
                .and_then(ekr_graph::Confidence::from_basis_points)
                .ok_or_else(|| error("confidence out of range"))?;
            Ok(old::ExtractionEvidence {
                evidence: ekr_graph::Evidence {
                    id: e.id.0.parse().map_err(error)?,
                    source,
                    content_hash: e.content_hash.0.parse().map_err(error)?,
                    observed_at: timestamp(&e.observed_at)?,
                    extracted_by: e.extracted_by.0.parse().map_err(error)?,
                    confidence: bp,
                },
                payload: ekr_core::bytes::decode(&item.payload).map_err(error)?,
            })
        })
        .collect::<Result<_, StoreError>>()?;
    Ok(old::ExtractionDocument {
        format: old::ExtractionFormat::V1,
        ontology,
        entities: input.entities.iter().map(|r| reference(r)).collect(),
        facts,
        evidence,
    })
}
