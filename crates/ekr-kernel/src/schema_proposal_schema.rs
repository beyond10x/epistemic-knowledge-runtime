//! Additive candidate schemas used only for validation and mapping preview.
use super::schema_proposals::error;
use ekr_core::contract_data as w;
use ekr_core::{PropertyId, TypeId};
use ekr_integrate::extraction::ValueSpec;
use ekr_ontology::{Cardinality, EdgeType, NodeType, Ontology, PropertyDefinition, ValueType};
use ekr_store::StoreError;
use std::collections::{BTreeMap, BTreeSet};

fn names(ontology: &Ontology) -> BTreeMap<String, Vec<TypeId>> {
    let document = ontology.to_document();
    let mut result = BTreeMap::<String, Vec<TypeId>>::new();
    for (name, id) in document
        .node_types
        .iter()
        .map(|t| (&t.name, t.id))
        .chain(document.edge_types.iter().map(|t| (&t.name, t.id)))
    {
        result.entry(name.clone()).or_default().push(id);
    }
    result
}

pub(super) fn type_id(ontology: &Ontology, name: &str) -> Result<TypeId, StoreError> {
    lookup(&names(ontology), name)
}
fn lookup(names: &BTreeMap<String, Vec<TypeId>>, name: &str) -> Result<TypeId, StoreError> {
    match names.get(name).map(Vec::as_slice) {
        Some([id]) => Ok(*id),
        _ => Err(error(format!("unknown or ambiguous schema type {name:?}"))),
    }
}

fn ids(
    values: &[String],
    names: &BTreeMap<String, Vec<TypeId>>,
) -> Result<BTreeSet<TypeId>, StoreError> {
    let result: BTreeSet<_> = values
        .iter()
        .map(|name| lookup(names, name))
        .collect::<Result<_, _>>()?;
    if result.len() != values.len() {
        return Err(error("duplicate type reference"));
    }
    Ok(result)
}

pub(super) fn enum_sets(value: &ValueSpec, out: &mut Vec<BTreeSet<String>>) {
    match value {
        ValueSpec::Enum { variants } => out.push(variants.iter().cloned().collect()),
        ValueSpec::List(element) => enum_sets(element, out),
        ValueSpec::Record(fields) => fields.values().for_each(|value| enum_sets(value, out)),
        _ => {}
    }
}

fn value_type(
    value: &ValueSpec,
    names: &BTreeMap<String, Vec<TypeId>>,
    supported: &[BTreeSet<String>],
) -> Result<ValueType, StoreError> {
    Ok(match value {
        ValueSpec::Boolean => ValueType::Boolean,
        ValueSpec::Decimal => ValueType::Decimal,
        ValueSpec::Duration => ValueType::Duration,
        ValueSpec::Float => ValueType::Float,
        ValueSpec::Integer => ValueType::Integer,
        ValueSpec::String => ValueType::String,
        ValueSpec::Timestamp => ValueType::Timestamp,
        ValueSpec::NodeRef { allowed_types } => ValueType::NodeRef {
            allowed_types: ids(allowed_types, names)?,
        },
        ValueSpec::Enum { variants } => {
            let set: BTreeSet<_> = variants.iter().cloned().collect();
            if set.len() != variants.len() {
                return Err(error("duplicate enum variant"));
            }
            let variants = set;
            if variants.is_empty() || !supported.contains(&variants) {
                return Err(error(
                    "enum variants require an exact retained local declaration",
                ));
            }
            ValueType::Enum { variants }
        }
        ValueSpec::List(value) => ValueType::List(Box::new(value_type(value, names, supported)?)),
        ValueSpec::Record(fields) => ValueType::Record(
            fields
                .iter()
                .map(|(name, value)| Ok((name.clone(), value_type(value, names, supported)?)))
                .collect::<Result<_, StoreError>>()?,
        ),
    })
}

fn cardinality(value: &w::EkrOntologyCardinality) -> Cardinality {
    match value {
        w::EkrOntologyCardinality::V0 => Cardinality::Many,
        w::EkrOntologyCardinality::V1 => Cardinality::One,
    }
}
fn property(
    value: &w::EkrIntegratePropertySpec,
    names: &BTreeMap<String, Vec<TypeId>>,
    supported: &[BTreeSet<String>],
) -> Result<PropertyDefinition, StoreError> {
    if value.required {
        return Err(error("required property additions are not permitted"));
    }
    let value_type = value_type(
        &super::incubation_document::value_spec(&value.value)?,
        names,
        supported,
    )?;
    let mut property = PropertyDefinition::new(PropertyId::mint(), &value.name, value_type);
    property.cardinality = cardinality(&value.cardinality);
    Ok(property)
}
fn properties(
    values: &[Box<w::EkrIntegratePropertySpec>],
    names: &BTreeMap<String, Vec<TypeId>>,
    supported: &[BTreeSet<String>],
) -> Result<BTreeMap<PropertyId, PropertyDefinition>, StoreError> {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .map(|value| {
            if !seen.insert(&value.name) {
                return Err(error("duplicate property declaration"));
            }
            let property = property(value, names, supported)?;
            Ok((property.id, property))
        })
        .collect()
}

/// Temporary IDs never leave this candidate ontology or enter review basis material.
pub(super) fn candidate(
    base: &Ontology,
    proposal: &w::EkrIntegrateSchemaProposalDocument,
    supported: &[BTreeSet<String>],
) -> Result<Ontology, StoreError> {
    if proposal.additions.is_empty() {
        return Err(error("proposal has no schema additions"));
    }
    let mut names = names(base);
    for operation in &proposal.additions {
        let name = match operation.as_ref() {
            w::EkrIntegrateAdditiveSchemaOperation::V0(_) => continue,
            w::EkrIntegrateAdditiveSchemaOperation::V1(value) => &value.value.name,
            w::EkrIntegrateAdditiveSchemaOperation::V2(value) => &value.value.name,
        };
        if names.contains_key(name) {
            return Err(error("type or relation already declared"));
        }
        names.insert(name.clone(), vec![TypeId::mint()]);
    }
    let mut document = base.to_document();
    for operation in &proposal.additions {
        match operation.as_ref() {
            w::EkrIntegrateAdditiveSchemaOperation::V0(_) => {}
            w::EkrIntegrateAdditiveSchemaOperation::V1(value) => {
                let value = &value.value;
                let mut edge = EdgeType::new(lookup(&names, &value.name)?, &value.name);
                edge.source_types = ids(&value.source_types, &names)?;
                edge.target_types = ids(&value.target_types, &names)?;
                edge.cardinality = cardinality(&value.cardinality);
                edge.properties = properties(&value.properties, &names, supported)?;
                document.edge_types.push(edge);
            }
            w::EkrIntegrateAdditiveSchemaOperation::V2(value) => {
                let value = &value.value;
                let mut node = NodeType::new(lookup(&names, &value.name)?, &value.name);
                node.parents = ids(&value.parents, &names)?;
                node.abstract_type = value.abstract_type;
                node.properties = properties(&value.properties, &names, supported)?;
                document.node_types.push(node);
            }
        }
    }
    let mut candidate = Ontology::load(document).map_err(error)?;
    for operation in &proposal.additions {
        let w::EkrIntegrateAdditiveSchemaOperation::V0(value) = operation.as_ref() else {
            continue;
        };
        let owner = lookup(&names, &value.value.owner_type)?;
        let property = property(&value.value.property, &names, supported)?;
        let present = if let Some(edge) = candidate.edge_type(owner) {
            edge.properties.values().any(|p| p.name == property.name)
        } else {
            candidate
                .properties_of(owner)
                .values()
                .any(|p| p.name == property.name)
        };
        if present {
            return Err(error(
                "property already declared, including inherited properties",
            ));
        }
        let mut document = candidate.to_document();
        let properties = if let Some(node) = document.node_types.iter_mut().find(|n| n.id == owner)
        {
            &mut node.properties
        } else {
            &mut document
                .edge_types
                .iter_mut()
                .find(|n| n.id == owner)
                .ok_or_else(|| error("unknown property owner"))?
                .properties
        };
        properties.insert(property.id, property);
        candidate = Ontology::load(document).map_err(error)?;
    }
    // The eventual elected transaction uses these exact declarations and identities. Checking
    // its delta here also keeps preview admission within the same additive operation boundary.
    super::application_plan::schema_operations(base, &candidate)?;
    Ok(candidate)
}
