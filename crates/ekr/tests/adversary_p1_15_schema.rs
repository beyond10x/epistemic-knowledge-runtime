//! Adversary pass 1 on `story:machine-readable-document-schemas`.
//!
//! The unit's claim: `ekr schema <format>` accepts exactly what the format's reader accepts, save
//! the gaps its `description` names (a key written twice, a `uniqueItems` list with a repeat, a
//! whole number written `1.0`, a range that ends before it starts). Each case here builds
//! documents outside those named gaps and asserts that the real reader and the printed schema
//! give the same answer for every one of them. A red case lists each disagreement: the reader's
//! verdict and the schema's refusals.

use std::process::Command;

use ekr::host::CliHostConfigurationV1;
use ekr_kernel::{SeedDocument, TransactionDocument};
use serde_json::Value;

const TRANSACTION: &str = "ekr.transaction-document/1";
const SEED: &str = "ekr-seed/2";
const HOST: &str = "ekr.cli-host/1";

fn stdout(args: &[&str]) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
        command.env_remove(var);
    }
    let output = command.args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn validator(format: &str) -> jsonschema::Validator {
    let schema: Value = serde_json::from_str(&stdout(&["schema", format])).unwrap();
    jsonschema::draft202012::new(&schema).unwrap()
}

/// The projection `schema_cli.rs` and the schema's own `description` define: the YAML read as a
/// plain `serde_yaml_ng::Value` and written as JSON.
fn instance(format: &str, document: &str) -> Value {
    if format == HOST {
        serde_json::from_str(document).unwrap()
    } else {
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(document).unwrap();
        serde_json::to_value(yaml).unwrap()
    }
}

fn read(format: &str, document: &str) -> Result<(), String> {
    match format {
        TRANSACTION => TransactionDocument::parse(document.as_bytes())
            .map(drop)
            .map_err(|e| e.to_string()),
        SEED => SeedDocument::from_yaml(document)
            .map(drop)
            .map_err(|e| e.to_string()),
        HOST => CliHostConfigurationV1::from_json(document.as_bytes())
            .map(drop)
            .map_err(|e| e.to_string()),
        other => panic!("no reader for {other}"),
    }
}

/// Every `(what, document)` on which the reader and the schema disagree, described.
fn disagreements(format: &str, documents: &[(&str, String)]) -> Vec<String> {
    let validator = validator(format);
    let mut out = Vec::new();
    for (what, document) in documents {
        let reader = read(format, document);
        let schema: Vec<String> = validator
            .iter_errors(&instance(format, document))
            .map(|e| format!("{}: {e}", e.instance_path()))
            .take(3)
            .collect();
        if reader.is_ok() != schema.is_empty() {
            let reader = match reader {
                Ok(()) => "reader ACCEPTS".to_owned(),
                Err(e) => format!("reader REFUSES ({})", truncate(&e)),
            };
            let schema = if schema.is_empty() {
                "schema ACCEPTS".to_owned()
            } else {
                format!("schema REFUSES ({})", truncate(&schema.join("; ")))
            };
            out.push(format!("{format}: {what}: {reader}, {schema}"));
        }
    }
    out
}

fn truncate(text: &str) -> String {
    text.chars().take(160).collect()
}

fn assert_agree(format: &str, documents: &[(&str, String)]) {
    let found = disagreements(format, documents);
    assert!(
        found.is_empty(),
        "the schema and the reader disagree:\n{}",
        found.join("\n")
    );
}

/// A disagreement the schema's own `description` names is a documented gap, not a defect: the
/// case then holds that the description names `phrase` and that the reader accepts while the
/// schema refuses, which is the direction the description states.
fn assert_documented_gap(format: &str, phrase: &str, documents: &[(&str, String)]) {
    let schema: Value = serde_json::from_str(&stdout(&["schema", format])).unwrap();
    let description = schema["description"].as_str().unwrap_or("");
    assert!(
        description.contains(phrase),
        "{format}: the description does not name {phrase:?}, so this is no documented gap"
    );
    let validator = validator(format);
    for (what, document) in documents {
        assert!(
            read(format, document).is_ok(),
            "{what}: the reader refuses it"
        );
        assert!(
            validator
                .iter_errors(&instance(format, document))
                .next()
                .is_some(),
            "{what}: the schema accepts it, so the description names a gap that is not there"
        );
    }
}

const ID: &str = "00000000-0000-4000-8000-000000000601";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";

/// A transaction document around `operations` (already indented two spaces as a YAML list) and
/// `evidence` (the same).
fn transaction(operations: &str, evidence: &str) -> String {
    format!(
        "format: {TRANSACTION}\ntransaction:\n  id: {ID}\n  proposer: {OPERATOR}\n  operations:{operations}\n  evidence:{evidence}\n"
    )
}

fn create_node(canonical_name: &str, properties: &str) -> String {
    format!(
        "\n  - !CreateNode\n    id: 00000000-0000-4000-8000-000000000304\n    root_id: \
         00000000-0000-4000-8000-000000000002\n    type_id: 00000000-0000-4000-8000-000000000202\n    \
         canonical_name: {canonical_name}\n    properties:{properties}"
    )
}

fn one_value(kind: &str, value: &str) -> String {
    format!(
        "\n      00000000-0000-4000-8000-000000000801:\n      - value_kind: {kind}\n        value: {value}"
    )
}

// A ----------------------------------------------------------------------------------------------

/// `DocumentLimit::EmptyOperations`: the reader refuses a transaction with no operation
/// (`crates/ekr-kernel/src/document.rs`, "at least one is required"). The schema must too.
#[test]
fn a_transaction_with_no_operation_is_refused_by_both() {
    let empty = transaction(" []", " []");
    assert!(
        read(TRANSACTION, &empty).is_err(),
        "the reader accepts {empty}"
    );
    assert_agree(TRANSACTION, &[("operations: []", empty)]);
}

/// `DOCUMENT_V1_LIMITS` is frozen per format version ("Changing it requires a version/migration
/// decision; callers cannot override it"), so it is part of what `ekr.transaction-document/1`
/// accepts: 256 operations, 1024 evidence entries, 65 536 bytes in one string.
#[test]
fn the_frozen_transaction_document_limits_are_the_schemas_too() {
    let delete = "\n  - !DeleteEdge 00000000-0000-4000-8000-000000000700";
    let evidence = "\n  - 00000000-0000-4000-8000-000000000401";
    let documents = vec![
        ("257 operations", transaction(&delete.repeat(257), " []")),
        (
            "1025 evidence entries",
            transaction(delete, &evidence.repeat(1025)),
        ),
        (
            "a 65537-byte canonical_name",
            transaction(&create_node(&"a".repeat(65_537), " {}"), " []"),
        ),
    ];
    assert_agree(TRANSACTION, &documents);
}

// B ----------------------------------------------------------------------------------------------

/// A `String` field (`canonical_name`, `Value::Decimal`, `Value::Enum`, `RetractionReason`,
/// `type_state`) is read by `deserialize_str`, which takes a plain scalar's text whatever it
/// resolves to. The projection resolves `12.50` to a number and `true` to a boolean, so the schema
/// sees another kind for a document the reader accepts. `value: 12.50` for a Decimal is the
/// natural way to write one.
#[test]
fn a_plain_scalar_a_string_field_reads_as_text_is_accepted_by_both() {
    let transactions = vec![
        (
            "Decimal value: 12.50",
            transaction(&create_node("Globex", &one_value("Decimal", "12.50")), " []"),
        ),
        (
            "Enum value: true",
            transaction(&create_node("Globex", &one_value("Enum", "true")), " []"),
        ),
        (
            "String value: 42",
            transaction(&create_node("Globex", &one_value("String", "42")), " []"),
        ),
        (
            "canonical_name: 2024",
            transaction(&create_node("2024", " {}"), " []"),
        ),
        (
            "RetractAssertion reason: 404",
            transaction(
                "\n  - !RetractAssertion\n    assertion: 00000000-0000-4000-8000-000000000512\n    \
                 reason: 404",
                " []",
            ),
        ),
    ];
    let mut found = disagreements(TRANSACTION, &transactions);

    let seed = stdout(&["example", SEED]);
    let seeds = vec![
        (
            "graph node canonical_name: 2024",
            seed.replacen("canonical_name: Alice", "canonical_name: 2024", 1),
        ),
        (
            "graph node type_state: 1",
            seed.replacen("type_state: null", "type_state: 1", 1),
        ),
        (
            "node type name: 7",
            seed.replacen("name: Person", "name: 7", 1),
        ),
    ];
    found.extend(disagreements(SEED, &seeds));
    assert!(
        found.is_empty(),
        "the schema and the reader disagree:\n{}",
        found.join("\n")
    );
}

/// The transaction reader ignores a tag on a typed scalar ("ignored tag metadata adds no semantic
/// depth", `document.rs`). The projection spells that tag as a one-key object, which no scalar's
/// schema admits.
#[test]
fn a_tag_on_a_scalar_is_judged_alike_by_both() {
    let documents = vec![
        (
            "proposer: !AgentId <id>",
            transaction(
                "\n  - !DeleteEdge 00000000-0000-4000-8000-000000000700",
                " []",
            )
            .replacen(
                &format!("proposer: {OPERATOR}"),
                &format!("proposer: !AgentId {OPERATOR}"),
                1,
            ),
        ),
        (
            "canonical_name: !Name Globex",
            transaction(&create_node("!Name Globex", " {}"), " []"),
        ),
    ];
    assert_documented_gap(TRANSACTION, "a tag on a scalar", &documents);
}

// C ----------------------------------------------------------------------------------------------

/// `Timestamp` is an `i64`, `Validating`'s counts are `u32`, `Value::Integer` is an `i64`; the
/// schema prints only `"format": "int64"` / `"uint32"`, which draft 2020-12 treats as an
/// annotation, so a number one past the type's range passes the schema and not the reader.
#[test]
fn a_whole_number_past_its_type_is_refused_by_both() {
    let example = stdout(&["example", TRANSACTION]);
    let documents = vec![
        (
            "valid_time.from = i64::MAX + 1",
            example.replacen("from: 1577836800000", "from: 9223372036854775808", 1),
        ),
        (
            "assessment !Validating completed = u32::MAX + 1",
            example.replacen(
                "assessment: Proposed",
                "assessment: !Validating\n      completed: 4294967296\n      required: 7",
                1,
            ),
        ),
        (
            "Integer value = i64::MAX + 1",
            transaction(
                &create_node("Globex", &one_value("Integer", "9223372036854775808")),
                " []",
            ),
        ),
    ];
    assert_agree(TRANSACTION, &documents);
}

/// `document.rs`: "nonfinite Float proposals remain available to kernel validation" — the reader
/// accepts `.nan`. The projection writes it as JSON `null`, and the schema's Float branch wants a
/// number.
#[test]
fn a_nonfinite_float_the_reader_accepts_is_accepted_by_the_schema() {
    let documents = vec![(
        "Float value: .nan",
        transaction(&create_node("Globex", &one_value("Float", ".nan")), " []"),
    )];
    assert_documented_gap(TRANSACTION, "`.nan` or `.inf`", &documents);
}

// D ----------------------------------------------------------------------------------------------

/// A YAML `null` (or an empty value) where the reader expects a list, a map or a struct whose
/// fields are all optional: the projection sees `null`, which no array/object schema admits.
#[test]
fn a_null_collection_is_judged_alike_by_both() {
    let example = stdout(&["example", TRANSACTION]);
    let transactions = vec![
        ("transaction.evidence: null", transaction("\n  - !DeleteEdge 00000000-0000-4000-8000-000000000700", " null")),
        ("CreateNode properties: null", transaction(&create_node("Globex", " null"), " []")),
        (
            "valid_time: null",
            example.replacen("valid_time:\n      from: 1577836800000\n      to: null", "valid_time: null", 1),
        ),
        (
            "Invoke arguments: null",
            transaction(
                "\n  - !Invoke\n    node: 00000000-0000-4000-8000-000000000303\n    operation: record_review\n    arguments: null",
                " []",
            ),
        ),
    ];
    let mut found = disagreements(TRANSACTION, &transactions);
    let seed = stdout(&["example", SEED]);
    let seeds = vec![
        (
            "node aliases: null",
            seed.replacen("aliases: []", "aliases: null", 1),
        ),
        (
            "node type parents: null",
            seed.replacen("parents: []", "parents: null", 1),
        ),
        (
            "node type operations: null",
            seed.replacen("operations: {}", "operations: null", 1),
        ),
        (
            "graph edges: null",
            seed.replacen("    edges:\n", "    edges: null\n    edges_:\n", 1),
        ),
    ];
    found.extend(disagreements(SEED, &seeds));
    assert!(
        found.is_empty(),
        "the schema and the reader disagree:\n{}",
        found.join("\n")
    );
}

/// Fields left out of a seed: what serde defaults the schema must not require, and what it does
/// not the schema must.
#[test]
fn a_seed_field_left_out_is_judged_alike_by_both() {
    let seed = stdout(&["example", SEED]);
    let payloads = seed.find("evidence_payloads:").unwrap();
    let documents = vec![
        (
            "node aliases left out",
            seed.replacen("        aliases: []\n", "", 1),
        ),
        (
            "node type_state left out",
            seed.replacen("        type_state: null\n", "", 1),
        ),
        ("evidence_payloads left out", seed[..payloads].to_owned()),
        ("ontology edge_types left out", {
            let start = seed.find("  edge_types:").unwrap();
            let end = seed.find("graph:\n").unwrap();
            format!("{}{}", &seed[..start], &seed[end..])
        }),
        (
            "property definition cardinality/required/constraints left out",
            seed.replacen(
                "        cardinality: One\n        required: false\n        constraints: []\n",
                "",
                1,
            ),
        ),
        (
            "edge type cardinality left out",
            seed.replacen(
                "    cardinality: One\n    properties: {}\n    inverse",
                "    properties: {}\n    inverse",
                1,
            ),
        ),
        (
            "confidence: -1",
            seed.replacen("confidence: 10000", "confidence: -1", 1),
        ),
        (
            "HumanStatement identity: null",
            seed.replacen("identity: Runtime operator", "identity: null", 1),
        ),
        (
            "evidence source !Url",
            seed.replacen(
                "source: !HumanStatement\n          identity: Runtime operator",
                "source: !Url https://example.org/acme",
                1,
            ),
        ),
        (
            "evidence source !Document without section",
            seed.replacen(
                "source: !HumanStatement\n          identity: Runtime operator",
                "source: !Document\n          document_id: annual-report",
                1,
            ),
        ),
    ];
    assert_agree(SEED, &documents);
}

/// The host document: shapes a JSON writer produces that no example prints.
#[test]
fn a_host_document_edge_is_judged_alike_by_both() {
    let host: Value = serde_json::from_str(&stdout(&["example", HOST])).unwrap();
    let mut documents = Vec::new();
    let mut change = |what: &'static str, edit: &dyn Fn(&mut Value)| {
        let mut document = host.clone();
        edit(&mut document);
        documents.push((what, serde_json::to_string(&document).unwrap()));
    };
    change("tenant empty", &|d| d["tenant"] = serde_json::json!(""));
    change("no agents", &|d| {
        d["authority"]["agents"] = serde_json::json!({})
    });
    change("no checks", &|d| {
        d["authority"]["validation_profile"]["checks"] = serde_json::json!([])
    });
    change("a check listed twice", &|d| {
        d["authority"]["validation_profile"]["checks"] = serde_json::json!(["Type", "Type"]);
    });
    change("capabilities empty", &|d| {
        d["authority"]["agents"]["00000000-0000-4000-8000-000000000101"]["capabilities"] =
            serde_json::json!([]);
    });
    change("context null", &|d| d["context"] = Value::Null);
    change("an agent keyed in uppercase", &|d| {
        let agents = d["authority"]["agents"].as_object_mut().unwrap();
        let agent = agents
            .remove("00000000-0000-4000-8000-000000000102")
            .unwrap();
        agents.insert("00000000-0000-4000-8000-00000000010A".into(), agent);
    });
    assert_agree(HOST, &documents);
}
