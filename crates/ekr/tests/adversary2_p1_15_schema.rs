//! Adversary pass 2 on `story:machine-readable-document-schemas`, against correction 1f9f663.
//!
//! The correction's claim: the printed schemas carry the frozen transaction-document/1 limits,
//! accept what the readers accept, and name every remaining gap in their `description`. Each case
//! here drives the real reader and the printed schema on a document outside the named gaps, or
//! holds a printed description or the CLI page to what the binary does.

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

fn printed(format: &str) -> Value {
    serde_json::from_str(&stdout(&["schema", format])).unwrap()
}

fn validator(format: &str) -> jsonschema::Validator {
    jsonschema::draft202012::new(&printed(format)).unwrap()
}

/// The projection the schemas' own description defines: the YAML read as plain YAML, as JSON.
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

fn truncate(text: &str) -> String {
    text.chars().take(160).collect()
}

/// Every `(what, document)` on which the reader and the schema disagree, described.
fn disagreements(format: &str, documents: &[(&str, String)]) -> Vec<String> {
    let validator = validator(format);
    let mut out = Vec::new();
    for (what, document) in documents {
        let reader = read(format, document);
        let schema: Vec<String> = validator
            .iter_errors(&instance(format, document))
            .map(|e| format!("{}: {}", e.instance_path(), truncate(&e.to_string())))
            .take(2)
            .collect();
        if reader.is_ok() != schema.is_empty() {
            let reader = match reader {
                Ok(()) => "reader ACCEPTS".to_owned(),
                Err(e) => format!("reader REFUSES ({})", truncate(&e)),
            };
            let schema = if schema.is_empty() {
                "schema ACCEPTS".to_owned()
            } else {
                format!("schema REFUSES ({})", schema.join("; "))
            };
            out.push(format!("{format}: {what}: {reader}, {schema}"));
        }
    }
    out
}

fn assert_no_disagreement(found: &[String]) {
    assert!(
        found.is_empty(),
        "the schema and the reader disagree:\n{}",
        found.join("\n")
    );
}

/// A disagreement the printed `description` names is a documented gap, not a defect. The branch
/// holds that the description names `phrase`, that at least one document disagrees (so the gap is
/// real), and that every disagreement runs the way the description says: `reader_accepts` true for
/// "the readers accept and this schema refuses", false for "the readers refuse, and this schema
/// cannot see".
fn assert_documented_gap(
    format: &str,
    phrase: &str,
    reader_accepts: bool,
    documents: &[(&str, String)],
) {
    let description = printed(format)["description"]
        .as_str()
        .unwrap_or("")
        .to_owned();
    assert!(
        description.contains(phrase),
        "{format}: the description does not name {phrase:?}, so this is no documented gap"
    );
    let found = disagreements(format, documents);
    assert!(
        !found.is_empty(),
        "{format}: no document disagrees, so {phrase:?} names a gap that is not there"
    );
    let documented = if reader_accepts {
        "reader ACCEPTS"
    } else {
        "reader REFUSES"
    };
    let other_way: Vec<String> = found
        .into_iter()
        .filter(|disagreement| !disagreement.contains(documented))
        .collect();
    assert_no_disagreement(&other_way);
}

const ID: &str = "00000000-0000-4000-8000-000000000601";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";

fn transaction(operations: &str) -> String {
    format!(
        "format: {TRANSACTION}\ntransaction:\n  id: {ID}\n  proposer: {OPERATOR}\n  operations:{operations}\n  evidence: []\n"
    )
}

fn create_node(canonical_name: &str) -> String {
    format!(
        "\n  - !CreateNode\n    id: 00000000-0000-4000-8000-000000000304\n    root_id: \
         00000000-0000-4000-8000-000000000002\n    type_id: 00000000-0000-4000-8000-000000000202\n    \
         canonical_name: {canonical_name}\n    properties: {{}}"
    )
}

/// An explicit (`? key`) mapping key: YAML allows an implicit key of at most 1024 characters.
fn invoke_with_argument(key: &str) -> String {
    format!(
        "\n  - !Invoke\n    node: 00000000-0000-4000-8000-000000000303\n    operation: \
         record_review\n    arguments:\n      ? {key}\n      : {{value_kind: String, value: x}}"
    )
}

// A ----------------------------------------------------------------------------------------------

/// `DOCUMENT_V1_LIMITS.string_bytes` and `key_bytes` are **bytes** (`Budget::string_with_credit`
/// compares `value.len()`); the schema writes them as `maxLength` / `propertyNames.maxLength`,
/// which JSON Schema counts in characters. Text outside ASCII passes the schema at up to four
/// times the bytes the reader takes. The description says "the schema carries the per-string
/// limits"; it names no byte/character difference.
#[test]
fn a_string_over_the_byte_limit_in_fewer_characters_is_refused_by_both() {
    let documents = vec![
        (
            "canonical_name of 40000 'é' (80000 bytes, 40000 characters)",
            transaction(&create_node(&"é".repeat(40_000))),
        ),
        (
            "Invoke argument key of 3000 'é' (6000 bytes, 3000 characters)",
            transaction(&invoke_with_argument(&"é".repeat(3_000))),
        ),
    ];
    assert_documented_gap(
        TRANSACTION,
        "a string or key within maxLength characters but over the byte limit",
        false,
        &documents,
    );
}

// B ----------------------------------------------------------------------------------------------

/// The description names "a tag on a scalar" as the one tag the readers ignore. serde_yaml_ng
/// ignores a local tag on a mapping or a sequence read into a struct, list or map too; the
/// projection spells it as a one-key object no struct/list/map schema admits.
#[test]
fn a_tag_on_a_mapping_or_a_list_is_judged_alike_by_both() {
    let example = stdout(&["example", TRANSACTION]);
    let transactions = vec![
        (
            "valid_time: !Range {from, to}",
            example.replacen("valid_time:\n", "valid_time: !Range\n", 1),
        ),
        (
            "assertion evidence: !Set [..]",
            example.replacen("    evidence:\n    - ", "    evidence: !Set\n    - ", 1),
        ),
        (
            "CreateNode properties: !Map {}",
            transaction(&create_node("Globex")).replacen(
                "properties: {}",
                "properties: !Map {}",
                1,
            ),
        ),
    ];
    assert_documented_gap(
        TRANSACTION,
        "a tag on a scalar, a mapping or a list",
        true,
        &transactions,
    );
    let seed = stdout(&["example", SEED]);
    let seeds = vec![
        (
            "node aliases: !List []",
            seed.replacen("aliases: []", "aliases: !List []", 1),
        ),
        (
            "ontology version: !Version {..}",
            seed.replacen("  version:\n", "  version: !Version\n", 1),
        ),
    ];
    assert_documented_gap(SEED, "a tag on a scalar, a mapping or a list", true, &seeds);
}

// C ----------------------------------------------------------------------------------------------

/// `unique_set` over `BTreeSet<String>` compares the decoded **text**; the schema's `uniqueItems`
/// compares the projected JSON **values**. Two plain scalars with different text that YAML
/// resolves to one value (`1` and `01`, `true` and `True`, `1` and `1.0`) are distinct to the
/// reader and duplicates to the schema. The description names only the other direction ("the
/// readers refuse, and this schema cannot see ... the same text written once plain and once
/// quoted").
#[test]
fn distinct_texts_yaml_resolves_to_one_value_are_judged_alike_by_both() {
    let seed = stdout(&["example", SEED]);
    let string_type = "        value_type:\n          value_kind: String\n";
    let variants = |list: &str| {
        seed.replacen(
            string_type,
            &format!(
                "        value_type:\n          value_kind: Enum\n          parameters:\n            variants: {list}\n"
            ),
            1,
        )
    };
    let documents = vec![
        ("Enum variants [1, 01]", variants("[1, 01]")),
        ("Enum variants [true, True]", variants("[true, True]")),
        ("Enum variants [1, 1.0]", variants("[1, 1.0]")),
    ];
    assert_documented_gap(
        SEED,
        "two texts YAML resolves to one value",
        true,
        &documents,
    );
}

// D ----------------------------------------------------------------------------------------------

/// The correction strips the derived descriptions from the host schema. Before it, the schema
/// said `Exactly \`ekr.authority-state/1\`` and so on for each fixed text the host's authority
/// check (`AuthorityStateV1::check`) requires; after it, those fields are `{"type": "string"}`
/// with nothing said, so a host document the schema passes is refused by every store verb
/// (`seed-authority-profile`). The printed schema must name each value, as a `const` or in words.
#[test]
fn the_host_schema_names_every_fixed_text_the_host_check_requires() {
    let out = stdout(&["schema", HOST]);
    let missing: Vec<&str> = [
        "ekr.authority-state/1",
        "ekr.p1-validation-profile/1",
        "ekr.p1-deterministic/1",
        "distinct-authenticated-actor/1",
        "retained-admissible-evidence/1",
        "ekr.p1-apply/1",
    ]
    .into_iter()
    .filter(|text| !out.contains(text))
    .collect();
    assert!(
        missing.is_empty(),
        "the host schema accepts any string where the host check requires: {missing:?}"
    );
}

/// Every id definition's description says "`ekr mint <kind>` prints a new one". That holds only
/// for ids `ekr mint` has a kind for.
#[test]
fn every_id_the_schema_says_ekr_mint_prints_has_a_mint_kind() {
    let kinds = [
        ("NodeId", "node"),
        ("EdgeId", "edge"),
        ("AssertionId", "assertion"),
        ("TransactionId", "transaction"),
        ("EvidenceId", "evidence"),
        ("TypeId", "type"),
        ("PropertyId", "property"),
        ("AgentId", "agent"),
        ("GraphRootId", "graph-root"),
        ("SchemaVersionId", "schema-version"),
    ];
    let mut wrong = Vec::new();
    for format in [TRANSACTION, SEED] {
        let schema = printed(format);
        for (name, definition) in schema["$defs"].as_object().unwrap() {
            let says_mint = definition["description"]
                .as_str()
                .is_some_and(|d| d.contains("ekr mint"));
            if says_mint && !kinds.iter().any(|(id, _)| id == name) {
                wrong.push(format!("{format}: {name}"));
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "these ids are described as printed by `ekr mint <kind>`, which has no kind for them: {wrong:?}"
    );
}

// E ----------------------------------------------------------------------------------------------

/// docs/cli.md `### ekr schema` tells a reader to "check a document before `ekr seed` or `ekr
/// propose`" and lists what the schema cannot see: a key written twice, an inverted range and
/// `1.0`. The printed description, after the correction, names more — and two of them run the
/// other way, the schema refusing a document the reader accepts. The page must name each.
#[test]
fn the_cli_page_names_each_gap_the_printed_description_names() {
    let root = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the manifest directory"),
    );
    let page = std::fs::read_to_string(root.join("../../docs/cli.md")).unwrap();
    let start = page
        .find("### `ekr schema`")
        .expect("docs/cli.md has an ekr schema section");
    let section = &page[start..];
    let section = &section[..section[4..].find("\n#").map_or(section.len(), |at| at + 4)];
    let description = printed(TRANSACTION)["description"]
        .as_str()
        .unwrap()
        .to_owned();
    let mut missing = Vec::new();
    for (gap, in_description, on_page) in [
        (
            "a tag on a scalar",
            "a tag on a scalar",
            &["tag on a scalar"][..],
        ),
        ("a Float .nan / .inf", "`.nan` or `.inf`", &[".nan"][..]),
        (
            "a uniqueItems repeat",
            "uniqueItems",
            &["uniqueItems", "twice in a list"][..],
        ),
        (
            "value before value_kind",
            "before its `value_kind`",
            &["value_kind"][..],
        ),
        (
            "whole-document limits",
            "262144",
            &["262144", "262 144", "256 KiB"][..],
        ),
    ] {
        assert!(
            description.contains(in_description),
            "the description no longer names {gap}"
        );
        if !on_page.iter().any(|phrase| section.contains(phrase)) {
            missing.push(gap);
        }
    }
    assert!(
        missing.is_empty(),
        "docs/cli.md `ekr schema` does not name these gaps the printed schema names: {missing:?}\n{section}"
    );
}
