//! Second independent attack of the frozen document profile and observation boundary.
use ekr_core::ContentHash;
use ekr_kernel::{DocumentError, DocumentLimit, GraphOperation, TransactionDocument};

const ID: &str = "00000000-0000-7000-8000-000000000001";
const PROPERTY: &str = "aaaaaaaa-aaaa-7aaa-8aaa-aaaaaaaaaaaa";

fn document(operations: &str) -> String {
    format!("format: ekr.transaction-document/1\ntransaction: {{id: {ID}, proposer: {ID}, operations: [{operations}], evidence: []}}\n")
}

fn node(name: &str) -> String {
    format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, canonical_name: {name}, properties: {{}}}}")
}

fn update(value: &str) -> String {
    format!("!UpdateProperty {{node: {ID}, property: {PROPERTY}, values: [{value}]}}")
}

fn require_limit(input: &str, expected: DocumentLimit) {
    match TransactionDocument::parse(input.as_bytes()) {
        Err(DocumentError::Limit(actual, _)) => assert_eq!(actual, expected),
        Err(other) => panic!("expected {expected}, received {other}"),
        Ok(_) => panic!("expected {expected}, accepted {} raw bytes", input.len()),
    }
}

#[test]
fn directive_expanded_tags_and_aliases_share_an_exact_total() {
    let prefix = format!("tag:ekr.test,2026:{}", "x".repeat(31_978));
    let tag_bytes = prefix.len() + "name".len();
    assert_eq!(tag_bytes, 32_000);
    let header = [
        "format",
        "ekr.transaction-document/1",
        "transaction",
        "id",
        ID,
        "proposer",
        ID,
        "operations",
        "evidence",
    ]
    .iter()
    .map(|s| s.len())
    .sum::<usize>();
    let fields = [
        "CreateNode",
        "id",
        ID,
        "root_id",
        ID,
        "type_id",
        ID,
        "canonical_name",
        "properties",
    ]
    .iter()
    .map(|s| s.len())
    .sum::<usize>();
    let remaining = 1_048_576 - header - 32 * (fields + tag_bytes) - 31;
    assert!(remaining < 65_536);
    let input = |extra: usize| {
        let operations = format!(
            "&first {}, {}, {}",
            node("!m!name a"),
            vec!["*first"; 30].join(","),
            node(&format!("!m!name {}", "b".repeat(remaining + extra)))
        );
        format!("%TAG !m! {prefix}\n---\n{}", document(&operations))
    };
    let exact = input(0);
    let parsed = TransactionDocument::parse(exact.as_bytes()).unwrap();
    assert_eq!(parsed.transaction().operations.len(), 32);
    assert_eq!(parsed.bytes(), exact.as_bytes());
    assert_eq!(parsed.hash(), ContentHash::of_bytes(exact.as_bytes()));
    require_limit(&input(1), DocumentLimit::TotalStringBytes);
}

#[test]
fn percent_decoded_utf8_tag_bytes_have_their_own_inclusive_limit() {
    let prefix = "tag:ekr.test,2026:";
    let remaining = 65_536 - prefix.len();
    let tag = format!(
        "{prefix}{}{}",
        "%C3%A9".repeat(remaining / 2),
        "x".repeat(remaining % 2)
    );
    let exact = document(&node(&format!("!<{tag}> text")));
    assert!(exact.len() < 262_144);
    let parsed = TransactionDocument::parse(exact.as_bytes()).unwrap();
    assert_eq!(parsed.bytes(), exact.as_bytes());
    require_limit(
        &document(&node(&format!("!<{tag}x> text"))),
        DocumentLimit::StringBytes,
    );
}

#[test]
fn tagged_alias_keys_refuse_duplicates_before_malformed_values() {
    let values = [
        "{value_kind: Record, value: {? &key !<tag:ekr.test,2026:key> 'é' : {value_kind: Integer, value: 1}, ? *key : *missing}}",
        "{value_kind: Record, value: {? &key !local true : {value_kind: Integer, value: 1}, ? *key : &cycle [*cycle]}}",
    ];
    for value in values {
        let error = TransactionDocument::parse(document(&update(value)).as_bytes()).unwrap_err();
        assert!(
            matches!(&error, DocumentError::InvalidDocument(message) if message.contains("duplicate decoded mapping key")),
            "{error}"
        );
    }
    let good = "{value_kind: Record, value: {? !local true : {value_kind: Integer, value: 1}, 'True': {value_kind: Integer, value: 2}}}";
    let operation = update(good);
    let direct: GraphOperation = serde_yaml_ng::from_str(&operation).unwrap();
    let parsed = TransactionDocument::parse(document(&operation).as_bytes()).unwrap();
    assert_eq!(parsed.transaction().operations, [direct]);
}

#[test]
fn reordered_enum_content_matches_the_shared_typed_decoder() {
    let values = [
        "{value: 42, value_kind: Integer}",
        "{value: !!str 42, value_kind: Integer}",
        "{value: '42', value_kind: String}",
        "{value: 42, value_kind: String}",
        "{value: [ {value: 'x', value_kind: String} ], value_kind: List}",
        "{value: !metadata [], value_kind: List}",
        "{value: null, value_kind: List}",
        "{value: {true: {value: 7, value_kind: Integer}}, value_kind: Record}",
    ];
    let mut admitted = 0;
    let mut refused = 0;
    for value in values {
        let operation = update(value);
        let direct = serde_yaml_ng::from_str::<GraphOperation>(&operation);
        let parsed = TransactionDocument::parse(document(&operation).as_bytes());
        match (direct, parsed) {
            (Ok(expected), Ok(actual)) => {
                assert_eq!(actual.transaction().operations, [expected], "{value}");
                admitted += 1;
            }
            (Err(_), Err(DocumentError::InvalidDocument(_))) => refused += 1,
            (expected, actual) => {
                panic!("typed parity lost for {value}: direct={expected:?}, bounded={actual:?}")
            }
        }
    }
    assert!(admitted >= 3);
    assert!(refused >= 1);
}

#[test]
fn assertion_object_enum_boundaries_contribute_semantic_depth() {
    let input = |leaf: &str| {
        let mut value = leaf.to_owned();
        for _ in 0..12 {
            value = format!("{{value_kind: List, value: [{value}]}}");
        }
        let operation = format!("!AddAssertion {{id: {ID}, root_id: {ID}, subject: !Node {ID}, predicate: !Property {PROPERTY}, object: !Value {value}, evidence: [], proposed_by: {ID}, assessment: Proposed, lifecycle: Active, valid_time: {{from: 0, to: null}}, transaction_time: {{recorded_from: 0, recorded_to: null}}}}");
        document(&operation)
    };
    // Root, transaction, operations, operation enum, operation map, object enum:
    // the thirteen List maps/sequences then occupy depths 7 through 32.
    TransactionDocument::parse(input("{value_kind: List, value: []}").as_bytes()).unwrap();
    require_limit(
        &input("{value_kind: List, value: [{value_kind: Integer, value: 0}]}"),
        DocumentLimit::Depth,
    );
}

#[test]
fn unknown_alias_after_an_otherwise_complete_document_never_disappears() {
    let valid = document(&node("item"));
    for suffix in ["---\n*missing\n", "---\n[unterminated\n"] {
        require_limit(&format!("{valid}{suffix}"), DocumentLimit::Documents);
    }
    let invalid = document(&node("item")).replace("properties: {}", "properties: {*missing: []}");
    assert!(matches!(
        TransactionDocument::parse(invalid.as_bytes()),
        Err(DocumentError::InvalidDocument(_))
    ));
}
