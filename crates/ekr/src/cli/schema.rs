//! `ekr schema <format>`: the JSON Schema (draft 2020-12) of one input format, generated from
//! the types its reader decodes — `ekr_kernel::schema` for the two YAML formats, the host type for
//! `ekr.cli-host/1`, the shape of `ekr_integrate::TypedReference` for `typed-reference`.
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
    };
    serde_json::to_value(&schema).map_err(Failure::fault)
}
