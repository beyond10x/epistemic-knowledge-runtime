//! Frozen additive operations for an elected application; these do not authorize publication.
use crate::{GraphOperation, PropertyModification};
use ekr_core::{PropertyId, TypeId};
use ekr_graph::CanonicalValue;
use ekr_ontology::{Ontology, PropertyDefinition};
use ekr_store::StoreError;
use std::collections::BTreeMap;

fn error(reason: &str) -> StoreError {
    StoreError::Document(format!("application-schema-plan: {reason}"))
}

fn properties(
    owner: TypeId,
    base: &BTreeMap<PropertyId, PropertyDefinition>,
    candidate: &BTreeMap<PropertyId, PropertyDefinition>,
    operations: &mut Vec<GraphOperation<CanonicalValue>>,
) -> Result<(), StoreError> {
    if base
        .iter()
        .any(|(id, value)| candidate.get(id) != Some(value))
    {
        return Err(error("existing property removed or changed"));
    }
    for (id, property) in candidate {
        if !base.contains_key(id) {
            if property.required || !property.constraints.is_empty() {
                return Err(error("new property must be optional and unconstrained"));
            }
            operations.push(GraphOperation::ModifyProperty(PropertyModification {
                owner: Some(owner),
                property: property.clone(),
            }));
        }
    }
    Ok(())
}

/// Extract the exact additive delta from an already checked candidate. This allocates nothing:
/// the elected candidate's identities survive every call and later stale transaction attempts.
pub(crate) fn schema_operations(
    base: &Ontology,
    candidate: &Ontology,
) -> Result<Vec<GraphOperation<CanonicalValue>>, StoreError> {
    if candidate.version() != base.version() {
        return Err(error("candidate changes schema lineage before election"));
    }
    let original = base.to_document();
    let changed = candidate.to_document();
    if original
        .node_types
        .iter()
        .any(|value| candidate.node_type(value.id).is_none())
        || original
            .edge_types
            .iter()
            .any(|value| candidate.edge_type(value.id).is_none())
    {
        return Err(error("existing type or relation removed"));
    }
    let mut operations = Vec::new();
    for node in changed.node_types {
        if let Some(prior) = base.node_type(node.id) {
            let mut unchanged = node.clone();
            unchanged.properties = prior.properties.clone();
            if unchanged != *prior {
                return Err(error("existing type definition changed"));
            }
            properties(
                node.id,
                &prior.properties,
                &node.properties,
                &mut operations,
            )?;
        } else {
            if node
                .properties
                .values()
                .any(|p| p.required || !p.constraints.is_empty())
                || node.lifecycle.is_some()
                || !node.operations.is_empty()
            {
                return Err(error("new type exceeds additive declaration vocabulary"));
            }
            operations.push(GraphOperation::DefineNodeType(Box::new(node)));
        }
    }
    for relation in changed.edge_types {
        if let Some(prior) = base.edge_type(relation.id) {
            let mut unchanged = relation.clone();
            unchanged.properties = prior.properties.clone();
            if unchanged != *prior {
                return Err(error("existing relation definition changed"));
            }
            properties(
                relation.id,
                &prior.properties,
                &relation.properties,
                &mut operations,
            )?;
        } else {
            if relation
                .properties
                .values()
                .any(|p| p.required || !p.constraints.is_empty())
            {
                return Err(error(
                    "new relation exceeds additive declaration vocabulary",
                ));
            }
            operations.push(GraphOperation::DefineEdgeType(Box::new(relation)));
        }
    }
    if operations.is_empty() {
        return Err(error("candidate has no schema additions"));
    }
    Ok(operations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ekr_core::{PropertyId, SchemaVersionId, Timestamp, TypeId};
    use ekr_ontology::{
        EdgeType, NodeType, OntologyDocument, PropertyDefinition, SchemaVersion, ValueType,
    };

    fn base() -> Ontology {
        let node = NodeType::new(TypeId::mint(), "Project");
        let mut relation = EdgeType::new(TypeId::mint(), "DependsOn");
        relation.source_types.insert(node.id);
        relation.target_types.insert(node.id);
        Ontology::load(OntologyDocument {
            version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
            node_types: vec![node],
            edge_types: vec![relation],
        })
        .unwrap()
    }

    #[test]
    fn frozen_schema_plan_preserves_allocations_and_existing_declarations() {
        let base = base();
        let mut document = base.to_document();
        let owner = document.node_types[0].id;
        let relation_owner = document.edge_types[0].id;
        let property = PropertyDefinition::new(PropertyId::mint(), "health", ValueType::String);
        let relation_property =
            PropertyDefinition::new(PropertyId::mint(), "reason", ValueType::String);
        document.node_types[0]
            .properties
            .insert(property.id, property.clone());
        document.edge_types[0]
            .properties
            .insert(relation_property.id, relation_property.clone());
        let new_node = NodeType::new(TypeId::mint(), "HealthObservation");
        let mut new_edge = EdgeType::new(TypeId::mint(), "HasHealthObservation");
        new_edge.source_types.insert(owner);
        new_edge.target_types.insert(new_node.id);
        document.node_types.push(new_node.clone());
        document.edge_types.push(new_edge.clone());
        let candidate = Ontology::load(document).unwrap();
        let operations = schema_operations(&base, &candidate).unwrap();
        assert_eq!(operations.len(), 4);
        assert!(operations.contains(&GraphOperation::DefineNodeType(Box::new(new_node))));
        assert!(operations.contains(&GraphOperation::DefineEdgeType(Box::new(new_edge))));
        assert!(operations.contains(&GraphOperation::ModifyProperty(
            crate::PropertyModification {
                owner: Some(owner),
                property,
            }
        )));
        assert!(operations.contains(&GraphOperation::ModifyProperty(
            crate::PropertyModification {
                owner: Some(relation_owner),
                property: relation_property,
            }
        )));
        assert_eq!(schema_operations(&base, &candidate).unwrap(), operations);
        assert_eq!(base.to_document().node_types[0].properties.len(), 0);
        let changes = operations
            .iter()
            .map(|operation| match operation {
                GraphOperation::DefineNodeType(node) => {
                    ekr_ontology::SchemaChange::DefineNodeType(*node.clone())
                }
                GraphOperation::DefineEdgeType(edge) => {
                    ekr_ontology::SchemaChange::DefineEdgeType(*edge.clone())
                }
                GraphOperation::ModifyProperty(change) => {
                    ekr_ontology::SchemaChange::ModifyProperty {
                        owner: change.owner.unwrap(),
                        property: change.property.clone(),
                    }
                }
                _ => panic!("schema election contains a data operation"),
            })
            .collect::<Vec<_>>();
        let evolved = base
            .evolve(SchemaVersionId::mint(), Timestamp::from_millis(1), &changes)
            .unwrap();
        assert_eq!(
            evolved.to_document().node_types,
            candidate.to_document().node_types
        );
        assert_eq!(
            evolved.to_document().edge_types,
            candidate.to_document().edge_types
        );
    }

    #[test]
    fn application_schema_plan_refuses_destructive_changes_and_empty_effects() {
        let base = base();
        assert!(schema_operations(&base, &base).is_err());
        let mut renamed = base.to_document();
        renamed.node_types[0].name = "RenamedProject".into();
        assert!(schema_operations(&base, &Ontology::load(renamed).unwrap()).is_err());
        let mut removed = base.to_document();
        removed.edge_types.clear();
        assert!(schema_operations(&base, &Ontology::load(removed).unwrap()).is_err());
        let mut required = base.to_document();
        let mut property =
            PropertyDefinition::new(PropertyId::mint(), "requiredHealth", ValueType::String);
        property.required = true;
        required.node_types[0]
            .properties
            .insert(property.id, property);
        assert!(schema_operations(&base, &Ontology::load(required).unwrap()).is_err());
    }

    #[test]
    fn application_schema_plan_cannot_redeclare_an_existing_property() {
        let mut document = base().to_document();
        let property =
            PropertyDefinition::new(PropertyId::mint(), "description", ValueType::String);
        document.node_types[0]
            .properties
            .insert(property.id, property.clone());
        let base = Ontology::load(document).unwrap();
        for remove in [false, true] {
            let mut changed = base.to_document();
            if remove {
                changed.node_types[0].properties.clear();
            } else {
                changed.node_types[0]
                    .properties
                    .get_mut(&property.id)
                    .unwrap()
                    .value_type = ValueType::Integer;
            }
            // An unrelated valid addition cannot hide a destructive edit.
            changed
                .node_types
                .push(NodeType::new(TypeId::mint(), "HealthObservation"));
            assert!(schema_operations(&base, &Ontology::load(changed).unwrap()).is_err());
        }
    }
}
