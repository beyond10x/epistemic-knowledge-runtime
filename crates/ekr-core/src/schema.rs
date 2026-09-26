//! JSON Schemas of the scalars whose `Deserialize` is written by hand, behind the `schema`
//! feature: each accepts the one text form the type's `FromStr` reads and nothing else.

use std::borrow::Cow;

use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};

use crate::ContentHash;

/// The one text form of every id: lowercase hyphenated, as `is_canonical_uuid_text` checks.
const UUID: &str = "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$";

/// The one text form of a content hash: 64 lowercase hex characters.
const HEX_64: &str = "^[0-9a-f]{64}$";

/// The schema every id newtype shares.
pub(crate) fn id() -> Schema {
    json_schema!({
        "type": "string",
        "pattern": UUID,
        "description": "A UUID in lowercase hyphenated form (`ekr mint <kind>` prints a new one)."
    })
}

impl JsonSchema for ContentHash {
    fn schema_name() -> Cow<'static, str> {
        "ContentHash".into()
    }

    fn schema_id() -> Cow<'static, str> {
        "ekr_core::ContentHash".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "pattern": HEX_64,
            "description": "A content hash: 64 lowercase hex characters (`ekr hash <file>`)."
        })
    }
}
