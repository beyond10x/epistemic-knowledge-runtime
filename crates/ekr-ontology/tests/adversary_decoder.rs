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

#[test]
fn every_nested_ontology_set_and_map_refuses_duplicate_decoded_members() {
    use ekr_ontology::{EdgeType, Lifecycle, OperationDefinition};
    use serde::de::DeserializeOwned;
    fn check<T: DeserializeOwned>(
        name: &str,
        valid: &str,
        duplicate: &str,
        accepted: &mut Vec<String>,
    ) {
        assert!(
            serde_json::from_str::<T>(valid).is_ok(),
            "{name} JSON control"
        );
        assert!(
            serde_yaml_ng::from_str::<T>(valid).is_ok(),
            "{name} YAML control"
        );
        if serde_json::from_str::<T>(duplicate).is_ok() {
            accepted.push(format!("{name}/json"));
        }
        if serde_yaml_ng::from_str::<T>(duplicate).is_ok() {
            accepted.push(format!("{name}/yaml"));
        }
    }
    let mut accepted = Vec::new();
    let id = "aaaaaaaa-aaaa-7aaa-8aaa-aaaaaaaaaaaa";
    let property =
        format!(r#"{{"id":"{id}","name":"value","value_type":{{"value_kind":"String"}}}}"#);
    let string = r#"{"value_kind":"String"}"#;
    {
        let field = "parents";
        let one = format!(r#"{{"id":"{id}","name":"Node","{field}":["{id}"]}}"#);
        check::<NodeType>(
            field,
            &one,
            &one.replace(&format!("[\"{id}\"]"), &format!("[\"{id}\",\"{id}\"]")),
            &mut accepted,
        );
    }
    for field in ["source_types", "target_types"] {
        let one = format!(r#"{{"id":"{id}","name":"Edge","{field}":["{id}"]}}"#);
        check::<EdgeType>(
            field,
            &one,
            &one.replace(&format!("[\"{id}\"]"), &format!("[\"{id}\",\"{id}\"]")),
            &mut accepted,
        );
    }
    for (name, kind, field, value) in [
        (
            "reference types",
            "NodeRef",
            "allowed_types",
            format!("\"{id}\""),
        ),
        ("enum variants", "Enum", "variants", "\"a\"".into()),
    ] {
        let one = format!(r#"{{"value_kind":"{kind}","parameters":{{"{field}":[{value}]}}}}"#);
        check::<ValueType>(
            name,
            &one,
            &one.replace(&format!("[{value}]"), &format!("[{value},{value}]")),
            &mut accepted,
        );
    }
    check::<Lifecycle>(
        "states",
        r#"{"initial":"a","states":["a"]}"#,
        r#"{"initial":"a","states":["a","a"]}"#,
        &mut accepted,
    );
    check::<Lifecycle>(
        "transitions",
        r#"{"initial":"a","states":["a"],"transitions":[{"from":"a","to":"a"}]}"#,
        r#"{"initial":"a","states":["a"],"transitions":[{"from":"a","to":"a"},{"from":"a","to":"a"}]}"#,
        &mut accepted,
    );
    let operation = r#"{"name":"keep"}"#;
    let one = format!(r#"{{"id":"{id}","name":"Node","operations":{{"keep":{operation}}}}}"#);
    check::<NodeType>(
        "operations",
        &one,
        &one.replace(
            &format!("\"keep\":{operation}"),
            &format!("\"keep\":{operation},\"keep\":{operation}"),
        ),
        &mut accepted,
    );
    let one = format!(r#"{{"name":"keep","arguments":{{"value":{string}}}}}"#);
    check::<OperationDefinition>(
        "arguments",
        &one,
        &one.replace(
            &format!("\"value\":{string}"),
            &format!("\"value\":{string},\"value\":{string}"),
        ),
        &mut accepted,
    );
    let one = format!(r#"{{"value_kind":"Record","parameters":{{"sets":{string}}}}}"#);
    check::<ValueType>(
        "record type",
        &one,
        &one.replace(
            &format!("\"sets\":{string}"),
            &format!("\"sets\":{string},\"sets\":{string}"),
        ),
        &mut accepted,
    );
    let value = r#"{"value_kind":"Integer","value":1}"#;
    let one = format!(r#"{{"value_kind":"Record","value":{{"sets":{value}}}}}"#);
    check::<Value>(
        "record value",
        &one,
        &one.replace(
            &format!("\"sets\":{value}"),
            &format!("\"sets\":{value},\"sets\":{value}"),
        ),
        &mut accepted,
    );
    let one = format!(r#"{{"id":"{id}","name":"Type","properties":{{"{id}":{property}}}}}"#);
    let two = one.replace(
        &format!("\"{id}\":{property}"),
        &format!("\"{id}\":{property},\"{id}\":{property}"),
    );
    check::<NodeType>("node properties", &one, &two, &mut accepted);
    check::<EdgeType>("edge properties", &one, &two, &mut accepted);
    assert!(
        accepted.is_empty(),
        "duplicate members were discarded: {accepted:?}"
    );
}
