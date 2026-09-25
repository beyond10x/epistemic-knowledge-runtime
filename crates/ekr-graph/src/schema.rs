//! JSON Schemas written by hand, behind the `schema` feature, for the graph carriers whose
//! decoding the derived schema cannot see: a range checked by `TryFrom`, a reference decoded as
//! its id, and a property whose value list the reader refuses when empty.

use std::borrow::Cow;
use std::marker::PhantomData;

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};

use crate::canonical::{CanonicalRef, CanonicalTarget};
use crate::evidence::Confidence;

/// A node's or an edge's value list for one property, which `crate::node::property_values`
/// refuses when empty ("empty outer property values must be absent").
pub(crate) struct NonEmptyValues<V>(PhantomData<V>);

impl<V: JsonSchema> JsonSchema for NonEmptyValues<V> {
    fn inline_schema() -> bool {
        true
    }

    fn schema_name() -> Cow<'static, str> {
        format!("NonEmptyValues_of_{}", V::schema_name()).into()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "array",
            "items": generator.subschema_for::<V>(),
            "minItems": 1
        })
    }
}

impl JsonSchema for Confidence {
    fn schema_name() -> Cow<'static, str> {
        "Confidence".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "ekr_graph::Confidence".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "integer",
            "minimum": 0,
            "maximum": Confidence::CERTAIN.basis_points(),
            "description": "Basis points, 0 to 10000 (certain)."
        })
    }
}

/// Decoded as the id it holds, so its schema is that id's.
impl<T: CanonicalTarget> JsonSchema for CanonicalRef<T>
where
    T::Id: JsonSchema,
{
    fn inline_schema() -> bool {
        T::Id::inline_schema()
    }

    fn schema_name() -> Cow<'static, str> {
        T::Id::schema_name()
    }

    fn schema_id() -> Cow<'static, str> {
        T::Id::schema_id()
    }

    fn json_schema(generator: &mut SchemaGenerator) -> Schema {
        T::Id::json_schema(generator)
    }
}
