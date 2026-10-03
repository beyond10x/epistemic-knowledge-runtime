//! Checked transport for frozen application transactions; never admission or validation.
//!
//! IDs, sets and structurally identical records pass through the existing strict native serde
//! readers. The differing sum types, values, timestamps, bytes and legacy property owner shape
//! have explicit transforms. Decode then re-encodes to the *generated typed model*: no optional
//! payload, duplicate set member or noncanonical spelling may disappear. Only absent/empty node
//! aliases and equivalent timestamp offsets normalize. No validated token is constructed here.
use crate::{GraphOperation, GraphTransaction};
use ekr_core::contract_data as w;
use ekr_graph::CanonicalValue;
use ekr_store::StoreError;
use serde::{de::DeserializeOwned, Serialize};
use serde_json::{json, Value};

mod projection;
#[cfg(test)]
mod tests;

fn refused(reason: impl std::fmt::Display) -> StoreError {
    StoreError::Document(format!("application-transaction: {reason}"))
}
fn read<T: DeserializeOwned>(value: Value) -> Result<T, StoreError> {
    serde_json::from_value(value).map_err(refused)
}
fn write<T: Serialize + ?Sized>(value: &T) -> Result<Value, StoreError> {
    serde_json::to_value(value).map_err(refused)
}
fn field<'a>(value: &'a mut Value, name: &str) -> Result<&'a mut Value, StoreError> {
    value
        .get_mut(name)
        .ok_or_else(|| refused(format!("missing {name}")))
}
fn object(value: &mut Value) -> Result<&mut serde_json::Map<String, Value>, StoreError> {
    value
        .as_object_mut()
        .ok_or_else(|| refused("expected record"))
}
fn array(value: &mut Value) -> Result<&mut Vec<Value>, StoreError> {
    value.as_array_mut().ok_or_else(|| refused("expected list"))
}

pub(crate) fn encode(
    transaction: &GraphTransaction<CanonicalValue>,
) -> Result<w::EkrKernelCanonicalTransactionProjection, StoreError> {
    let mut result = write(transaction)?;
    let operations = transaction
        .operations
        .iter()
        .map(|operation| {
            // Exhaustive native match makes new operations an explicit codec decision.
            let kind = match operation {
                GraphOperation::CreateNode(_) => "CreateNode",
                GraphOperation::UpdateProperty(_) => "UpdateProperty",
                GraphOperation::CreateEdge(_) => "CreateEdge",
                GraphOperation::DeleteEdge(_) => "DeleteEdge",
                GraphOperation::AddAssertion(_) => "AddAssertion",
                GraphOperation::RetractAssertion(_) => "RetractAssertion",
                GraphOperation::DefineNodeType(_) => "DefineNodeType",
                GraphOperation::DefineEdgeType(_) => "DefineEdgeType",
                GraphOperation::ModifyProperty(_) => "ModifyProperty",
                GraphOperation::MergeEntity(_) => "MergeEntity",
                GraphOperation::Invoke { .. } => "Invoke",
                GraphOperation::SupersedeAssertion(_) => "SupersedeAssertion",
                GraphOperation::AddEvidence(_) => "AddEvidence",
                GraphOperation::WidenEdgeType(_) => "WidenEdgeType",
                GraphOperation::AddAlias(_) => "AddAlias",
                GraphOperation::AttachEvidence(_) => "AttachEvidence",
            };
            let mut native = write(operation)?;
            let mut value = object(&mut native)?
                .remove(kind)
                .ok_or_else(|| refused("native operation tag"))?;
            projection::operation(kind, &mut value, true)?;
            Ok(json!({"kind":kind,"value":value}))
        })
        .collect::<Result<Vec<_>, StoreError>>()?;
    result["operations"] = Value::Array(operations);
    read(result)
}

pub(crate) fn decode(
    projection: &w::EkrKernelCanonicalTransactionProjection,
) -> Result<GraphTransaction<CanonicalValue>, StoreError> {
    let mut json = write(projection)?;
    for operation in array(field(&mut json, "operations")?)? {
        let kind = operation
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| refused("operation kind"))?
            .to_owned();
        let mut value = field(operation, "value")?.take();
        projection::operation(&kind, &mut value, false)?;
        *operation = json!({kind:value});
    }
    let transaction: GraphTransaction<CanonicalValue> = read(json)?;
    let mut normalized = projection.clone();
    // Empty aliases carry exactly the same ordered sequence as historical absence.
    for operation in &mut normalized.operations {
        if let w::EkrKernelCanonicalOperationProjection::V5(node) = &mut **operation {
            if matches!(&node.value.aliases, w::EssPresence::Present(aliases) if aliases.is_empty())
            {
                node.value.aliases = w::EssPresence::Absent;
            }
        }
    }
    if encode(&transaction)? != normalized {
        return Err(refused("projection loses or changes data"));
    }
    Ok(transaction)
}
