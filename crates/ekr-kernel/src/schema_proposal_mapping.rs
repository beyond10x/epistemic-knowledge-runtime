//! Explicit selectors and typed values; no expression language or inferred entity creation.
use super::schema_proposals::{coordinate, error, fact_index, Source};
use crate::VerifiedRead;
use ekr_core::contract_data as w;
use ekr_core::{AssertionId, NodeId, TypeId};
use ekr_graph::{Assertion, GraphSnapshot, Predicate, Subject};
use ekr_integrate::{ExtractedFact, ResolutionOutcome, TypedReference};
use ekr_ontology::{Ontology, Value, ValueType};
use ekr_store::StoreError;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default, serde::Serialize)]
pub(super) struct Effects {
    bindings: Vec<(Option<NodeId>, Option<NodeId>, Option<Value>)>,
    claims: BTreeMap<AssertionId, Assertion>,
    properties: Vec<Option<ekr_ontology::PropertyDefinition>>,
    relations: Vec<Option<ekr_ontology::EdgeType>>,
}

fn resolve(
    read: &VerifiedRead,
    type_id: TypeId,
    aliases: &[String],
    role: &str,
    blockers: &mut Vec<String>,
) -> Option<NodeId> {
    if read.graph.ontology.node_type(type_id).is_none() {
        blockers.push(format!("{role}: proposed type has no canonical entities"));
        return None;
    }
    let reference = TypedReference {
        type_id,
        aliases: aliases.to_vec(),
    };
    match ekr_integrate::resolve_indexed(
        GraphSnapshot::of(&read.graph),
        &read.aliases(),
        &reference,
    ) {
        ResolutionOutcome::Resolved(resolved) => Some(resolved.node_id),
        other => {
            blockers.push(format!("{role}: {other:?}"));
            None
        }
    }
}

// Generated transport collections intentionally own boxed records.
#[allow(clippy::vec_box)]
pub(super) fn preview(
    read: &VerifiedRead,
    proposal: &w::EkrIntegrateSchemaProposalDocument,
    ontology: &Ontology,
    sources: &BTreeMap<String, Source>,
    strict: bool,
) -> Result<(Vec<Box<w::EkrIntegrateMappingPreview>>, Effects), StoreError> {
    match checked_preview(read, proposal, ontology, sources, strict) {
        Ok(result) => return Ok(result),
        Err(StoreError::Document(_)) if !strict => {}
        Err(error) => return Err(error),
    }
    let mut result = Vec::new();
    let mut effects = Effects::default();
    for mapping in &proposal.mappings {
        let mut item = proposal.clone();
        item.mappings = vec![mapping.clone()];
        match checked_preview(read, &item, ontology, sources, false) {
            Ok((preview, material)) => {
                result.extend(preview);
                effects.bindings.extend(material.bindings);
                effects.claims.extend(material.claims);
                effects.properties.extend(material.properties);
                effects.relations.extend(material.relations);
            }
            Err(StoreError::Document(reason)) => {
                result.push(Box::new(w::EkrIntegrateMappingPreview {
                    source: mapping.source.clone(),
                    item: mapping.source_item.clone(),
                    mapping: mapping.clone(),
                    blockers: vec![reason],
                    corrections: proposal.corrections.clone(),
                }))
            }
            Err(error) => return Err(error),
        }
    }
    for correction in &proposal.corrections {
        let id = correction.assertion_id.0.parse().map_err(error)?;
        if let Some(claim) = read.graph.assertions.get(&id) {
            effects.claims.insert(id, claim.clone());
        }
    }
    Ok((result, effects))
}

#[allow(clippy::vec_box)]
fn checked_preview(
    read: &VerifiedRead,
    proposal: &w::EkrIntegrateSchemaProposalDocument,
    ontology: &Ontology,
    sources: &BTreeMap<String, Source>,
    strict: bool,
) -> Result<(Vec<Box<w::EkrIntegrateMappingPreview>>, Effects), StoreError> {
    let nodes = mapping_nodes(read);
    let mut preview = Vec::new();
    let mut effects = Effects {
        bindings: Vec::new(),
        claims: BTreeMap::new(),
        properties: Vec::new(),
        relations: Vec::new(),
    };
    let mut seen = BTreeSet::new();
    for mapping in &proposal.mappings {
        let coordinate = coordinate(&mapping.source)?;
        if !seen.insert((
            coordinate,
            &mapping.source_item,
            &mapping.target_type,
            &mapping.target_member,
        )) {
            return Err(error("duplicate source/item/target mapping"));
        }
        let material = mapping_material(read, mapping, ontology, sources, &nodes, strict)?;
        let subject = material.binding.0;
        let predicate = material.predicate;
        let blockers = match &material.outcome {
            crate::application_mapping::MappingOutcome::Ready(_) => Vec::new(),
            crate::application_mapping::MappingOutcome::Blocked(blockers) => blockers.clone(),
        };
        effects.properties.push(material.property);
        effects.relations.push(material.relation);
        if let Some(subject) = subject {
            for claim in read.graph.assertions.values().filter(|claim| {
                matches!(claim.subject, Subject::Node(id) if id.id() == subject)
                    && claim.predicate == predicate
            }) {
                effects.claims.insert(claim.id, claim.clone());
            }
        }
        effects.bindings.push(material.binding);
        preview.push(Box::new(w::EkrIntegrateMappingPreview {
            source: mapping.source.clone(),
            item: mapping.source_item.clone(),
            mapping: mapping.clone(),
            blockers,
            corrections: proposal.corrections.clone(),
        }));
    }
    let corrections = proposal
        .corrections
        .iter()
        .map(|value| {
            crate::human_review::correction_from_document(value).map_err(|r| error(r.reason))
        })
        .collect::<Result<Vec<_>, _>>()?;
    crate::human_review::corrections_bytes(&corrections).map_err(|r| error(r.reason))?;
    let mut claims = BTreeSet::new();
    for correction in &proposal.corrections {
        let id: AssertionId = correction.assertion_id.0.parse().map_err(error)?;
        if !claims.insert(id) {
            return Err(error("duplicate claim correction"));
        }
        let claim = read
            .graph
            .assertions
            .get(&id)
            .ok_or_else(|| error("correction claim is missing"))?;
        effects.claims.insert(id, claim.clone());
    }
    Ok((preview, effects))
}

/// Shared semantic result and the unchanged review-effect material derived alongside it.
pub(super) struct MappingMaterial {
    pub outcome: crate::application_mapping::MappingOutcome,
    binding: (Option<NodeId>, Option<NodeId>, Option<Value>),
    predicate: Predicate,
    property: Option<ekr_ontology::PropertyDefinition>,
    relation: Option<ekr_ontology::EdgeType>,
}

pub(super) fn mapping_nodes(read: &VerifiedRead) -> BTreeMap<NodeId, TypeId> {
    read.graph
        .nodes
        .iter()
        .map(|(id, node)| (*id, node.type_id))
        .collect()
}

pub(super) fn mapping_material(
    read: &VerifiedRead,
    mapping: &w::EkrIntegrateKnowledgeMapping,
    ontology: &Ontology,
    sources: &BTreeMap<String, Source>,
    nodes: &BTreeMap<NodeId, TypeId>,
    strict: bool,
) -> Result<MappingMaterial, StoreError> {
    let coordinate = coordinate(&mapping.source)?;
    let source = sources
        .get(&coordinate)
        .ok_or_else(|| error("mapping source is not selected"))?;
    if !source.selected.contains(&mapping.source_item) {
        return Err(error("mapping source item is not selected"));
    }
    let fact = source
        .document
        .facts
        .get(fact_index(&mapping.source_item)?)
        .ok_or_else(|| error("mapping fact does not exist"))?;
    let reference = match fact {
        ExtractedFact::Property(fact) => &fact.subject,
        ExtractedFact::Relation(fact) => &fact.subject,
    };
    if mapping.source_type != reference.node_type {
        return Err(error("mapping source type differs from selected fact"));
    }
    let target = super::schema_proposal_schema::type_id(ontology, &mapping.target_type)?;
    if ontology.node_type(target).is_none() {
        return Err(error("mapping target must be a node type"));
    }
    let properties = ontology.properties_of(target);
    let properties: Vec<_> = properties
        .values()
        .filter(|p| p.name == mapping.target_member)
        .collect();
    let relations: Vec<_> = ontology
        .to_document()
        .edge_types
        .into_iter()
        .filter(|r| r.name == mapping.target_member)
        .collect();
    if properties.len() + relations.len() != 1 {
        return Err(error("unknown or ambiguous target member"));
    }
    let mut blockers = Vec::new();
    let subject = resolve(read, target, &reference.aliases, "subject", &mut blockers);
    let mut object = None;
    let mut value = None;
    match mapping.value.as_ref() {
        w::EkrIntegrateMappingValue::V0(constant) => {
            value = Some(super::incubation_value::decode(&constant.value)?);
        }
        w::EkrIntegrateMappingValue::V1(selector) => {
            let ExtractedFact::Property(fact) = fact else {
                return Err(error("CopyField requires a selected property fact"));
            };
            if selector.value.declaration != fact.subject.node_type
                || selector.value.field != fact.property
            {
                return Err(error("CopyField does not select the fact's declared field"));
            }
            value = Some(fact.value.clone());
        }
        w::EkrIntegrateMappingValue::V2(selector) => {
            let ExtractedFact::Relation(fact) = fact else {
                return Err(error("CopyRelation requires a selected relation fact"));
            };
            if selector.value.declaration != fact.subject.node_type
                || selector.value.relation != fact.relation
            {
                return Err(error(
                    "CopyRelation does not select the fact's declared relation",
                ));
            }
            if relations.is_empty() {
                return Err(error("CopyRelation requires a target relation"));
            }
            let object_type =
                super::schema_proposal_schema::type_id(ontology, &fact.object.node_type)?;
            if !relations[0]
                .target_types
                .iter()
                .any(|allowed| ontology.conforms_to(object_type, *allowed))
            {
                return Err(error("relation object type is outside target endpoints"));
            }
            object = resolve(
                read,
                object_type,
                &fact.object.aliases,
                "object",
                &mut blockers,
            );
        }
    }
    let (current_property, current_relation);
    let predicate = if let Some(property) = properties.first() {
        let value = value
            .as_ref()
            .ok_or_else(|| error("property mapping has no typed value"))?;
        if let Err(err) = ontology.check_value(value, &property.value_type, nodes) {
            if strict {
                return Err(error(err));
            }
            blockers.push(error(err).to_string());
        }
        current_property = read
            .graph
            .ontology
            .properties_of(target)
            .get(&property.id)
            .map(|p| (*p).clone());
        current_relation = None;
        Predicate::Property(property.id)
    } else {
        let relation = &relations[0];
        if !relation
            .source_types
            .iter()
            .any(|allowed| ontology.conforms_to(target, *allowed))
        {
            return Err(error("relation subject type is outside target endpoints"));
        }
        if let Some(value) = &value {
            let Value::NodeRef(id) = value else {
                return Err(error(
                    "relation constants and copied values must be NodeRef",
                ));
            };
            ontology
                .check_value(
                    value,
                    &ValueType::NodeRef {
                        allowed_types: relation.target_types.clone(),
                    },
                    nodes,
                )
                .map_err(error)?;
            object = Some(*id);
        }
        current_property = None;
        current_relation = read.graph.ontology.edge_type(relation.id).cloned();
        Predicate::Relation(relation.id)
    };

    let outcome = if blockers.is_empty() {
        let subject = subject.ok_or_else(|| error("mapping has no resolved subject"))?;
        let object = match predicate {
            Predicate::Property(_) => ekr_graph::Object::Value(
                ekr_graph::CanonicalValue::try_from(
                    value
                        .clone()
                        .ok_or_else(|| error("mapping has no typed value"))?,
                )
                .map_err(error)?,
            ),
            Predicate::Relation(_) => ekr_graph::Object::Node(ekr_graph::CanonicalRef::new(
                object.ok_or_else(|| error("mapping has no resolved object"))?,
            )),
        };
        crate::application_mapping::MappingOutcome::Ready(crate::application_mapping::MappedClaim {
            subject: Subject::Node(ekr_graph::CanonicalRef::new(subject)),
            predicate,
            object,
            evidence: fact.evidence().to_vec(),
        })
    } else {
        crate::application_mapping::MappingOutcome::Blocked(blockers)
    };
    Ok(MappingMaterial {
        outcome,
        binding: (subject, object, value),
        predicate,
        property: current_property,
        relation: current_relation,
    })
}
