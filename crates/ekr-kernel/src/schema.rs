//! JSON Schemas (draft 2020-12) of the two YAML documents the kernel reads, behind the `schema`
//! feature: `ekr.transaction-document/1` ([`TransactionDocument`](crate::TransactionDocument))
//! and `ekr-seed/2` ([`SeedDocument`]).
//!
//! Each is generated from the types those readers deserialize. Where a type decodes by hand, its
//! schema is written beside that decoder: the id and hash scalars in `ekr-core`, a confidence, a
//! reference and a non-empty property value list in `ekr-graph`, and here the graph document a
//! seed nests, whose envelope `ekr-store` decodes.

use std::collections::BTreeMap;

use ekr_core::{AssertionId, EdgeId, EvidenceId, NodeId, RevisionNumber};
use ekr_graph::{Assertion, Edge, Evidence, GraphRoot, Node};
use ekr_ontology::Value;
use schemars::generate::SchemaSettings;
use schemars::transform::RecursiveTransform;
use schemars::{json_schema, Schema, SchemaGenerator};

use crate::document::Envelope;
use crate::SeedDocument;

/// What every YAML format's schema says about the one thing JSON Schema cannot express.
const YAML_TAGS: &str = "The document is YAML; this schema validates its JSON projection, \
the document read as plain YAML and written as JSON. A YAML tag `!Kind value` (an operation such \
as `!AddAssertion`, a subject such as `!Node <id>`, an evidence source such as `!HumanStatement`) \
has no JSON Schema form, so the projection and this schema spell it as the one-key object \
{\"!Kind\": value}. The readers do not accept that object in place of the tag: write the tag. \
Beyond the schema, the readers also refuse a mapping key written twice, a list the schema marks \
uniqueItems holding one value twice, a whole number written with a fraction (`1.0`), and a \
time range or transaction time that ends before it starts.";

/// The draft 2020-12 generator every schema here is written with, with [`yaml_unit_forms`].
fn generator() -> SchemaGenerator {
    SchemaSettings::draft2020_12()
        .with_transform(RecursiveTransform(yaml_unit_forms))
        .into_generator()
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
    let Some(branches) = schema
        .get_mut("oneOf")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    let mut tagged = Vec::new();
    for branch in branches.iter_mut() {
        if branch.get("type").and_then(serde_json::Value::as_str) == Some("string") {
            if let Some(name) = branch.get("const").and_then(serde_json::Value::as_str) {
                tagged.push(serde_json::json!({
                    "type": "object",
                    "properties": { format!("!{name}"): { "type": "null" } },
                    "required": [format!("!{name}")],
                    "additionalProperties": false,
                }));
            }
        }
        let only_the_tag = branch
            .get("properties")
            .and_then(serde_json::Value::as_object)
            .is_some_and(|properties| {
                properties.len() == 1
                    && properties
                        .get("value_kind")
                        .is_some_and(|kind| kind.get("const").is_some())
            });
        if only_the_tag && branch.get("additionalProperties") == Some(&false.into()) {
            if let Some(properties) = branch
                .get_mut("properties")
                .and_then(serde_json::Value::as_object_mut)
            {
                properties.insert("parameters".to_owned(), serde_json::json!({"type": "null"}));
            }
        }
    }
    branches.extend(tagged);
}

fn titled(mut schema: Schema, title: &str, description: &str) -> Schema {
    schema.insert("title".to_owned(), title.into());
    schema.insert("description".to_owned(), description.into());
    schema
}

/// The JSON Schema of an `ekr.transaction-document/1` document, which `ekr propose` reads.
#[must_use]
pub fn transaction_document() -> Schema {
    titled(
        generator().into_root_schema_for::<Envelope>(),
        "ekr.transaction-document/1",
        &format!(
            "A transaction document for `ekr propose`: one transaction, its operations and the \
             evidence they cite (`ekr example ekr.transaction-document/1`, `ekr operations`). \
             {YAML_TAGS}"
        ),
    )
}

/// The JSON Schema of an `ekr-seed/2` document, which `ekr seed` reads.
#[must_use]
pub fn seed_document() -> Schema {
    titled(
        generator().into_root_schema_for::<SeedDocument>(),
        "ekr-seed/2",
        &format!(
            "A seed for `ekr seed`: the ontology, the graph at revision 0 and the payload bytes \
             of its evidence (`ekr example ekr-seed/2`). {YAML_TAGS}"
        ),
    )
}

/// `SeedDocument.graph`: the `ekr.graph-document/2` envelope `ekr_store::GraphDocument` decodes by
/// hand, around the fields of its remote derive, over the proposal value space.
pub(crate) fn graph_document(generator: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "type": "object",
        "description": "The graph at revision 0, in the ekr.graph-document/2 envelope.",
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
