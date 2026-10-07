use ess_conformance::target::{TargetError, ViewRow};
use ess_primitives::{facts::Number, node::Node};
use serde_json::Value;
pub(super) fn fail(operation: &str, error: impl std::fmt::Display) -> TargetError {
    TargetError::unavailable(operation, error.to_string())
}
pub(super) fn value<T: serde::Serialize>(v: T) -> Result<Value, TargetError> {
    serde_json::to_value(v).map_err(|e| fail("retained projection", e))
}
pub(super) fn map(v: Value) -> Result<ViewRow, TargetError> {
    node(v)?
        .as_map()
        .cloned()
        .ok_or_else(|| fail("row", "expected record"))
}
pub(super) fn node(v: Value) -> Result<Node, TargetError> {
    Ok(match v {
        Value::Null => Node::Null,
        Value::Bool(v) => Node::Bool(v),
        Value::String(v) => Node::Text(v),
        Value::Number(v) => Node::Number(Number::from(
            v.as_i64()
                .ok_or_else(|| fail("number", "expected exact i64"))?,
        )),
        Value::Array(v) => Node::Seq(v.into_iter().map(node).collect::<Result<_, _>>()?),
        Value::Object(v) => Node::Map(
            v.into_iter()
                .map(|(k, v)| Ok((k, node(v)?)))
                .collect::<Result<_, TargetError>>()?,
        ),
    })
}
pub(super) fn input(v: &Node) -> Result<Value, TargetError> {
    Ok(match v {
        Node::Null => Value::Null,
        Node::Bool(v) => (*v).into(),
        Node::Text(v) => v.clone().into(),
        Node::Number(v) => v
            .as_i64()
            .ok_or_else(|| fail("input number", "expected exact i64"))?
            .into(),
        Node::Seq(v) => Value::Array(v.iter().map(input).collect::<Result<_, _>>()?),
        Node::Map(v) => Value::Object(
            v.iter()
                .map(|(k, v)| Ok((k.clone(), input(v)?)))
                .collect::<Result<_, TargetError>>()?,
        ),
    })
}
