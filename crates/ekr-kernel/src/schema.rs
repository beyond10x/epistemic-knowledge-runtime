//! JSON Schemas (draft 2020-12) of the two YAML documents the kernel reads, behind the `schema`
//! feature: `ekr.transaction-document/1` ([`TransactionDocument`](crate::TransactionDocument))
//! and `ekr-seed/2` ([`SeedDocument`]).
//!
//! Each is generated from the types those readers deserialize. Where a type decodes by hand, its
//! schema is written beside that decoder: the id and hash scalars in `ekr-core`, a confidence, a
//! reference and a non-empty property value list in `ekr-graph`, and here the graph document a
//! seed nests, whose envelope `ekr-store` decodes.
//!
//! The generated schema is then brought to what the YAML readers accept, by transforms each
//! named for the reader behaviour it describes: [`yaml_unit_forms`], [`text_from_any_scalar`],
//! [`integer_ranges`] and, for the transaction document's frozen profile, [`v1_limits`]. The
//! printed schema is for agents, so [`agent_facing`] drops the maintainers' rustdoc the derive
//! copies in, and [`describe`] writes the few descriptions an agent reads.

use std::collections::BTreeMap;

use ekr_core::{AssertionId, EdgeId, EvidenceId, NodeId, RevisionNumber};
use ekr_graph::{Assertion, Edge, Evidence, GraphRoot, Node};
use ekr_ontology::Value;
use schemars::generate::SchemaSettings;
use schemars::transform::RecursiveTransform;
use schemars::{json_schema, Schema, SchemaGenerator};
use serde_json::{json, Value as Json};

use crate::document::{Envelope, DOCUMENT_V1_LIMITS};
use crate::SeedDocument;

/// What every YAML format's schema says about what JSON Schema cannot express. Each gap named
/// here is one `crates/ekr/tests/schema_cli.rs` or `adversary_p1_15_schema.rs` shows is real.
const YAML_TAGS: &str = "The document is YAML; this schema validates its JSON projection, \
the document read as plain YAML and written as JSON. A YAML tag `!Kind value` (an operation such \
as `!AddAssertion`, a subject such as `!Node <id>`, an evidence source such as `!HumanStatement`) \
has no JSON Schema form, so the projection and this schema spell it as the one-key object \
{\"!Kind\": value}. The readers do not accept that object in place of the tag: write the tag. \
Where the schema and the readers differ, the readers accept and this schema refuses: a tag on a \
scalar, a mapping or a list that names no variant (`!AgentId <id>`, `!Range {from, to}`), which \
the readers ignore; a Float written `.nan` or `.inf`, which the projection writes as null; an id \
or content hash written only in digits and unquoted, which YAML reads as a number (quote it); and \
two texts YAML resolves to one value in a list the schema marks uniqueItems (`[true, True]`, \
`[1, 1.0]`), which the readers hold distinct. The readers refuse, and this schema cannot see: a \
mapping key written twice; a list the schema marks uniqueItems holding one value twice, including \
the same text written once plain and once quoted; a whole number written with a fraction (`1.0`); a time \
range or transaction time that ends before it starts; and a value's `value` written before its \
`value_kind` (or a value type's `parameters` before its `value_kind`) when it is a plain number, \
boolean or null for a kind read as text.";

/// The frozen `ekr.transaction-document/1` limits the schema cannot express, named on its root.
const V1_LIMITS_UNSEEN: &str = "The transaction reader also refuses a document over 262144 \
bytes, nested deeper than 32 containers, with more than 32768 values and keys, or with more than \
1048576 bytes of text in all, and a string or key within maxLength characters but over the \
byte limit (the limits count bytes, maxLength counts characters: text outside ASCII); the schema \
carries the per-list, per-map and per-string limits.";

/// The draft 2020-12 generator of the two YAML formats; `limits` adds [`v1_limits`].
fn generator(limits: bool) -> SchemaGenerator {
    let mut settings = SchemaSettings::draft2020_12()
        .with_transform(RecursiveTransform(agent_facing))
        .with_transform(RecursiveTransform(yaml_unit_forms))
        .with_transform(RecursiveTransform(text_from_any_scalar))
        .with_transform(RecursiveTransform(integer_ranges));
    if limits {
        settings = settings.with_transform(RecursiveTransform(v1_limits));
    }
    settings.into_generator()
}

/// Drops every description the derive copied from rustdoc: those are written for maintainers
/// (paths, decision records, design sections). [`describe`] writes the agent-facing ones after.
fn agent_facing(schema: &mut Schema) {
    schema.remove("description");
}

/// The JSON types of a schema, whether `type` is one name or a list.
fn types(schema: &Schema) -> Vec<String> {
    match schema.get("type") {
        Some(Json::String(name)) => vec![name.clone()],
        Some(Json::Array(names)) => names
            .iter()
            .filter_map(Json::as_str)
            .map(str::to_owned)
            .collect(),
        _ => Vec::new(),
    }
}

/// Two spellings of a unit variant the YAML reader accepts and the derived schema does not:
///
/// * an enum's unit variant, derived as the branch `{"type": "string", "const": N}`, may also be
///   written as a tag with no value (`!Proposed`, `!Active null`), projected as `{"!N": null}`;
/// * an adjacently tagged unit variant (`ValueType`'s `{value_kind: String}`), derived as a closed
///   object holding only its tag, may also carry its content key as `parameters: null`.
///
/// Every such branch here comes from a derived enum: a `const` a field declares is not a branch
/// of a `oneOf`, and `ValueType` is the one adjacently tagged enum with unit variants.
fn yaml_unit_forms(schema: &mut Schema) {
    let Some(branches) = schema.get_mut("oneOf").and_then(Json::as_array_mut) else {
        return;
    };
    let mut tagged = Vec::new();
    for branch in branches.iter_mut() {
        if branch.get("type").and_then(Json::as_str) == Some("string") {
            if let Some(name) = branch.get("const").and_then(Json::as_str) {
                tagged.push(json!({
                    "type": "object",
                    "properties": { format!("!{name}"): { "type": "null" } },
                    "required": [format!("!{name}")],
                    "additionalProperties": false,
                }));
            }
        }
        let only_the_tag = branch
            .get("properties")
            .and_then(Json::as_object)
            .is_some_and(|properties| {
                properties.len() == 1
                    && properties
                        .get("value_kind")
                        .is_some_and(|kind| kind.get("const").is_some())
            });
        if only_the_tag && branch.get("additionalProperties") == Some(&false.into()) {
            if let Some(properties) = branch.get_mut("properties").and_then(Json::as_object_mut) {
                properties.insert("parameters".to_owned(), json!({"type": "null"}));
            }
        }
    }
    branches.extend(tagged);
}

/// A field of Rust type `String` is read from a plain YAML scalar as its text, whatever the
/// scalar resolves to: `12.50`, `true`, `42` and `null` are all text to it, and the projection
/// writes them as a number, a boolean or null. A free text schema (no `const`, `enum`, `pattern`
/// or `format`) therefore admits those JSON types too. Ids, hashes and constants keep their one
/// spelling: the reader parses their text and a number or boolean is never one of them.
fn text_from_any_scalar(schema: &mut Schema) {
    let kinds = types(schema);
    if !kinds.iter().any(|kind| kind == "string")
        || ["const", "enum", "pattern", "format"]
            .iter()
            .any(|keyword| schema.get(*keyword).is_some())
    {
        return;
    }
    let mut widened = kinds;
    for kind in ["number", "boolean", "null"] {
        if !widened.iter().any(|known| known == kind) {
            widened.push(kind.to_owned());
        }
    }
    schema.insert("type".to_owned(), json!(widened));
}

/// The range of each whole-number format the derive names. Draft 2020-12 treats `format` as an
/// annotation, so without `minimum` and `maximum` a number past `i64` passes a schema its reader
/// refuses.
fn integer_ranges(schema: &mut Schema) {
    if !types(schema).iter().any(|kind| kind == "integer") {
        return;
    }
    let (minimum, maximum): (Json, Json) = match schema.get("format").and_then(Json::as_str) {
        Some("int64") => (i64::MIN.into(), i64::MAX.into()),
        Some("uint64") => (0.into(), u64::MAX.into()),
        Some("int32") => (i32::MIN.into(), i32::MAX.into()),
        Some("uint32") => (0.into(), u32::MAX.into()),
        Some("int16") => (i16::MIN.into(), i16::MAX.into()),
        Some("uint16") => (0.into(), u16::MAX.into()),
        Some("uint8") => (0.into(), u8::MAX.into()),
        _ => return,
    };
    if schema.get("minimum").is_none() {
        schema.insert("minimum".to_owned(), minimum);
    }
    if schema.get("maximum").is_none() {
        schema.insert("maximum".to_owned(), maximum);
    }
}

/// [`DOCUMENT_V1_LIMITS`] where one list, map or string carries it: every list at most
/// `sequence_elements` long, every map at most `mapping_entries` entries with keys of at most
/// `key_bytes`, every text at most `string_bytes`. `operations` and `evidence` get their own
/// bounds in [`transaction_document`].
fn v1_limits(schema: &mut Schema) {
    let kinds = types(schema);
    if kinds.iter().any(|kind| kind == "string") && schema.get("maxLength").is_none() {
        schema.insert(
            "maxLength".to_owned(),
            DOCUMENT_V1_LIMITS.string_bytes.into(),
        );
    }
    if kinds.iter().any(|kind| kind == "array") && schema.get("maxItems").is_none() {
        schema.insert(
            "maxItems".to_owned(),
            DOCUMENT_V1_LIMITS.sequence_elements.into(),
        );
    }
    let map = schema.get("patternProperties").is_some()
        || schema
            .get("additionalProperties")
            .is_some_and(Json::is_object);
    if kinds.iter().any(|kind| kind == "object") && map {
        schema.insert(
            "maxProperties".to_owned(),
            DOCUMENT_V1_LIMITS.mapping_entries.into(),
        );
        schema.insert(
            "propertyNames".to_owned(),
            json!({ "maxLength": DOCUMENT_V1_LIMITS.key_bytes }),
        );
    }
}

/// The agent-facing descriptions: the root, each top-level field and each definition an agent
/// meets by name. Scalars are described once, where they are defined.
fn describe(mut schema: Schema, title: &str, root: &str, fields: &[(&str, &str)]) -> Schema {
    schema.insert("title".to_owned(), title.into());
    schema.insert("description".to_owned(), root.into());
    if let Some(properties) = schema.get_mut("properties").and_then(Json::as_object_mut) {
        for (field, description) in fields {
            if let Some(Json::Object(property)) = properties.get_mut(*field) {
                property.insert("description".to_owned(), (*description).into());
            }
        }
    }
    if let Some(definitions) = schema.get_mut("$defs").and_then(Json::as_object_mut) {
        for (name, definition) in definitions.iter_mut() {
            let Some(definition) = definition.as_object_mut() else {
                continue;
            };
            if let Some(description) = scalar_description(name) {
                definition.insert("description".to_owned(), description.into());
            }
        }
    }
    schema
}

/// What an agent needs to know about a scalar it writes, by its definition name.
fn scalar_description(name: &str) -> Option<&'static str> {
    Some(match name {
        "ContentHash" => {
            "A content hash: 64 lowercase hex characters. `ekr hash <file>` prints the one a \
             payload has."
        }
        "Timestamp" => "Milliseconds since the Unix epoch, UTC.",
        "RevisionNumber" => "A revision number: 0 for the seed, one more per commit (`ekr head`).",
        "Confidence" => "Confidence in basis points, 0 to 10000 (certain).",
        "NodeId" | "EdgeId" | "AssertionId" | "TransactionId" | "EvidenceId" | "TypeId"
        | "PropertyId" | "AgentId" | "GraphRootId" | "SchemaVersionId" => {
            "An id: a UUID in lowercase hyphenated form. `ekr mint <kind>` prints a new one; \
             `ekr snapshot` and `ekr ontology` print existing ones."
        }
        id if id.ends_with("Id") => "An id: a UUID in lowercase hyphenated form.",
        _ => return None,
    })
}

/// The JSON Schema of an `ekr.transaction-document/1` document, which `ekr propose` reads.
///
/// # Panics
///
/// Never for the types it is generated from: the transaction definition and its `operations` and
/// `evidence` lists are what the derive of [`crate::GraphTransaction`] writes.
#[must_use]
pub fn transaction_document() -> Schema {
    let mut schema = generator(true).into_root_schema_for::<Envelope>();
    let transaction = schema
        .get_mut("$defs")
        .and_then(|definitions| definitions.get_mut("GraphTransaction"))
        .and_then(|transaction| transaction.get_mut("properties"))
        .expect("the transaction document defines GraphTransaction");
    transaction["operations"]["minItems"] = 1.into();
    transaction["operations"]["maxItems"] = DOCUMENT_V1_LIMITS.operations.into();
    transaction["evidence"]["maxItems"] = DOCUMENT_V1_LIMITS.evidence.into();
    describe(
        schema,
        "ekr.transaction-document/1",
        &format!(
            "A transaction document for `ekr propose`: one transaction, its operations and the \
             evidence they cite (`ekr example ekr.transaction-document/1`, `ekr operations`). \
             {YAML_TAGS} {V1_LIMITS_UNSEEN}"
        ),
        &[
            ("format", "Exactly `ekr.transaction-document/1`."),
            (
                "transaction",
                "The proposal: its new id (`ekr mint transaction`), the proposer (the host's \
                 operator), 1 to 256 operations (`ekr operations`) and the evidence ids its \
                 assertions cite.",
            ),
        ],
    )
}

/// The JSON Schema of an `ekr-seed/2` document, which `ekr seed` reads.
#[must_use]
pub fn seed_document() -> Schema {
    describe(
        generator(false).into_root_schema_for::<SeedDocument>(),
        "ekr-seed/2",
        &format!(
            "A seed for `ekr seed`: the ontology, the graph at revision 0 and the payload bytes \
             of its evidence (`ekr example ekr-seed/2`). {YAML_TAGS}"
        ),
        &[
            ("format", "Exactly `ekr-seed/2`."),
            (
                "ontology",
                "The schema version and the node and edge types the graph is typed by.",
            ),
            (
                "graph",
                "The graph at revision 0: root, nodes, edges, assertions and evidence, in the \
                 `ekr.graph-document/2` envelope.",
            ),
            (
                "evidence_payloads",
                "The retained bytes of each evidence entry, keyed by its content_hash (`ekr hash \
                 <file>` prints both), as a list of byte values.",
            ),
        ],
    )
}

/// `SeedDocument.graph`: the `ekr.graph-document/2` envelope `ekr_store::GraphDocument` decodes by
/// hand, around the fields of its remote derive, over the proposal value space.
pub(crate) fn graph_document(generator: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "type": "object",
        "properties": {
            "format": { "const": "ekr.graph-document/2" },
            "graph": {
                "type": "object",
                "properties": {
                    "root": generator.subschema_for::<GraphRoot>(),
                    "revision": generator.subschema_for::<RevisionNumber>(),
                    "nodes": generator.subschema_for::<BTreeMap<NodeId, Node<Value>>>(),
                    "edges": generator.subschema_for::<BTreeMap<EdgeId, Edge<Value>>>(),
                    "assertions":
                        generator.subschema_for::<BTreeMap<AssertionId, Assertion<Value>>>(),
                    "evidence": generator.subschema_for::<BTreeMap<EvidenceId, Evidence>>(),
                },
                "required": ["root", "revision", "nodes", "edges", "assertions", "evidence"],
                "additionalProperties": false,
            },
        },
        "required": ["format", "graph"],
        "additionalProperties": false,
    })
}
