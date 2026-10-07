//! Explicit transforms at the native/generated wire differences only.
use super::*;
use ekr_core::canonical::Canonical;

pub(super) fn operation(kind: &str, value: &mut Value, encode: bool) -> Result<(), StoreError> {
    match kind {
        "CreateNode" | "CreateEdge" => {
            for values in object(field(value, "properties")?)?.values_mut() {
                for item in array(values)? {
                    canonical_value(item, encode)?;
                }
            }
        }
        "UpdateProperty" => {
            for item in array(field(value, "values")?)? {
                canonical_value(item, encode)?;
            }
        }
        "Invoke" => {
            for item in object(field(value, "arguments")?)?.values_mut() {
                canonical_value(item, encode)?;
            }
        }
        "DefineNodeType" | "DefineEdgeType" => {
            for property in object(field(value, "properties")?)?.values_mut() {
                value_type(field(property, "value_type")?, encode)?;
            }
            if kind == "DefineNodeType" {
                optional(value, "lifecycle", encode)?;
                for operation in object(field(value, "operations")?)?.values_mut() {
                    optional(operation, "transition", encode)?;
                    for argument in object(field(operation, "arguments")?)?.values_mut() {
                        value_type(argument, encode)?;
                    }
                }
            } else {
                optional(value, "inverse", encode)?;
            }
        }
        "ModifyProperty" => {
            if encode && value.get("owner").is_none() {
                *value = json!({"property":value.take()});
            }
            value_type(field(field(value, "property")?, "value_type")?, encode)?;
            if !encode && value.get("owner").is_none() {
                *value = field(value, "property")?.take();
            }
        }
        "AddAssertion" => assertion(value, encode)?,
        "SupersedeAssertion" => timestamp(field(value, "effective_from")?, encode)?,
        "AddEvidence" => {
            let payload = field(value, "payload")?;
            *payload = if encode {
                Value::String(ekr_core::bytes::encode(&read::<Vec<u8>>(payload.take())?))
            } else {
                write(
                    &ekr_core::bytes::decode(
                        payload.as_str().ok_or_else(|| refused("payload bytes"))?,
                    )
                    .map_err(refused)?,
                )?
            };
            let evidence = field(value, "evidence")?;
            let (from, to) = if encode {
                ("confidence", "confidence_bp")
            } else {
                ("confidence_bp", "confidence")
            };
            let confidence = object(evidence)?
                .remove(from)
                .ok_or_else(|| refused("confidence"))?;
            object(evidence)?.insert(to.into(), confidence);
            timestamp(field(evidence, "observed_at")?, encode)?;
            sum(
                field(evidence, "source")?,
                encode,
                &[
                    ("Url", "url"),
                    ("GraphAssertion", "assertion"),
                    ("Observation", "observation"),
                ],
                &[],
            )?;
            let source = field(evidence, "source")?;
            // Optional native enum fields serialize null; generated absence is explicit.
            if encode {
                optional(source, "section", true)?;
                optional(source, "identity", true)?;
            }
        }
        "DeleteEdge" | "RetractAssertion" | "MergeEntity" | "WidenEdgeType" | "AddAlias"
        | "AttachEvidence" => {}
        _ => return Err(refused("unsupported operation")),
    }
    Ok(())
}

fn optional(value: &mut Value, name: &str, encode: bool) -> Result<(), StoreError> {
    if encode && value.get(name) == Some(&Value::Null) {
        object(value)?.remove(name);
    }
    Ok(())
}

fn timestamp(value: &mut Value, encode: bool) -> Result<(), StoreError> {
    *value = if encode {
        let millis = value
            .as_i64()
            .ok_or_else(|| refused("timestamp milliseconds"))?;
        let instant =
            time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000)
                .map_err(|_| refused("timestamp outside generated calendar range"))?;
        write(&w::EssTimestamp(instant))?
    } else {
        let instant: w::EssTimestamp = read(value.take())?;
        let nanos = instant.0.unix_timestamp_nanos();
        if nanos % 1_000_000 != 0 {
            return Err(refused("submillisecond timestamp"));
        }
        json!(i64::try_from(nanos / 1_000_000).map_err(|_| refused("timestamp overflow"))?)
    };
    Ok(())
}

fn canonical_value(value: &mut Value, encode: bool) -> Result<(), StoreError> {
    *value = if encode {
        let native: CanonicalValue = read(value.take())?;
        let plain: ekr_ontology::Value = native.clone().into();
        let projected: w::EkrGraphTypedValue = read(json!({
            "kind": plain.kind(), "canonical_bytes": ekr_core::bytes::encode(&native.canonical_bytes())
        }))?;
        // Existing decoder bounds nested containers to depth 32. Refuse the same explicit
        // representability boundary on encode, never emit a value this codec cannot read.
        let restored = crate::incubation_value::decode(&projected)
            .map_err(|_| refused("canonical value encoding or nesting limit"))?;
        if restored != plain {
            return Err(refused("canonical value changed"));
        }
        write(&projected)?
    } else {
        let projected: w::EkrGraphTypedValue = read(value.take())?;
        let decoded = crate::incubation_value::decode(&projected)
            .map_err(|_| refused("canonical value encoding or nesting limit"))?;
        let native: CanonicalValue = decoded.try_into().map_err(refused)?;
        write(&native)?
    };
    Ok(())
}

fn value_type(value: &mut Value, encode: bool) -> Result<(), StoreError> {
    let tag = if encode { "value_kind" } else { "kind" };
    let kind = value
        .get(tag)
        .and_then(Value::as_str)
        .ok_or_else(|| refused("value type kind"))?
        .to_owned();
    if encode {
        let mut fields = serde_json::Map::new();
        fields.insert("kind".into(), json!(kind));
        if let Some(mut parameters) = object(value)?.remove("parameters") {
            match kind.as_str() {
                "NodeRef" | "Enum" => fields.extend(object(&mut parameters)?.clone()),
                "List" => {
                    value_type(&mut parameters, true)?;
                    fields.insert("element".into(), parameters);
                }
                "Record" => {
                    for member in object(&mut parameters)?.values_mut() {
                        value_type(member, true)?;
                    }
                    fields.insert("fields".into(), parameters);
                }
                _ => return Err(refused("unexpected type parameters")),
            }
        }
        *value = Value::Object(fields);
    } else {
        object(value)?.remove("kind");
        let parameters = match kind.as_str() {
            "NodeRef" | "Enum" => Some(value.take()),
            "List" => {
                let mut element = field(value, "element")?.take();
                value_type(&mut element, false)?;
                Some(element)
            }
            "Record" => {
                let mut fields = field(value, "fields")?.take();
                for member in object(&mut fields)?.values_mut() {
                    value_type(member, false)?;
                }
                Some(fields)
            }
            _ => None,
        };
        *value = json!({"value_kind":kind});
        if let Some(parameters) = parameters {
            object(value)?.insert("parameters".into(), parameters);
        }
    }
    Ok(())
}

/// Convert an externally tagged native enum to the ESS kind/payload record. Scalar payloads
/// have declared field names; record payloads preserve every field; unit alternatives admit none.
fn sum(
    value: &mut Value,
    encode: bool,
    scalars: &[(&str, &str)],
    units: &[&str],
) -> Result<(), StoreError> {
    if encode {
        let (kind, mut fields) = if let Some(kind) = value.as_str() {
            (kind.to_owned(), serde_json::Map::new())
        } else {
            let map = object(value)?;
            if map.len() != 1 {
                return Err(refused("native enum tag"));
            }
            let (kind, payload) = map
                .iter()
                .next()
                .ok_or_else(|| refused("native enum tag"))?;
            let fields = if let Some((_, name)) = scalars.iter().find(|(tag, _)| *tag == kind) {
                [(name.to_string(), payload.clone())].into_iter().collect()
            } else {
                payload
                    .as_object()
                    .ok_or_else(|| refused("native enum payload"))?
                    .clone()
            };
            (kind.clone(), fields)
        };
        fields.insert("kind".into(), json!(kind));
        *value = Value::Object(fields);
    } else {
        let kind = object(value)?
            .remove("kind")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| refused("enum kind"))?;
        if units.contains(&kind.as_str()) {
            if !object(value)?.is_empty() {
                return Err(refused("unit enum payload"));
            }
            *value = json!(kind);
        } else if let Some((_, name)) = scalars.iter().find(|(tag, _)| *tag == kind) {
            if object(value)?.len() != 1 {
                return Err(refused("scalar enum payload"));
            }
            *value = json!({kind:field(value, name)?.take()});
        } else {
            *value = json!({kind:value.take()});
        }
    }
    Ok(())
}

fn assertion(value: &mut Value, encode: bool) -> Result<(), StoreError> {
    sum(
        field(value, "subject")?,
        encode,
        &[("Node", "id"), ("Edge", "id"), ("Type", "id")],
        &[],
    )?;
    sum(
        field(value, "predicate")?,
        encode,
        &[("Property", "id"), ("Relation", "id")],
        &[],
    )?;
    let item = field(value, "object")?;
    if !encode && item.get("kind").and_then(Value::as_str) == Some("Value") {
        canonical_value(field(item, "value")?, false)?;
    }
    sum(
        item,
        encode,
        &[
            ("Value", "value"),
            ("Node", "reference"),
            ("Type", "reference"),
        ],
        &[],
    )?;
    if encode && item.get("kind").and_then(Value::as_str) == Some("Value") {
        canonical_value(field(item, "value")?, true)?;
    }
    sum(field(value, "assessment")?, encode, &[], &["Proposed"])?;
    let lifecycle = field(value, "lifecycle")?;
    if !encode {
        if let Some(time) = lifecycle.get_mut("effective_from") {
            timestamp(time, false)?;
        }
    }
    sum(lifecycle, encode, &[], &["Active"])?;
    if encode {
        if let Some(time) = lifecycle.get_mut("effective_from") {
            timestamp(time, true)?;
        }
    }
    for (range, names) in [
        ("valid_time", ["from", "to"]),
        ("transaction_time", ["recorded_from", "recorded_to"]),
    ] {
        let range = field(value, range)?;
        for name in names {
            optional(range, name, encode)?;
            if let Some(time) = range.get_mut(name) {
                timestamp(time, encode)?;
            }
        }
    }
    Ok(())
}
