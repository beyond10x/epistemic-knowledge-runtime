//! Every field of the kernel's encodings is written where its declaration says it is.
//!
//! **Written by adversary pass 2, against a bound this unit had stated, and adopted unchanged.**
//! The bound was false and the file is the proof: a field's position is observable without fixing
//! any identifier and without a stored byte vector, so the mechanism said to be unavailable is the
//! one that covers the most. Two claims about this table have now been measured and refused — the
//! first by the implementor, that a hash-separation property covers field order; the second by the
//! adversary, that pinning the layout needed a dependency the story forbids. A stated bound is a
//! claim like any other.
//!
//! The sentence it was written against, since the sentence has now been corrected in place:
//!
//! > **What it covers and nothing wider:** the structs listed below, and within each only the
//! > fields that carry a distinct id. A `bool`, a `String` or a count is not located this way, so
//! > `symmetric` and `transitive` could still be swapped with each other and nothing here would
//! > notice. Pinning the whole layout needs a stored byte vector over fixed ids, and an id is only
//! > constructible here by minting […] which the story forbids adding.
//!
//! The second sentence is false, and this file is the counter-example: a field's *position* is
//! observable without fixing any id and without a stored vector. Hold every other field constant,
//! vary one field, and the offset of the first differing byte is where that field is written. The
//! offsets must ascend in declaration order, for **every** field, whatever its type.
//!
//! This covers the two things the id probe cannot: non-id fields, and structs that carry fewer
//! than two ids — `EntityMerge`, whose own encoding doc says "a merge and its reverse are two
//! different operations", and `PropertyDefinition`, five of whose six fields carry no id.

use std::collections::BTreeMap;

use ekr_core::canonical::Canonical;
use ekr_core::{EdgeId, GraphRootId, NodeId, PropertyId, TypeId};
use ekr_graph::CanonicalValue;
use ekr_kernel::{
    EdgeDraft, EntityMerge, GraphOperation, NodeDraft, PropertyModification, PropertyMutation,
};
use ekr_ontology::{Cardinality, EdgeType, NodeType, PropertyDefinition, ValueType};

/// The offset of the first byte at which two encodings differ.
fn first_difference(
    left: &GraphOperation<CanonicalValue>,
    right: &GraphOperation<CanonicalValue>,
) -> usize {
    let (left, right) = (left.canonical_bytes(), right.canonical_bytes());
    left.iter()
        .zip(right.iter())
        .position(|(one, other)| one != other)
        .unwrap_or_else(|| {
            assert_ne!(
                left.len(),
                right.len(),
                "two operations differing in one field encoded to identical bytes"
            );
            left.len().min(right.len())
        })
}

/// Asserts that varying each field in turn moves a byte, and that the bytes move in the order the
/// fields are declared.
fn fields_ascend<F>(what: &str, baseline: &GraphOperation<CanonicalValue>, variants: Vec<(&str, F)>)
where
    F: Fn() -> GraphOperation<CanonicalValue>,
{
    let found: Vec<(&str, usize)> = variants
        .iter()
        .map(|(field, build)| (*field, first_difference(baseline, &build())))
        .collect();
    let offsets: Vec<usize> = found.iter().map(|(_, at)| *at).collect();
    let mut ascending = offsets.clone();
    ascending.sort_unstable();
    assert_eq!(
        offsets, ascending,
        "{what}: the fields are written at {found:?}, which is not declaration order"
    );
}

/// Every field of the kernel's own encodings is written in declaration order, id-bearing or not.
#[test]
fn every_field_of_the_kernels_encodings_is_written_in_declaration_order() {
    let root = GraphRootId::mint();
    let other_root = GraphRootId::mint();
    let (type_id, other_type) = (TypeId::mint(), TypeId::mint());
    let (node, other_node) = (NodeId::mint(), NodeId::mint());
    let (edge, other_edge) = (EdgeId::mint(), EdgeId::mint());
    let (property, other_property) = (PropertyId::mint(), PropertyId::mint());
    let (source, target) = (NodeId::mint(), NodeId::mint());

    let draft = || NodeDraft::<CanonicalValue> {
        id: node,
        root_id: root,
        type_id,
        canonical_name: "one".to_owned(),
        properties: [(property, vec![CanonicalValue::String("one".to_owned())])]
            .into_iter()
            .collect(),
    };
    fields_ascend(
        "NodeDraft: id, root_id, type_id, canonical_name, properties",
        &GraphOperation::CreateNode(draft()),
        vec![
            (
                "id",
                Box::new(move || {
                    let mut it = draft();
                    it.id = other_node;
                    GraphOperation::CreateNode(it)
                }) as Box<dyn Fn() -> GraphOperation<CanonicalValue>>,
            ),
            (
                "root_id",
                Box::new(move || {
                    let mut it = draft();
                    it.root_id = other_root;
                    GraphOperation::CreateNode(it)
                }),
            ),
            (
                "type_id",
                Box::new(move || {
                    let mut it = draft();
                    it.type_id = other_type;
                    GraphOperation::CreateNode(it)
                }),
            ),
            (
                "canonical_name",
                Box::new(move || {
                    let mut it = draft();
                    it.canonical_name = "two".to_owned();
                    GraphOperation::CreateNode(it)
                }),
            ),
            (
                "properties",
                Box::new(move || {
                    let mut it = draft();
                    it.properties = [(property, vec![CanonicalValue::String("two".to_owned())])]
                        .into_iter()
                        .collect();
                    GraphOperation::CreateNode(it)
                }),
            ),
        ],
    );

    let mutation = || PropertyMutation::<CanonicalValue> {
        node,
        property,
        values: vec![CanonicalValue::String("one".to_owned())],
    };
    fields_ascend(
        "PropertyMutation: node, property, values",
        &GraphOperation::UpdateProperty(mutation()),
        vec![
            (
                "node",
                Box::new(move || {
                    let mut it = mutation();
                    it.node = other_node;
                    GraphOperation::UpdateProperty(it)
                }) as Box<dyn Fn() -> GraphOperation<CanonicalValue>>,
            ),
            (
                "property",
                Box::new(move || {
                    let mut it = mutation();
                    it.property = other_property;
                    GraphOperation::UpdateProperty(it)
                }),
            ),
            (
                "values",
                Box::new(move || {
                    let mut it = mutation();
                    it.values = vec![CanonicalValue::String("two".to_owned())];
                    GraphOperation::UpdateProperty(it)
                }),
            ),
        ],
    );

    let edge_draft = || EdgeDraft::<CanonicalValue> {
        id: edge,
        root_id: root,
        type_id,
        source,
        target,
        properties: [(property, vec![CanonicalValue::String("one".to_owned())])]
            .into_iter()
            .collect(),
    };
    fields_ascend(
        "EdgeDraft: id, root_id, type_id, source, target, properties",
        &GraphOperation::CreateEdge(edge_draft()),
        vec![
            (
                "id",
                Box::new(move || {
                    let mut it = edge_draft();
                    it.id = other_edge;
                    GraphOperation::CreateEdge(it)
                }) as Box<dyn Fn() -> GraphOperation<CanonicalValue>>,
            ),
            (
                "root_id",
                Box::new(move || {
                    let mut it = edge_draft();
                    it.root_id = other_root;
                    GraphOperation::CreateEdge(it)
                }),
            ),
            (
                "type_id",
                Box::new(move || {
                    let mut it = edge_draft();
                    it.type_id = other_type;
                    GraphOperation::CreateEdge(it)
                }),
            ),
            (
                "source",
                Box::new(move || {
                    let mut it = edge_draft();
                    it.source = other_node;
                    GraphOperation::CreateEdge(it)
                }),
            ),
            (
                "target",
                Box::new(move || {
                    let mut it = edge_draft();
                    it.target = other_node;
                    GraphOperation::CreateEdge(it)
                }),
            ),
            (
                "properties",
                Box::new(move || {
                    let mut it = edge_draft();
                    it.properties = [(property, vec![CanonicalValue::String("two".to_owned())])]
                        .into_iter()
                        .collect();
                    GraphOperation::CreateEdge(it)
                }),
            ),
        ],
    );

    // `EntityMerge` is in no mechanism's table, and its own encoding doc is the one that says the
    // order carries meaning: "which id survives is the decision the operation records".
    fields_ascend(
        "EntityMerge: absorbed, into",
        &GraphOperation::<CanonicalValue>::MergeEntity(EntityMerge {
            absorbed: node,
            into: source,
        }),
        vec![
            (
                "absorbed",
                Box::new(move || {
                    GraphOperation::<CanonicalValue>::MergeEntity(EntityMerge {
                        absorbed: other_node,
                        into: source,
                    })
                }) as Box<dyn Fn() -> GraphOperation<CanonicalValue>>,
            ),
            (
                "into",
                Box::new(move || {
                    GraphOperation::<CanonicalValue>::MergeEntity(EntityMerge {
                        absorbed: node,
                        into: target,
                    })
                }),
            ),
        ],
    );

    // `PropertyDefinition`: five of its six fields carry no id, so the id probe sees one of them.
    // It reaches the encoding inside a `PropertyModification`, whose owner is written first.
    let definition = || {
        let mut declared = PropertyDefinition::new(property, "one", ValueType::String);
        declared.cardinality = Cardinality::One;
        declared.required = false;
        declared.constraints = vec!["one".to_owned()];
        declared
    };
    let modification = move |property| PropertyModification {
        owner: Some(type_id),
        property,
    };
    fields_ascend(
        "PropertyModification: owner, property",
        &GraphOperation::<CanonicalValue>::ModifyProperty(modification(definition())),
        vec![
            (
                "owner",
                Box::new(move || {
                    GraphOperation::<CanonicalValue>::ModifyProperty(PropertyModification {
                        owner: Some(other_type),
                        property: definition(),
                    })
                }) as Box<dyn Fn() -> GraphOperation<CanonicalValue>>,
            ),
            (
                "property",
                Box::new(move || {
                    let mut it = definition();
                    it.id = other_property;
                    GraphOperation::<CanonicalValue>::ModifyProperty(modification(it))
                }),
            ),
        ],
    );
    fields_ascend(
        "PropertyDefinition: id, name, value_type, cardinality, required, constraints",
        &GraphOperation::<CanonicalValue>::ModifyProperty(modification(definition())),
        vec![
            (
                "id",
                Box::new(move || {
                    let mut it = definition();
                    it.id = other_property;
                    GraphOperation::<CanonicalValue>::ModifyProperty(modification(it))
                }) as Box<dyn Fn() -> GraphOperation<CanonicalValue>>,
            ),
            (
                "name",
                Box::new(move || {
                    let mut it = definition();
                    it.name = "two".to_owned();
                    GraphOperation::<CanonicalValue>::ModifyProperty(modification(it))
                }),
            ),
            (
                "value_type",
                Box::new(move || {
                    let mut it = definition();
                    it.value_type = ValueType::Integer;
                    GraphOperation::<CanonicalValue>::ModifyProperty(modification(it))
                }),
            ),
            (
                "cardinality",
                Box::new(move || {
                    let mut it = definition();
                    it.cardinality = Cardinality::Many;
                    GraphOperation::<CanonicalValue>::ModifyProperty(modification(it))
                }),
            ),
            (
                "required",
                Box::new(move || {
                    let mut it = definition();
                    it.required = true;
                    GraphOperation::<CanonicalValue>::ModifyProperty(modification(it))
                }),
            ),
            (
                "constraints",
                Box::new(move || {
                    let mut it = definition();
                    it.constraints = vec!["two".to_owned()];
                    GraphOperation::<CanonicalValue>::ModifyProperty(modification(it))
                }),
            ),
        ],
    );

    // `EdgeType`: the struct the swap was measured on, and the two `bool`s the stated bound names
    // as unreachable — `symmetric` and `transitive`.
    let edge_type = || {
        let mut declared = EdgeType::new(type_id, "one");
        declared.source_types = [type_id].into_iter().collect();
        declared.target_types = [type_id].into_iter().collect();
        declared.cardinality = Cardinality::One;
        declared.properties = [(property, definition())].into_iter().collect();
        declared.inverse = Some(type_id);
        declared.symmetric = false;
        declared.transitive = false;
        declared
    };
    fields_ascend(
        "EdgeType: id, name, source_types, target_types, cardinality, properties, inverse, \
         symmetric, transitive",
        &GraphOperation::<CanonicalValue>::DefineEdgeType(Box::new(edge_type())),
        vec![
            (
                "id",
                Box::new(move || {
                    let mut it = edge_type();
                    it.id = other_type;
                    GraphOperation::<CanonicalValue>::DefineEdgeType(Box::new(it))
                }) as Box<dyn Fn() -> GraphOperation<CanonicalValue>>,
            ),
            (
                "name",
                Box::new(move || {
                    let mut it = edge_type();
                    it.name = "two".to_owned();
                    GraphOperation::<CanonicalValue>::DefineEdgeType(Box::new(it))
                }),
            ),
            (
                "source_types",
                Box::new(move || {
                    let mut it = edge_type();
                    it.source_types = [other_type].into_iter().collect();
                    GraphOperation::<CanonicalValue>::DefineEdgeType(Box::new(it))
                }),
            ),
            (
                "target_types",
                Box::new(move || {
                    let mut it = edge_type();
                    it.target_types = [other_type].into_iter().collect();
                    GraphOperation::<CanonicalValue>::DefineEdgeType(Box::new(it))
                }),
            ),
            (
                "cardinality",
                Box::new(move || {
                    let mut it = edge_type();
                    it.cardinality = Cardinality::Many;
                    GraphOperation::<CanonicalValue>::DefineEdgeType(Box::new(it))
                }),
            ),
            (
                "properties",
                Box::new(move || {
                    let mut it = edge_type();
                    let mut inner = definition();
                    inner.name = "two".to_owned();
                    it.properties = [(property, inner)].into_iter().collect();
                    GraphOperation::<CanonicalValue>::DefineEdgeType(Box::new(it))
                }),
            ),
            (
                "inverse",
                Box::new(move || {
                    let mut it = edge_type();
                    it.inverse = Some(other_type);
                    GraphOperation::<CanonicalValue>::DefineEdgeType(Box::new(it))
                }),
            ),
            (
                "symmetric",
                Box::new(move || {
                    let mut it = edge_type();
                    it.symmetric = true;
                    GraphOperation::<CanonicalValue>::DefineEdgeType(Box::new(it))
                }),
            ),
            (
                "transitive",
                Box::new(move || {
                    let mut it = edge_type();
                    it.transitive = true;
                    GraphOperation::<CanonicalValue>::DefineEdgeType(Box::new(it))
                }),
            ),
        ],
    );

    // `NodeType`: seven fields, one of which carries an id.
    let node_type = || {
        let mut declared = NodeType::new(type_id, "one");
        declared.parents = [type_id].into_iter().collect();
        declared.properties = [(property, definition())].into_iter().collect();
        declared.abstract_type = false;
        declared.lifecycle = None;
        declared.operations = BTreeMap::new();
        declared
    };
    fields_ascend(
        "NodeType: id, name, parents, properties, abstract_type",
        &GraphOperation::<CanonicalValue>::DefineNodeType(Box::new(node_type())),
        vec![
            (
                "id",
                Box::new(move || {
                    let mut it = node_type();
                    it.id = other_type;
                    GraphOperation::<CanonicalValue>::DefineNodeType(Box::new(it))
                }) as Box<dyn Fn() -> GraphOperation<CanonicalValue>>,
            ),
            (
                "name",
                Box::new(move || {
                    let mut it = node_type();
                    it.name = "two".to_owned();
                    GraphOperation::<CanonicalValue>::DefineNodeType(Box::new(it))
                }),
            ),
            (
                "parents",
                Box::new(move || {
                    let mut it = node_type();
                    it.parents = [other_type].into_iter().collect();
                    GraphOperation::<CanonicalValue>::DefineNodeType(Box::new(it))
                }),
            ),
            (
                "properties",
                Box::new(move || {
                    let mut it = node_type();
                    let mut inner = definition();
                    inner.name = "two".to_owned();
                    it.properties = [(property, inner)].into_iter().collect();
                    GraphOperation::<CanonicalValue>::DefineNodeType(Box::new(it))
                }),
            ),
            (
                "abstract_type",
                Box::new(move || {
                    let mut it = node_type();
                    it.abstract_type = true;
                    GraphOperation::<CanonicalValue>::DefineNodeType(Box::new(it))
                }),
            ),
        ],
    );
}
