//! `story:machine-readable-document-schemas`: `ekr schema <format>` prints a JSON Schema (draft
//! 2020-12) for each document an agent writes, generated from the Rust types the readers
//! deserialize.
//!
//! Every case drives fresh binary processes and holds the printed schema to the real readers
//! (`TransactionDocument::parse`, `SeedDocument::from_yaml`, `CliHostConfigurationV1::from_json`)
//! in both directions: what an example or operation page prints is accepted by both, and a
//! document a reader refuses for a missing field, an unknown field or a wrong value kind is
//! refused by the schema too.
//!
//! The two YAML formats are validated as their JSON projection: the document read into a
//! `serde_yaml_ng::Value` and written as JSON, which spells a YAML tag `!Kind value` as the
//! one-key object `{"!Kind": value}`. That is the spelling the schemas use, and
//! [`a_yaml_tag_is_spelled_as_a_one_key_object_the_readers_do_not_accept`] holds what each
//! schema says about it.

use std::process::{Command, Output};

use ekr::host::CliHostConfigurationV1;
use ekr_kernel::{SeedDocument, TransactionDocument};
use serde_json::{json, Value};

const TRANSACTION: &str = "ekr.transaction-document/1";
const SEED: &str = "ekr-seed/2";
const HOST: &str = "ekr.cli-host/1";
/// `(format, the alias `ekr example` also accepts)`.
const FORMATS: [(&str, &str); 3] = [(TRANSACTION, "transaction"), (SEED, "seed"), (HOST, "host")];
const DRAFT_2020_12: &str = "https://json-schema.org/draft/2020-12/schema";

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
        command.env_remove(var);
    }
    command
}

fn run(args: &[&str]) -> Output {
    ekr().args(args).output().unwrap()
}

/// Exit 0, stdout as text, stderr empty.
fn text(args: &[&str]) -> String {
    let output = run(args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "{args:?} wrote stderr");
    String::from_utf8(output.stdout).unwrap()
}

/// `ekr schema <format>`'s stdout, which must be exactly one JSON document.
fn schema(format: &str) -> Value {
    let out = text(&["schema", format]);
    serde_json::from_str(&out).unwrap_or_else(|e| panic!("schema {format}: not JSON ({e}): {out}"))
}

fn validator(format: &str) -> jsonschema::Validator {
    let schema = schema(format);
    jsonschema::draft202012::new(&schema)
        .unwrap_or_else(|e| panic!("schema {format} does not compile: {e}"))
}

/// The instance a schema validates: the JSON host document as it is, a YAML document as its JSON
/// projection (`serde_yaml_ng::Value` written as JSON; a tag `!Kind value` is `{"!Kind": value}`).
fn instance(format: &str, document: &str) -> Value {
    if format == HOST {
        serde_json::from_str(document).unwrap()
    } else {
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(document)
            .unwrap_or_else(|e| panic!("not YAML ({e}): {document}"));
        serde_json::to_value(yaml).unwrap()
    }
}

/// The format's real reader, as the binary's verbs call it.
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

/// The schema's refusals of `document`, one line each; empty when it validates.
fn refusals(validator: &jsonschema::Validator, format: &str, document: &str) -> Vec<String> {
    let instance = instance(format, document);
    validator
        .iter_errors(&instance)
        .map(|error| format!("{}: {error}", error.instance_path()))
        .collect()
}

/// `document` with `from` replaced by `to`, once; the example must contain `from`.
fn edit(document: &str, from: &str, to: &str) -> String {
    assert!(document.contains(from), "{from:?} is not in {document}");
    document.replacen(from, to, 1)
}

/// The kinds `ekr operations` lists, one per line, the kind name first.
fn listed_kinds() -> Vec<String> {
    text(&["operations"])
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split_whitespace().next().unwrap().to_owned())
        .collect()
}

/// The example operation in `ekr operations <Kind>`: every line after the `Example` heading.
fn example_operation(kind: &str) -> String {
    let out = text(&["operations", kind]);
    let at = out
        .lines()
        .position(|line| line.starts_with("Example"))
        .unwrap_or_else(|| panic!("{kind}: no Example section in {out}"));
    let mut example: String = out.lines().skip(at + 1).collect::<Vec<_>>().join("\n");
    example.push('\n');
    example
}

/// A complete `ekr.transaction-document/1` around one printed operation entry.
fn document_around(operation: &str) -> String {
    let zero = "00000000-0000-4000-8000-000000000000";
    let mut text = format!(
        "format: {TRANSACTION}\ntransaction:\n  id: {zero}\n  proposer: {zero}\n  operations:\n"
    );
    for line in operation.lines() {
        text.push_str("  ");
        text.push_str(line);
        text.push('\n');
    }
    text.push_str("  evidence: []\n");
    text
}

// 1 --------------------------------------------------------------------------------------------

/// `ekr schema <format>` prints, for each of the three formats and under the same alias `ekr
/// example` takes, one JSON document that is a valid draft 2020-12 schema and declares that
/// draft; it is exactly what the kernel's and the host's generators return; an unknown format is
/// a usage error.
#[test]
fn schema_prints_one_draft_2020_12_json_schema_per_format() {
    for (format, alias) in FORMATS {
        let printed = schema(format);
        assert_eq!(printed["$schema"], DRAFT_2020_12, "{format}");
        jsonschema::draft202012::meta::validate(&printed)
            .unwrap_or_else(|e| panic!("{format}: not a draft 2020-12 schema: {e}"));
        assert_eq!(schema(alias), printed, "{alias} and {format} differ");
        assert!(
            printed["title"]
                .as_str()
                .is_some_and(|t| t.contains(format)),
            "{format}: the title does not name the format: {printed}"
        );
    }
    assert_eq!(
        schema(TRANSACTION),
        serde_json::to_value(ekr_kernel::schema::transaction_document()).unwrap()
    );
    assert_eq!(
        schema(SEED),
        serde_json::to_value(ekr_kernel::schema::seed_document()).unwrap()
    );
    assert_eq!(
        schema(HOST),
        serde_json::to_value(ekr::host::CliHostConfigurationV1::json_schema_document()).unwrap()
    );
    let unknown = run(&["schema", "ekr-seed/9"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(unknown.stdout.is_empty());
}

// 2 --------------------------------------------------------------------------------------------

/// Every `ekr example` document is accepted by its reader and validates against its schema.
#[test]
fn every_example_document_validates_against_its_printed_schema() {
    for (format, _) in FORMATS {
        let example = text(&["example", format]);
        read(format, &example).unwrap_or_else(|e| panic!("{format}: the reader refused: {e}"));
        let refused = refusals(&validator(format), format, &example);
        assert!(
            refused.is_empty(),
            "{format}: the schema refuses the example:\n{}",
            refused.join("\n")
        );
    }
}

/// Every `ekr operations <Kind>` example, in a transaction document, is accepted by the reader
/// and validates against the transaction-document schema — the four kinds P1 does not apply too,
/// since they parse.
#[test]
fn every_operation_example_validates_against_the_transaction_document_schema() {
    let validator = validator(TRANSACTION);
    let kinds = listed_kinds();
    assert_eq!(kinds.len(), 12, "{kinds:?}");
    for kind in kinds {
        let document = document_around(&example_operation(&kind));
        read(TRANSACTION, &document)
            .unwrap_or_else(|e| panic!("{kind}: the reader refused {document}: {e}"));
        let refused = refusals(&validator, TRANSACTION, &document);
        assert!(
            refused.is_empty(),
            "{kind}: the schema refuses {document}:\n{}",
            refused.join("\n")
        );
    }
}

// 3 --------------------------------------------------------------------------------------------

/// `(what is wrong, the document)`: each is refused by the format's reader.
fn refused_documents(format: &str) -> Vec<(&'static str, String)> {
    let example = text(&["example", format]);
    let operator = "00000000-0000-4000-8000-000000000101";
    match format {
        TRANSACTION => vec![
            (
                "missing transaction.proposer",
                edit(&example, &format!("  proposer: {operator}\n"), ""),
            ),
            (
                "unknown transaction.note",
                edit(&example, "transaction:\n", "transaction:\n  note: extra\n"),
            ),
            (
                "transaction.id is a number",
                edit(
                    &example,
                    "  id: 00000000-0000-4000-8000-000000000601",
                    "  id: 601",
                ),
            ),
            (
                "transaction.id is uppercase",
                edit(
                    &example,
                    "  id: 00000000-0000-4000-8000-000000000601",
                    "  id: 00000000-0000-4000-8000-00000000060A",
                ),
            ),
            (
                "an operation kind that does not exist",
                edit(&example, "- !AddAssertion", "- !AddClaim"),
            ),
            (
                "an operation written without its tag",
                edit(&example, "- !AddAssertion\n", "- AddAssertion:\n"),
            ),
            (
                "assessment is not a variant",
                edit(&example, "assessment: Proposed", "assessment: Probable"),
            ),
            (
                "valid_time.from is text",
                edit(&example, "from: 1577836800000", "from: yesterday"),
            ),
            (
                "an assertion cites one evidence id twice",
                edit(
                    &example,
                    "    evidence:\n    - 00000000-0000-4000-8000-000000000401\n",
                    "    evidence:\n    - 00000000-0000-4000-8000-000000000401\n    - \
                     00000000-0000-4000-8000-000000000401\n",
                ),
            ),
            (
                "another format version",
                edit(
                    &example,
                    "format: ekr.transaction-document/1",
                    "format: ekr.transaction-document/2",
                ),
            ),
        ],
        SEED => vec![
            (
                "missing evidence confidence",
                edit(&example, "        confidence: 10000\n", ""),
            ),
            (
                "missing graph",
                edit(
                    &example,
                    "graph:\n  format: ekr.graph-document/2\n",
                    "graph_:\n  format: ekr.graph-document/2\n",
                ),
            ),
            (
                "unknown evidence field",
                edit(
                    &example,
                    "        confidence: 10000\n",
                    "        confidence: 10000\n        weight: 1\n",
                ),
            ),
            (
                "graph.revision is text",
                edit(&example, "    revision: 0\n", "    revision: zero\n"),
            ),
            (
                "confidence above 10000 basis points",
                edit(&example, "confidence: 10000", "confidence: 10001"),
            ),
            (
                "a content hash in uppercase",
                edit(
                    &example,
                    "content_hash: 2f954f8f77731e11a4dca21e0bb6c566f719aae4116838f3095f60649e5429db",
                    "content_hash: 2F954F8F77731E11A4DCA21E0BB6C566F719AAE4116838F3095F60649E5429DB",
                ),
            ),
            (
                "a payload byte above 255",
                edit(&example, "[65, 108, 105,", "[65, 256, 105,"),
            ),
            (
                "a node keyed by something that is not an id",
                edit(
                    &example,
                    "      00000000-0000-4000-8000-000000000301:\n",
                    "      alice:\n",
                ),
            ),
            (
                "a node property with no values",
                edit(
                    &example,
                    "        canonical_name: Acme\n        aliases: []\n        type_state: null\n        \
                     properties: {}\n",
                    "        canonical_name: Acme\n        aliases: []\n        type_state: null\n        \
                     properties:\n          00000000-0000-4000-8000-000000000801: []\n",
                ),
            ),
            (
                "another graph document format",
                edit(
                    &example,
                    "format: ekr.graph-document/2",
                    "format: ekr.graph-document/1",
                ),
            ),
            (
                "another seed format",
                edit(&example, "format: ekr-seed/2", "format: ekr-seed/1"),
            ),
            (
                "a value_kind that does not exist",
                edit(&example, "value_kind: String", "value_kind: Text"),
            ),
        ],
        HOST => {
            let host: Value = serde_json::from_str(&example).unwrap();
            let mut documents = Vec::new();
            let mut change = |what: &'static str, edit: &dyn Fn(&mut Value)| {
                let mut document = host.clone();
                edit(&mut document);
                documents.push((what, serde_json::to_string_pretty(&document).unwrap()));
            };
            change("missing tenant", &|d| {
                d.as_object_mut().unwrap().remove("tenant");
            });
            change("unknown field", &|d| d["extra"] = json!(true));
            change("tenant is a number", &|d| d["tenant"] = json!(5));
            change("another host format", &|d| {
                d["format"] = json!("ekr.cli-host/2");
            });
            change("context.operator is not an id", &|d| {
                d["context"]["operator"] = json!("operator");
            });
            change("missing context.validator", &|d| {
                d["context"].as_object_mut().unwrap().remove("validator");
            });
            change("a capability listed twice", &|d| {
                d["authority"]["agents"][operator]["capabilities"] = json!(["read", "read"]);
            });
            change("a check that is not a validator", &|d| {
                d["authority"]["validation_profile"]["checks"] = json!(["Spelling"]);
            });
            documents
        }
        other => panic!("no refusals for {other}"),
    }
}

/// Per format, documents the reader refuses — a missing required field, an unknown field, a
/// wrong value kind, and more — are refused by the schema too.
#[test]
fn each_schema_refuses_documents_its_reader_refuses() {
    for (format, _) in FORMATS {
        let validator = validator(format);
        let documents = refused_documents(format);
        assert!(documents.len() >= 3, "{format}");
        let mut accepted = Vec::new();
        for (what, document) in documents {
            assert!(
                read(format, &document).is_err(),
                "{format}: {what}: the reader accepts {document}"
            );
            if refusals(&validator, format, &document).is_empty() {
                accepted.push(what);
            }
        }
        assert!(
            accepted.is_empty(),
            "{format}: the schema accepts what the reader refuses: {accepted:?}"
        );
    }
}

/// `(what is unusual, the document)`: each is accepted by the format's reader.
fn accepted_documents(format: &str) -> Vec<(&'static str, String)> {
    let example = text(&["example", format]);
    match format {
        TRANSACTION => vec![
            (
                "transaction.evidence names one id twice (a set, read without a duplicate check)",
                edit(
                    &example,
                    "  evidence:\n  - 00000000-0000-4000-8000-000000000401\n",
                    "  evidence:\n  - 00000000-0000-4000-8000-000000000401\n  - \
                     00000000-0000-4000-8000-000000000401\n",
                ),
            ),
            (
                "valid_time omits its optional bounds",
                edit(
                    &example,
                    "    valid_time:\n      from: 1577836800000\n      to: null\n",
                    "    valid_time: {}\n",
                ),
            ),
            (
                "a unit variant written as a tag with no value (`!Proposed`, `!Active null`)",
                edit(
                    &edit(&example, "assessment: Proposed", "assessment: !Proposed"),
                    "lifecycle: Active",
                    "lifecycle: !Active null",
                ),
            ),
        ],
        SEED => vec![
            (
                "a node type omits every defaulted field",
                edit(
                    &example,
                    "    name: Person\n    parents: []\n    properties: {}\n    abstract_type: \
                     false\n    lifecycle: null\n    operations: {}\n",
                    "    name: Person\n",
                ),
            ),
            (
                "a HumanStatement with no identity",
                edit(
                    &example,
                    "!HumanStatement\n          identity: Runtime operator\n",
                    "!HumanStatement {}\n",
                ),
            ),
            (
                "a value_type without parameters that writes `parameters: null`",
                edit(
                    &example,
                    "        value_type:\n          value_kind: String\n",
                    "        value_type:\n          value_kind: String\n          parameters: null\n",
                ),
            ),
            (
                "cardinality and space written as tags (`!One`, `!Canonical`)",
                edit(
                    &edit(&example, "    cardinality: One\n", "    cardinality: !One\n"),
                    "space: Canonical",
                    "space: !Canonical",
                ),
            ),
            (
                "the schema version omits its optional parent",
                edit(
                    &example,
                    "    parent: null\n    created_at: 0\n  node_types:",
                    "    created_at: 0\n  node_types:",
                ),
            ),
        ],
        HOST => vec![(
            "the host document on one line",
            serde_json::to_string(&serde_json::from_str::<Value>(&example).unwrap()).unwrap(),
        )],
        other => panic!("no acceptances for {other}"),
    }
}

/// The other direction: a document the reader accepts in a form no example prints — a defaulted
/// field left out, an optional one absent, a set with a repeat the reader does not refuse — is
/// accepted by the schema too.
#[test]
fn each_schema_accepts_documents_its_reader_accepts() {
    for (format, _) in FORMATS {
        let validator = validator(format);
        for (what, document) in accepted_documents(format) {
            read(format, &document)
                .unwrap_or_else(|e| panic!("{format}: {what}: the reader refused: {e}"));
            let refused = refusals(&validator, format, &document);
            assert!(
                refused.is_empty(),
                "{format}: {what}: the schema refuses what the reader accepts:\n{}",
                refused.join("\n")
            );
        }
    }
}

/// The keys of an object, sorted.
fn keys(object: &Value) -> Vec<String> {
    let mut keys: Vec<String> = object
        .as_object()
        .unwrap_or_else(|| panic!("not an object: {object}"))
        .keys()
        .cloned()
        .collect();
    keys.sort();
    keys
}

/// The one schema written beside a decoder it cannot derive from: the `ekr.graph-document/2`
/// envelope a seed nests, which `ekr-store` decodes by hand. It names, requires and closes over
/// exactly the fields that envelope writes, at both levels, with the format it writes.
#[test]
fn the_seed_schemas_graph_document_is_the_envelope_the_store_writes() {
    let seed = SeedDocument::from_yaml(&text(&["example", SEED])).unwrap();
    let written = serde_json::to_value(&seed.graph).unwrap();
    let printed = schema(SEED)["properties"]["graph"].clone();
    for (level, written, printed) in [
        ("graph", &written, &printed),
        (
            "graph.graph",
            &written["graph"],
            &printed["properties"]["graph"],
        ),
    ] {
        assert_eq!(keys(&printed["properties"]), keys(written), "{level}");
        let mut required: Vec<String> = printed["required"]
            .as_array()
            .unwrap_or_else(|| panic!("{level}: nothing required"))
            .iter()
            .map(|key| key.as_str().unwrap().to_owned())
            .collect();
        required.sort();
        assert_eq!(required, keys(written), "{level}");
        assert_eq!(printed["additionalProperties"], false, "{level}");
    }
    assert_eq!(printed["properties"]["format"]["const"], written["format"]);
}

// 4 --------------------------------------------------------------------------------------------

/// JSON Schema has no YAML tag. Each YAML format's schema says, in its `description`, that it
/// spells `!Kind value` as the one-key object `{"!Kind": value}` and that the readers do not
/// accept that object in place of the tag; the host schema, for a JSON document, says neither.
/// The readers' refusal is held here, not only said: the object form, and the variant name as a
/// plain key, are refused where the tag is accepted.
#[test]
fn a_yaml_tag_is_spelled_as_a_one_key_object_the_readers_do_not_accept() {
    for format in [TRANSACTION, SEED] {
        let description = schema(format)["description"]
            .as_str()
            .unwrap_or_else(|| panic!("{format}: no description"))
            .to_owned();
        for needle in ["!Kind", "{\"!Kind\": value}", "do not accept"] {
            assert!(
                description.contains(needle),
                "{format}: the description does not say {needle:?}: {description}"
            );
        }
    }
    let host = schema(HOST)["description"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    assert!(!host.contains("!Kind"), "{host}");

    let example = text(&["example", TRANSACTION]);
    let tagged = "subject: !Node 00000000-0000-4000-8000-000000000301";
    read(TRANSACTION, &example).unwrap();
    for spelled in [
        "subject: {\"!Node\": 00000000-0000-4000-8000-000000000301}",
        "subject: {Node: 00000000-0000-4000-8000-000000000301}",
    ] {
        let document = edit(&example, tagged, spelled);
        assert!(
            read(TRANSACTION, &document).is_err(),
            "the transaction reader accepts {spelled}"
        );
    }
    let seed = text(&["example", SEED]);
    let document = edit(
        &seed,
        "source: !HumanStatement\n          identity: Runtime operator\n",
        "source: {\"!HumanStatement\": {identity: Runtime operator}}\n",
    );
    assert!(
        read(SEED, &document).is_err(),
        "the seed reader accepts the object form"
    );
}

// 5 --------------------------------------------------------------------------------------------

/// `ekr guide` and `ekr --help` name `ekr schema`, and it needs no store configuration: it exits
/// 0 whatever `EKR_*` holds.
#[test]
fn guide_and_help_name_schema_and_it_needs_no_configuration() {
    let guide = text(&["guide"]);
    assert!(guide.contains("ekr schema"), "{guide}");
    let help = text(&["--help"]);
    assert!(
        help.lines()
            .skip_while(|line| !line.starts_with("Commands:"))
            .any(|line| line.split_whitespace().next() == Some("schema")),
        "{help}"
    );
    for (var, value) in [
        ("EKR_BACKEND", "bogus"),
        ("EKR_STORE", ""),
        ("EKR_HOST", ""),
    ] {
        let output = ekr()
            .env(var, value)
            .args(["schema", SEED])
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(0), "{var}={value:?}");
    }
}
