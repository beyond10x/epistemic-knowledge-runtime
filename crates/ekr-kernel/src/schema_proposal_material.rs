//! Relevant schema dependencies in a deterministic review representation.
use super::schema_proposals::error;
use ekr_core::{contract_data as w, TypeId};
use ekr_ontology::{Ontology, ValueType};
use ekr_store::StoreError;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

fn references(value: &ValueType, pending: &mut Vec<TypeId>) {
    match value {
        ValueType::NodeRef { allowed_types } => pending.extend(allowed_types),
        ValueType::List(value) => references(value, pending),
        ValueType::Record(fields) => fields.values().for_each(|value| references(value, pending)),
        _ => {}
    }
}
// All arrays in ontology declarations are sets except lifecycle operation arguments. Preserve
// array order generally; canonicalise only set-valued identity lists, whose minted IDs vary.
fn normalized(value: Value, ids: &BTreeMap<String, String>) -> Value {
    match value {
        Value::String(value) => Value::String(ids.get(&value).cloned().unwrap_or(value)),
        Value::Array(values) => {
            Value::Array(values.into_iter().map(|v| normalized(v, ids)).collect())
        }
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| {
                    let mut value = normalized(value, ids);
                    if matches!(
                        key.as_str(),
                        "parents" | "source_types" | "target_types" | "allowed_types"
                    ) {
                        if let Value::Array(values) = &mut value {
                            values.sort_by_cached_key(Value::to_string);
                        }
                    }
                    (ids.get(&key).cloned().unwrap_or(key), value)
                })
                .collect(),
        ),
        value => value,
    }
}

pub(super) fn relevant(
    current: &Ontology,
    candidate: &Ontology,
    proposal: &w::EkrIntegrateSchemaProposalDocument,
) -> Result<BTreeMap<String, Value>, StoreError> {
    let document = candidate.to_document();
    let current_doc = current.to_document();
    let existing: BTreeSet<_> = current_doc
        .node_types
        .iter()
        .map(|v| v.id)
        .chain(current_doc.edge_types.iter().map(|v| v.id))
        .collect();
    let existing_properties: BTreeSet<_> = current_doc
        .node_types
        .iter()
        .flat_map(|v| v.properties.keys())
        .chain(
            current_doc
                .edge_types
                .iter()
                .flat_map(|v| v.properties.keys()),
        )
        .copied()
        .collect();
    let mut ids = BTreeMap::new();
    for (id, name, properties) in document
        .node_types
        .iter()
        .map(|v| (v.id, &v.name, &v.properties))
        .chain(
            document
                .edge_types
                .iter()
                .map(|v| (v.id, &v.name, &v.properties)),
        )
    {
        if !existing.contains(&id) {
            ids.insert(id.to_string(), format!("proposed-type:{name}"));
        }
        for property in properties.values() {
            if !existing_properties.contains(&property.id) {
                ids.insert(
                    property.id.to_string(),
                    format!("proposed-property:{name}:{}", property.name),
                );
            }
        }
    }
    let mut names = BTreeSet::new();
    for operation in &proposal.additions {
        names.insert(match operation.as_ref() {
            w::EkrIntegrateAdditiveSchemaOperation::V0(v) => v.value.owner_type.as_str(),
            w::EkrIntegrateAdditiveSchemaOperation::V1(v) => v.value.name.as_str(),
            w::EkrIntegrateAdditiveSchemaOperation::V2(v) => v.value.name.as_str(),
        });
    }
    for mapping in &proposal.mappings {
        names.insert(&mapping.target_type);
        names.insert(&mapping.target_member);
    }
    let mut pending: Vec<_> = document
        .node_types
        .iter()
        .filter(|v| names.contains(v.name.as_str()))
        .map(|v| v.id)
        .chain(
            document
                .edge_types
                .iter()
                .filter(|v| names.contains(v.name.as_str()))
                .map(|v| v.id),
        )
        .collect();
    let mut seen = BTreeSet::new();
    let mut result = BTreeMap::new();
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            continue;
        }
        let (name, value) = if let Some(node) = candidate.node_type(id) {
            pending.extend(&node.parents);
            for property in node.properties.values() {
                references(&property.value_type, &mut pending);
            }
            (&node.name, serde_json::to_value(node).map_err(error)?)
        } else if let Some(edge) = candidate.edge_type(id) {
            pending.extend(&edge.source_types);
            pending.extend(&edge.target_types);
            pending.extend(edge.inverse);
            for property in edge.properties.values() {
                references(&property.value_type, &mut pending);
            }
            (&edge.name, serde_json::to_value(edge).map_err(error)?)
        } else {
            return Err(error("missing schema dependency"));
        };
        result.insert(
            format!(
                "{name}:{}",
                ids.get(&id.to_string())
                    .cloned()
                    .unwrap_or_else(|| id.to_string())
            ),
            normalized(value, &ids),
        );
    }
    Ok(result)
}
