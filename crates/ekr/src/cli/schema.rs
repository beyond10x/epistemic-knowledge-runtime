//! `ekr schema <format>`: the JSON Schema (draft 2020-12) of one input format, generated from
//! the types its reader decodes — `ekr_kernel::schema` for the two YAML formats, the host type for
//! `ekr.cli-host/1`, the shape of `ekr_integrate::TypedReference` for `typed-reference`, and
//! `ekr_integrate::ExtractionDocument` for `ekr.extraction-document/1`.
//! `crates/ekr/tests/schema_cli.rs` holds each to its reader in both directions.

use super::ExampleFormat;
use crate::exit::Failure;
use crate::host::CliHostConfigurationV1;

/// The schema as one JSON document, which the verb prints as every JSON verb does.
pub(super) fn run(format: ExampleFormat) -> Result<serde_json::Value, Failure> {
    let schema = match format {
        ExampleFormat::TransactionDocument => {
            ekr_kernel::schema::transaction_document(ekr_kernel::DocumentFormat::V2)
        }
        ExampleFormat::TransactionDocumentV1 => {
            ekr_kernel::schema::transaction_document(ekr_kernel::DocumentFormat::V1)
        }
        ExampleFormat::Seed => ekr_kernel::schema::seed_document(),
        ExampleFormat::Host => CliHostConfigurationV1::json_schema_document(),
        ExampleFormat::TypedReference => super::resolve::schema(),
        ExampleFormat::Extraction => extraction_document(),
    };
    serde_json::to_value(&schema).map_err(Failure::fault)
}

/// The JSON Schema of an `ekr.extraction-document/1`, generated from
/// `ekr_integrate::ExtractionDocument` as the seed's is from its type.
fn extraction_document() -> schemars::Schema {
    let format = ekr_integrate::EXTRACTION_FORMAT;
    let mut schema = ekr_kernel::schema::yaml_document::<ekr_integrate::ExtractionDocument>(
        format,
        &format!(
            "An extraction document: what an extracting agent read from its sources, for a store \
             to take in (`ekr example {format}`). Types, properties and relations are named, never \
             identified. Beyond the schema, the reader also refuses, each by its code: a \
             document over {} bytes (`extraction-document-too-large`); containers nested deeper \
             than {} (`extraction-document-too-deep`); a YAML alias (`*name`, \
             `extraction-yaml-alias`); a mapping key written twice \
             (`extraction-document-malformed`); a name declared twice in the document, a node \
             type, an edge type or a property of one type (`extraction-name-duplicate`); and, \
             against the store it is read for, a store type redeclared with other parents, \
             abstractness or cardinality (`extraction-type-conflict`), a node type or edge type \
             that neither the document's ontology nor the store declares \
             (`extraction-type-undeclared`), a named thing or a fact's subject or object with no \
             alias but the empty string (`reference-without-identity`), a property the subject's \
             type and its ancestors do not declare (`extraction-property-undeclared`), a value \
             its property's type does not hold (`extraction-value-mismatch`), a relation between \
             node types its edge type does not connect (`extraction-relation-ends`), a fact \
             citing an evidence id no evidence item of the document carries \
             (`fact-evidence-unlisted`), two evidence items under one id (`duplicate-identity`) \
             and a payload that does not hash to its entry's content_hash \
             (`evidence-payload-mismatch`). A fact citing no evidence, an empty or repeated Enum \
             variant list and an empty or repeated NodeRef type list are refused by both, by the \
             reader as `fact-without-evidence`, `extraction-value-type-empty` and \
             `extraction-name-duplicate`.",
            ekr_integrate::EXTRACTION_INPUT_BYTES,
            ekr_integrate::EXTRACTION_DEPTH
        ),
        &[
            ("format", &format!("Exactly `{format}`.")),
            (
                "ontology",
                "The node and edge types the document needs, by name, each with its parents, ends \
                 and properties: a type the store lacks is added, one it holds is kept. Omit it \
                 when the store declares every type the document names.",
            ),
            (
                "entities",
                "The named things the source mentions, each a node type's name and the aliases \
                 the thing is known by; one is resolved against the store before it is created.",
            ),
            (
                "facts",
                "What the source says about the named things: `!Property` (a value of a property \
                 of the subject's type) or `!Relation` (an edge type between subject and object), \
                 each citing the ids of at least one evidence item of this document.",
            ),
            (
                "evidence",
                "The evidence items the facts cite, each the entry and payload an `!AddEvidence` \
                 carries: the entry under a fresh id (`ekr mint evidence`) with the host \
                 operator as `extracted_by`, and the exact bytes its `content_hash` addresses \
                 (`ekr hash`).",
            ),
        ],
    );
    // An alias is a YAML string, as a typed reference's is: the reader refuses a null, a boolean
    // or a number, so the schema does not widen it to any scalar as it does other text.
    if let Some(aliases) = schema
        .get_mut("$defs")
        .and_then(|definitions| definitions.get_mut("ExtractedReference"))
        .and_then(|reference| reference["properties"]["aliases"].as_object_mut())
    {
        aliases.insert("items".to_owned(), serde_json::json!({"type": "string"}));
    }
    // A fact's `evidence` decodes as empty when absent, so that the reader can refuse it by name
    // (`fact-without-evidence`); the schema refuses both, which the derive cannot say.
    for fact in ["PropertyFact", "RelationFact"] {
        if let Some(fact) = schema
            .get_mut("$defs")
            .and_then(|definitions| definitions.get_mut(fact))
        {
            if let Some(evidence) = fact["properties"]["evidence"].as_object_mut() {
                evidence.remove("default");
            }
            if let Some(required) = fact["required"].as_array_mut() {
                required.push("evidence".into());
            }
        }
    }
    schema
}
