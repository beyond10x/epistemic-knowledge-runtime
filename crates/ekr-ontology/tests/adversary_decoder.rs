//! Strict decoding distinguishes semantic members from user-owned record field names.

use std::collections::BTreeMap;

use ekr_core::{PropertyId, SchemaVersionId, Timestamp, TypeId};
use ekr_ontology::{
    NodeType, Ontology, OntologyDocument, PropertyDefinition, SchemaVersion, Value, ValueType,
};

#[test]
fn record_keys_named_like_semantic_members_survive_nested_roundtrips() {
    let names = [
        "sets",
        "constraints",
        "value_kind",
        "parameters",
        "value",
        "unit",
    ];
    let fields: BTreeMap<_, _> = names
        .iter()
        .map(|name| {
            (
                (*name).to_owned(),
                ValueType::List(Box::new(ValueType::String)),
            )
        })
        .collect();
    let values: BTreeMap<_, _> = names
        .iter()
        .map(|name| {
            (
                (*name).to_owned(),
                Value::List(vec![Value::String((*name).into())]),
            )
        })
        .collect();
    let property = PropertyId::mint();
    let mut node = NodeType::new(TypeId::mint(), "Decision");
    node.properties.insert(
        property,
        PropertyDefinition::new(property, "record", ValueType::Record(fields)),
    );
    let document = OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: vec![node],
        edge_types: vec![],
    };
    let encoded = serde_yaml_ng::to_string(&document).unwrap();
    assert_eq!(
        Ontology::from_yaml(&encoded).unwrap(),
        Ontology::load(document.clone()).unwrap()
    );
    assert_eq!(
        serde_yaml_ng::from_str::<OntologyDocument>(&encoded).unwrap(),
        document
    );
    let value = Value::Record(values);
    assert_eq!(
        serde_yaml_ng::from_str::<Value>(&serde_yaml_ng::to_string(&value).unwrap()).unwrap(),
        value
    );
    assert_eq!(
        serde_json::from_str::<Value>(&serde_json::to_string(&value).unwrap()).unwrap(),
        value
    );
}

#[test]
fn unknown_value_members_are_refused_beneath_a_record_key_and_list_element() {
    let supported = serde_json::json!({
        "value_kind": "Record", "value": {
            "sets": {"value_kind": "List", "value": [
                {"value_kind": "Integer", "value": 1}
            ]}
        }
    });
    assert!(
        serde_yaml_ng::from_str::<Value>(&serde_yaml_ng::to_string(&supported).unwrap()).is_ok()
    );
    for path in ["/value/sets", "/value/sets/value/0"] {
        let mut input = supported.clone();
        input
            .pointer_mut(path)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unit".into(), serde_json::json!("seconds"));
        let refused = serde_yaml_ng::from_str::<Value>(&serde_yaml_ng::to_string(&input).unwrap())
            .expect_err("nested unsupported unit must not disappear");
        assert!(refused.to_string().contains("unit"), "{path}: {refused}");
        let refused = serde_json::from_value::<Value>(input)
            .expect_err("JSON must enforce the same nested boundary");
        assert!(refused.to_string().contains("unit"), "{path}: {refused}");
    }
}
