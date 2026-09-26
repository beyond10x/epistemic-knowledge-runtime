//! Adversarial transaction-document profile probes; no store or proposal authority.
use ekr_core::ContentHash;
use ekr_kernel::{DocumentError, DocumentLimit, GraphOperation, TransactionDocument};
use ekr_ontology::Value;

const ID: &str = "00000000-0000-7000-8000-000000000001";
const PROPERTY: &str = "aaaaaaaa-aaaa-7aaa-8aaa-aaaaaaaaaaaa";

fn document(operations: &str) -> String {
    format!("format: ekr.transaction-document/1\ntransaction:\n  id: {ID}\n  proposer: {ID}\n  operations: [{operations}]\n  evidence: []\n")
}

fn node(name: &str) -> String {
    format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, canonical_name: {name}, properties: {{}}}}")
}

fn require_limit(input: &str, expected: DocumentLimit) {
    match TransactionDocument::parse(input.as_bytes()) {
        Err(DocumentError::Limit(actual, _)) => assert_eq!(actual, expected),
        Err(error) => panic!("expected {expected}, got {error}"),
        Ok(parsed) => panic!(
            "expected {expected}, accepted {} raw bytes and {} operations",
            input.len(),
            parsed.transaction().operations.len()
        ),
    }
}

#[test]
fn tags_and_typed_scalar_strings_share_one_expanded_string_budget() {
    let text = format!("0.{}", "0".repeat(32_766));
    let tag = format!("tag:ekr.test,2026:{}", "x".repeat(32_740));
    let operation = node(&format!("!<{tag}> {text}"));
    let direct: GraphOperation = serde_yaml_ng::from_str(&operation).unwrap();
    let GraphOperation::CreateNode(created) = direct else {
        panic!("wrong operation")
    };
    assert_eq!(created.canonical_name, text);
    let small = document(&operation);
    TransactionDocument::parse(small.as_bytes()).unwrap();
    let expanded = document(&format!(
        "&item {operation}, {}",
        vec!["*item"; 19].join(",")
    ));
    assert!(expanded.len() < 262_144);
    assert!((tag.len() + text.len()) * 20 > 1_048_576);
    if let Some(path) = std::env::var_os("EKR_ADVERSARY_INPUT") {
        std::fs::write(path, &expanded).unwrap();
    }
    require_limit(&expanded, DocumentLimit::TotalStringBytes);
}

#[test]
fn coerced_numeric_strings_charge_each_alias_without_tag_help() {
    let text = format!("0.{}", "0".repeat(65_534));
    let operation = node(&text);
    let small = document(&operation);
    TransactionDocument::parse(small.as_bytes()).unwrap();
    let expanded = document(&format!(
        "&item {operation}, {}",
        vec!["*item"; 16].join(",")
    ));
    assert!(expanded.len() < 262_144);
    require_limit(&expanded, DocumentLimit::TotalStringBytes);
}

#[test]
fn supported_scalar_and_record_spellings_keep_direct_typed_semantics() {
    for value in [
        "{value_kind: String, value: true}",
        "{value_kind: String, value: 0x10}",
        "{value_kind: String, value: .nan}",
        "{value_kind: Decimal, value: 1.2500}",
        "{value_kind: Record, value: {true: {value_kind: String, value: '&literal'}, 1: {value_kind: List, value: []}, '01': {value_kind: Record, value: {}}}}",
        "{value_kind: List, value: [{value_kind: Float, value: .inf}, {value_kind: String, value: '.inf'}]}",
    ] {
        let operation = format!("!UpdateProperty {{node: {ID}, property: {PROPERTY}, values: [{value}]}}");
        let direct: GraphOperation = serde_yaml_ng::from_str(&operation).unwrap();
        let input = document(&operation);
        let parsed = TransactionDocument::parse(input.as_bytes()).unwrap();
        assert_eq!(parsed.transaction().operations, [direct]);
        assert_eq!(parsed.bytes(), input.as_bytes());
        assert_eq!(parsed.hash(), ContentHash::of_bytes(input.as_bytes()));
        let GraphOperation::UpdateProperty(change) = &parsed.transaction().operations[0] else {
            panic!("wrong operation")
        };
        assert!(!matches!(change.values[0], Value::Float(value) if value.is_nan()));
    }
}

#[test]
fn duplicate_refusal_precedes_expansion_of_a_recursive_second_value() {
    let duplicate_record = "{value_kind: Record, value: {a: {value_kind: Integer, value: 1}, \"\\u0061\": &cycle [*cycle]}}";
    let operations = [
        format!("!UpdateProperty {{node: {ID}, property: {PROPERTY}, values: [{duplicate_record}]}}"),
        format!("!DefineNodeType {{id: {ID}, name: kind, operations: {{action: {{name: action}}, action: &cycle [*cycle]}}}}"),
        format!("!Invoke {{node: {ID}, operation: action, arguments: {{true: {{value_kind: Integer, value: 1}}, 'true': &cycle [*cycle]}}}}"),
    ];
    for operation in operations {
        let error = TransactionDocument::parse(document(&operation).as_bytes()).unwrap_err();
        assert!(
            matches!(&error, DocumentError::InvalidDocument(message) if message.contains("duplicate decoded mapping key")),
            "duplicate priority lost: {error}"
        );
    }
}

#[test]
fn unsupported_operation_nested_containers_keep_required_and_optional_shapes() {
    let valid = format!("!DefineNodeType {{id: {ID}, name: kind, lifecycle: {{initial: open, states: [open], transitions: []}}, operations: {{act: {{name: act, arguments: {{}}, preconditions: [], emits: [], transition: null}}}}, properties: {{}}}}");
    TransactionDocument::parse(document(&valid).as_bytes()).unwrap();
    for (from, to) in [
        ("states: [open]", "states: null"),
        ("transitions: []", "transitions:"),
        ("arguments: {}", "arguments: null"),
        ("preconditions: []", "preconditions:"),
        ("emits: []", "emits: {}"),
        ("properties: {}", "properties:"),
    ] {
        let changed = valid.replace(from, to);
        assert!(
            matches!(
                TransactionDocument::parse(document(&changed).as_bytes()),
                Err(DocumentError::InvalidDocument(_))
            ),
            "required container accepted {from} -> {to}"
        );
    }
}
