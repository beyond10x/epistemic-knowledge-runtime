//! Executable contract for the original bounded transaction document.
use std::io::{self, Read};

use ekr_core::ContentHash;
use ekr_kernel::{DocumentError, DocumentLimit, GraphOperation, TransactionDocument};
use ekr_ontology::Value;

const ID: &str = "00000000-0000-7000-8000-000000000001";
const PROPERTY: &str = "aaaaaaaa-aaaa-7aaa-8aaa-aaaaaaaaaaaa";

fn document(operation: &str) -> String {
    format!("format: ekr.transaction-document/1\ntransaction:\n  id: {ID}\n  proposer: {ID}\n  operations:\n    - {operation}\n  evidence: []\n")
}
fn update(value: &str) -> String {
    document(&format!(
        "!UpdateProperty {{node: {ID}, property: {PROPERTY}, values: [{value}]}}"
    ))
}
fn string_value(text: &str) -> String {
    format!("{{value_kind: String, value: {text}}}")
}
fn refused_as(input: &[u8], limit: DocumentLimit) {
    assert!(
        matches!(TransactionDocument::parse(input), Err(DocumentError::Limit(actual)) if actual == limit)
    );
}

#[test]
fn exact_original_bytes_and_payload_domain_hash_are_retained() {
    let original = format!(
        "---\r\n# input\r\n{}...\r\n# end\r\n",
        update(&string_value("42")).replace('\n', "\r\n")
    );
    let parsed = TransactionDocument::parse(original.as_bytes()).unwrap();
    assert_eq!(parsed.bytes(), original.as_bytes());
    assert_eq!(parsed.hash(), ContentHash::of_bytes(original.as_bytes()));
    let GraphOperation::UpdateProperty(change) = &parsed.transaction().operations[0] else {
        panic!("wrong operation")
    };
    assert_eq!(change.values, vec![Value::String("42".into())]);
}

#[test]
fn nonfinite_floats_survive_without_json_round_tripping() {
    for spelling in [".nan", ".NaN", ".NAN", ".inf", "-.Inf", "+.INF"] {
        let input = update(&format!("{{value_kind: Float, value: {spelling}}}"));
        let parsed = TransactionDocument::parse(input.as_bytes()).unwrap();
        assert_eq!(parsed.bytes(), input.as_bytes());
        let GraphOperation::UpdateProperty(change) = &parsed.transaction().operations[0] else {
            panic!("wrong operation")
        };
        assert!(matches!(change.values[0], Value::Float(v) if !v.is_finite()));
    }
    let parsed = TransactionDocument::parse(update(&string_value("'.nan'")).as_bytes()).unwrap();
    let GraphOperation::UpdateProperty(change) = &parsed.transaction().operations[0] else {
        panic!("wrong operation")
    };
    assert_eq!(change.values, [Value::String(".nan".into())]);
}

#[test]
fn arbitrary_record_keys_are_data_and_explicit_empty_containers_survive() {
    let record = "{value_kind: Record, value: {format: &v {value_kind: List, value: []}, transaction: *v, operations: *v, value_kind: *v, '<<': *v, true: *v, '': *v, 'λ': *v, '*&': {value_kind: Record, value: {}}}}";
    TransactionDocument::parse(update(record).as_bytes()).unwrap();
}

#[test]
fn duplicate_keys_and_unknown_fields_are_not_ignored() {
    let valid = update(&string_value("ok"));
    for input in [
        format!("format: ekr.transaction-document/1\n{valid}"),
        valid.replace("  id:", &format!("  id: {ID}\n  id:")),
        valid.replace("node:", &format!("node: {ID}, node:")),
        valid.replace(
            "value_kind: String",
            "value_kind: String, value_kind: String",
        ),
        valid.replace("format:", "unknown: true\nformat:"),
        valid.replace("  id:", "  unknown: true\n  id:"),
        valid.replace("node:", "unknown: true, node:"),
        valid.replace("value_kind: String", "unknown: true, value_kind: String"),
        update(
            "{value_kind: Record, value: {a: &v {value_kind: String, value: ok}, \"\\u0061\": *v}}",
        ),
        update(
            "{value_kind: Record, value: {true: &v {value_kind: String, value: ok}, 'true': *v}}",
        ),
    ] {
        assert!(
            TransactionDocument::parse(input.as_bytes()).is_err(),
            "accepted {input}"
        );
    }
}

#[test]
fn required_collections_refuse_null_and_wrong_nesting() {
    for value in [
        "{value_kind: List, value:}",
        "{value_kind: List, value: null}",
        "{value_kind: List, value: {}}",
        "{value_kind: Record, value:}",
        "{value_kind: Record, value: null}",
        "{value_kind: Record, value: []}",
    ] {
        assert!(
            TransactionDocument::parse(update(value).as_bytes()).is_err(),
            "accepted {value}"
        );
    }
    for collection in ["", "null", "{}"] {
        let input = document(&format!(
            "!UpdateProperty {{node: {ID}, property: {PROPERTY}, values: {collection}}}"
        ));
        assert!(
            TransactionDocument::parse(input.as_bytes()).is_err(),
            "accepted {input}"
        );
    }
}

#[test]
fn only_one_utf8_document_and_the_named_format_are_admitted() {
    let valid = update(&string_value("ok"));
    for input in [
        format!("{valid}---\n"),
        format!("{valid}---\n{valid}"),
        String::new(),
        valid.replace("document/1", "document/2"),
    ] {
        assert!(TransactionDocument::parse(input.as_bytes()).is_err());
    }
    assert!(matches!(
        TransactionDocument::parse(&[255]),
        Err(DocumentError::InvalidUtf8)
    ));
    assert!(matches!(
        TransactionDocument::parse(valid.replace("document/1", "document/2").as_bytes()),
        Err(DocumentError::UnsupportedFormat(_))
    ));
}

#[test]
fn typed_yaml_tags_and_json_compatible_collection_syntax_are_distinct() {
    TransactionDocument::parse(update(&string_value("\"ok\"")).as_bytes()).unwrap();
    let object = document(&format!("{{DeleteEdge: {ID}}}"));
    assert!(TransactionDocument::parse(object.as_bytes()).is_err());
}

#[test]
fn original_document_byte_limit_is_inclusive_and_counts_utf8_bytes() {
    let limits: ekr_kernel::DocumentLimits = ekr_kernel::DOCUMENT_V1_LIMITS;
    assert_eq!(limits.input_bytes, 262_144);
    let mut exact = update(&string_value("é"));
    exact.push('#');
    exact.extend(std::iter::repeat_n('x', limits.input_bytes - exact.len()));
    TransactionDocument::parse(exact.as_bytes()).unwrap();
    exact.push('x');
    refused_as(exact.as_bytes(), DocumentLimit::InputBytes);
}

#[test]
fn a_host_upload_cap_does_not_redefine_historical_parsing() {
    let input = update(&string_value("ok"));
    assert!(matches!(
        TransactionDocument::read_with_upload_limit(input.as_bytes(), input.len() - 1),
        Err(DocumentError::Limit(DocumentLimit::UploadBytes))
    ));
    let retained = TransactionDocument::parse(input.as_bytes()).unwrap();
    assert_eq!(
        TransactionDocument::read(input.as_bytes()).unwrap().hash(),
        retained.hash()
    );
}

struct ShortReader {
    bytes: Vec<u8>,
    offset: usize,
    read_count: usize,
}
impl Read for ShortReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.read_count += 1;
        if self.read_count == 1 {
            return Err(io::ErrorKind::Interrupted.into());
        }
        let count = buffer.len().min(3).min(self.bytes.len() - self.offset);
        buffer[..count].copy_from_slice(&self.bytes[self.offset..self.offset + count]);
        self.offset += count;
        Ok(count)
    }
}
#[test]
fn read_ingress_handles_short_reads_and_stops_after_limit_plus_one() {
    let input = update(&string_value("ok"));
    let mut reader = ShortReader {
        bytes: input.as_bytes().to_vec(),
        offset: 0,
        read_count: 0,
    };
    assert_eq!(
        TransactionDocument::read(&mut reader).unwrap().bytes(),
        input.as_bytes()
    );
    let mut reader = ShortReader {
        bytes: vec![b' '; 262_200],
        offset: 0,
        read_count: 0,
    };
    assert!(matches!(
        TransactionDocument::read(&mut reader),
        Err(DocumentError::Limit(DocumentLimit::InputBytes))
    ));
    assert_eq!(reader.offset, 262_145);
}

#[test]
fn decoded_string_and_key_limits_are_inclusive_in_bytes() {
    TransactionDocument::parse(update(&string_value(&"é".repeat(32_768))).as_bytes()).unwrap();
    refused_as(
        update(&string_value(&format!("{}a", "é".repeat(32_768)))).as_bytes(),
        DocumentLimit::StringBytes,
    );
    let record = |key: String| {
        update(&format!(
            "{{value_kind: Record, value: {{? '{key}' : {{value_kind: Integer, value: 1}}}}}}"
        ))
    };
    TransactionDocument::parse(record("é".repeat(2_048)).as_bytes()).unwrap();
    refused_as(
        record(format!("{}a", "é".repeat(2_048))).as_bytes(),
        DocumentLimit::KeyBytes,
    );
}

#[test]
fn operations_and_input_evidence_have_independent_inclusive_limits() {
    let repeated = |count: usize| {
        format!("format: ekr.transaction-document/1\ntransaction: {{id: {ID}, proposer: {ID}, operations: [{}], evidence: []}}", vec![format!("!DeleteEdge {ID}"); count].join(","))
    };
    TransactionDocument::parse(repeated(256).as_bytes()).unwrap();
    refused_as(repeated(257).as_bytes(), DocumentLimit::Operations);
    refused_as(repeated(0).as_bytes(), DocumentLimit::EmptyOperations);
    let evidence = |count: usize| {
        document(&format!("!DeleteEdge {ID}")).replace(
            "evidence: []",
            &format!("evidence: [{}]", vec![ID; count].join(",")),
        )
    };
    assert_eq!(
        TransactionDocument::parse(evidence(1_024).as_bytes())
            .unwrap()
            .transaction()
            .evidence
            .len(),
        1
    );
    refused_as(evidence(1_025).as_bytes(), DocumentLimit::Evidence);
}

#[test]
fn unknown_and_recursive_aliases_refuse_without_process_failure() {
    for value in [
        "*absent",
        "&recursive {value_kind: List, value: [*recursive]}",
    ] {
        assert!(TransactionDocument::parse(update(value).as_bytes()).is_err());
    }
}

#[test]
fn explicit_enum_boundaries_and_alias_positions_count_toward_depth() {
    let nested = |leaf: &str| {
        let mut value = leaf.to_owned();
        for _ in 0..12 {
            value = format!("{{value_kind: List, value: [{value}]}}");
        }
        update(&value)
    };
    // Root(1), transaction(2), operations(3), tag(4), operation(5), values(6),
    // then thirteen Value maps and thirteen List sequences: deepest container 32.
    TransactionDocument::parse(nested("{value_kind: List, value: []}").as_bytes()).unwrap();
    refused_as(
        nested("{value_kind: List, value: [{value_kind: Integer, value: 1}]}").as_bytes(),
        DocumentLimit::Depth,
    );
    // The anchored leaf first occurs shallower; its second occurrence must use
    // its expanded position, rather than the anchor's original depth.
    let mut value = "*leaf".to_owned();
    for _ in 0..13 {
        value = format!("{{value_kind: List, value: [{value}]}}");
    }
    let input = document(&format!("!UpdateProperty {{node: {ID}, property: {PROPERTY}, values: [&leaf {{value_kind: Integer, value: 1}}, {value}]}}"));
    refused_as(input.as_bytes(), DocumentLimit::Depth);
}

#[test]
fn one_mapping_and_sequence_have_independent_inclusive_limits() {
    let record = |count| {
        let entries = (1..count)
            .map(|i| format!("k{i}: *v"))
            .collect::<Vec<_>>()
            .join(",");
        update(&format!("{{value_kind: Record, value: {{k0: &v {{value_kind: Integer, value: 1}}, {entries}}}}}"))
    };
    TransactionDocument::parse(record(4_096).as_bytes()).unwrap();
    refused_as(record(4_097).as_bytes(), DocumentLimit::MappingEntries);
    let list = |count: usize| {
        update(&format!(
            "{{value_kind: List, value: [&v {{value_kind: Integer, value: 1}}, {}]}}",
            vec!["*v"; count - 1].join(",")
        ))
    };
    TransactionDocument::parse(list(4_096).as_bytes()).unwrap();
    refused_as(list(4_097).as_bytes(), DocumentLimit::SequenceElements);
}

#[test]
fn expanded_node_boundary_counts_every_alias_occurrence() {
    let input = |one_more_key: bool| {
        let left = vec!["*v"; 3_272].join(",");
        let right = vec!["*v"; if one_more_key { 3_272 } else { 3_273 }].join(",");
        let extra = if one_more_key { ", c: *v" } else { "" };
        update(&format!("{{value_kind: Record, value: {{a: {{value_kind: List, value: [&v {{value_kind: Integer, value: 1}}, {left}]}}, b: {{value_kind: List, value: [{right}]}}{extra}}}}}"))
    };
    // 21 surrounding nodes + 6,549 Value records * 5 + two Record keys.
    // Moving one integer to a third Record key adds exactly one syntax node.
    TransactionDocument::parse(input(false).as_bytes()).unwrap();
    refused_as(input(true).as_bytes(), DocumentLimit::Nodes);
}

#[test]
fn expanded_string_budget_is_inclusive_and_not_reset_at_an_alias() {
    let surrounding = [
        "format",
        "ekr.transaction-document/1",
        "transaction",
        "id",
        ID,
        "proposer",
        ID,
        "operations",
        "UpdateProperty",
        "node",
        ID,
        "property",
        PROPERTY,
        "values",
        "value_kind",
        "List",
        "value",
        "evidence",
    ]
    .iter()
    .map(|text| text.len())
    .sum::<usize>();
    // Each String Value contributes its two keys (15 bytes), discriminant (6),
    // and value. Fifteen aliases carry full-length strings; the final one fills
    // exactly the remaining budget. The raw document remains below 256 KiB.
    let remaining = 1_048_576 - surrounding - 16 * 21 - 15 * 65_536;
    let input = |tail| {
        update(&format!(
            "{{value_kind: List, value: [&s {}, {}, {}]}}",
            string_value(&"x".repeat(65_536)),
            vec!["*s"; 14].join(","),
            string_value(&"x".repeat(tail))
        ))
    };
    TransactionDocument::parse(input(remaining).as_bytes()).unwrap();
    refused_as(
        input(remaining + 1).as_bytes(),
        DocumentLimit::TotalStringBytes,
    );
}

#[test]
fn duplicate_typed_ids_are_refused_before_decoding_the_duplicate_value() {
    let upper = PROPERTY;
    let property = format!("{{id: {PROPERTY}, name: field, value_type: {{value_kind: String}}}}");
    let properties = format!("{{{PROPERTY}: {property}, {upper}: not-a-property}} ");
    for operation in [
        format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, canonical_name: item, properties: {{{PROPERTY}: [], {upper}: not-a-sequence}}}}"),
        format!("!CreateEdge {{id: {ID}, root_id: {ID}, type_id: {ID}, source: {ID}, target: {ID}, properties: {{{PROPERTY}: [], {upper}: not-a-sequence}}}}"),
        format!("!DefineNodeType {{id: {ID}, name: kind, properties: {properties}}}"),
        format!("!DefineEdgeType {{id: {ID}, name: relation, properties: {properties}}}"),
    ] {
        let input = document(&operation);
        let refusal = TransactionDocument::parse(input.as_bytes()).unwrap_err().to_string();
        assert!(refusal.contains("duplicate decoded mapping key"), "{refusal}");
        let error = serde_yaml_ng::from_str::<GraphOperation>(&operation).unwrap_err().to_string();
        assert!(error.contains("duplicate decoded map key"), "{error}");
    }
}

#[test]
fn text_that_looks_like_an_id_remains_a_distinct_record_key() {
    let value = format!(
        "{{value_kind: Record, value: {{{PROPERTY}: &v {{value_kind: List, value: []}}, {}: *v}}}}",
        PROPERTY.to_uppercase()
    );
    let parsed = TransactionDocument::parse(update(&value).as_bytes()).unwrap();
    let GraphOperation::UpdateProperty(change) = &parsed.transaction().operations[0] else {
        panic!("wrong operation")
    };
    assert!(matches!(&change.values[0], Value::Record(fields) if fields.len() == 2));
}

#[test]
fn nested_ontology_carriers_remain_strict_even_for_unsupported_operations() {
    let cases = [
        format!("!MergeEntity {{absorbed: {ID}, into: {ID}, unknown: true}}"),
        format!("!Invoke {{node: {ID}, operation: action, arguments: {{}}, unknown: true}}"),
        format!("!DefineNodeType {{id: {ID}, name: kind, unknown: true}}"),
        format!("!DefineEdgeType {{id: {ID}, name: relation, unknown: true}}"),
        format!("!ModifyProperty {{owner: {ID}, property: {{id: {PROPERTY}, name: field, value_type: {{value_kind: String}}}}, unknown: true}}"),
        format!("!ModifyProperty {{owner: {ID}, property: {{id: {PROPERTY}, name: field, value_type: {{value_kind: String}}, unknown: true}}}}"),
        format!("!DefineNodeType {{id: {ID}, name: kind, lifecycle: {{initial: open, states: [open], unknown: true}}}}"),
        format!("!DefineNodeType {{id: {ID}, name: kind, lifecycle: {{initial: open, states: [open], transitions: [{{from: open, to: open, unknown: true}}]}}}}"),
        format!("!DefineNodeType {{id: {ID}, name: kind, operations: {{action: {{name: action, unknown: true}}}}}}"),
        format!("!ModifyProperty {{owner: {ID}, property: {{id: {PROPERTY}, name: field, value_type: {{value_kind: List, parameters: {{value_kind: String, unknown: true}}}}}}}}"),
    ];
    for operation in cases {
        let error = TransactionDocument::parse(document(&operation).as_bytes())
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("unknown field") || error.contains("string \"unknown\""),
            "{error}"
        );
    }
}

#[test]
fn duplicate_lists_and_ordered_outer_property_values_survive() {
    let operation = format!("!UpdateProperty {{node: {ID}, property: {PROPERTY}, values: [{{value_kind: List, value: [&v {{value_kind: Integer, value: 7}}, *v]}}, {{value_kind: Integer, value: 3}}, {{value_kind: Integer, value: 3}}]}}");
    let parsed = TransactionDocument::parse(document(&operation).as_bytes()).unwrap();
    let GraphOperation::UpdateProperty(change) = &parsed.transaction().operations[0] else {
        panic!("wrong operation")
    };
    assert_eq!(
        change.values,
        [
            Value::List(vec![Value::Integer(7), Value::Integer(7)]),
            Value::Integer(3),
            Value::Integer(3)
        ]
    );
}

#[test]
fn only_the_canonical_id_spelling_is_admitted_and_yaml_escapes_do_not_hide_duplicates() {
    for alternate in [
        PROPERTY.to_uppercase(),
        PROPERTY.replace('-', ""),
        format!("{{{PROPERTY}}}"),
        format!("urn:uuid:{PROPERTY}"),
    ] {
        let input = document(&format!(
            "!UpdateProperty {{node: {ID}, property: '{alternate}', values: []}}"
        ));
        let error = TransactionDocument::parse(input.as_bytes())
            .unwrap_err()
            .to_string();
        assert!(error.contains("is not a UUID identifier"), "{error}");
    }
    let escaped = PROPERTY.replacen('a', "\\u0061", 1);
    let input = document(&format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, canonical_name: item, properties: {{{PROPERTY}: [], \"{escaped}\": not-a-sequence}}}}"));
    let error = TransactionDocument::parse(input.as_bytes())
        .unwrap_err()
        .to_string();
    assert!(error.contains("duplicate decoded mapping key"), "{error}");
}

#[test]
fn a_full_collection_refuses_before_expanding_the_next_child() {
    let input = update(&format!(
        "{{value_kind: List, value: [&v {{value_kind: Integer, value: 1}}, {}, &cycle [*cycle]]}}",
        vec!["*v"; 4_095].join(",")
    ));
    refused_as(input.as_bytes(), DocumentLimit::SequenceElements);
    let entries = (1..4_096)
        .map(|i| format!("k{i}: *v"))
        .collect::<Vec<_>>()
        .join(",");
    let input = update(&format!("{{value_kind: Record, value: {{k0: &v {{value_kind: Integer, value: 1}}, {entries}, ? '{}' : *v}}}}", "x".repeat(4_097)));
    refused_as(input.as_bytes(), DocumentLimit::MappingEntries);
}

#[test]
fn scalar_null_cannot_supply_top_level_or_schema_collections() {
    for operation in [
        format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, canonical_name: item, properties:}}"),
        format!("!CreateEdge {{id: {ID}, root_id: {ID}, type_id: {ID}, source: {ID}, target: {ID}, properties: null}}"),
        format!("!DefineNodeType {{id: {ID}, name: item, parents:}}"),
        format!("!DefineNodeType {{id: {ID}, name: item, operations:}}"),
        format!("!Invoke {{node: {ID}, operation: action, arguments:}}"),
    ] { assert!(TransactionDocument::parse(document(&operation).as_bytes()).is_err(), "{operation}"); }
    let valid = document(&format!("!DeleteEdge {ID}"));
    for suffix in ["evidence:", "evidence: null"] {
        assert!(
            TransactionDocument::parse(valid.replace("evidence: []", suffix).as_bytes()).is_err()
        );
    }
    let with_optional_null = document(&format!(
        "!DefineNodeType {{id: {ID}, name: item, lifecycle: null}}"
    ));
    TransactionDocument::parse(with_optional_null.as_bytes()).unwrap();
}

#[test]
fn assertion_payloads_preserve_typed_enums_and_checked_temporal_ranges() {
    let operation = format!("!AddAssertion {{id: {ID}, root_id: {ID}, subject: !Node {ID}, predicate: !Property {PROPERTY}, object: !Value {{value_kind: String, value: claim}}, evidence: [], proposed_by: {ID}, assessment: Proposed, lifecycle: Active, valid_time: {{from: 0, to: null}}, transaction_time: {{recorded_from: 0, recorded_to: null}}}}");
    let input = document(&operation);
    TransactionDocument::parse(input.as_bytes()).unwrap();
    for broken in [
        input.replace("subject:", "extra: true, subject:"),
        input.replace("from: 0, to: null", "from: 0, to: null, extra: true"),
        input.replace("recorded_to: null", "recorded_to: null, extra: true"),
        input.replace(
            "assessment: Proposed",
            "assessment: !Accepted {validators: [], extra: true}",
        ),
    ] {
        let error = TransactionDocument::parse(broken.as_bytes())
            .unwrap_err()
            .to_string();
        assert!(error.contains("unknown field"), "{error}");
    }
    for broken in [
        input.replace("from: 0, to: null", "from: 2, to: 1"),
        input.replace(
            "recorded_from: 0, recorded_to: null",
            "recorded_from: 2, recorded_to: 1",
        ),
    ] {
        let error = TransactionDocument::parse(broken.as_bytes())
            .unwrap_err()
            .to_string();
        assert!(error.contains("precedes its start"), "{error}");
    }
}

#[test]
fn a_second_document_has_a_named_refusal_even_when_it_is_empty() {
    let valid = document(&format!("!DeleteEdge {ID}"));
    refused_as(format!("{valid}---\n").as_bytes(), DocumentLimit::Documents);
    refused_as(
        format!("{valid}---\n{valid}").as_bytes(),
        DocumentLimit::Documents,
    );
}

#[test]
fn current_operation_payloads_use_the_shared_decoder_without_shape_loss() {
    let operations = [
        format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, canonical_name: 42, properties: {{}}}}"),
        format!("!UpdateProperty {{node: {ID}, property: {PROPERTY}, values: []}}"),
        format!("!CreateEdge {{id: {ID}, root_id: {ID}, type_id: {ID}, source: {ID}, target: {ID}, properties: {{}}}}"),
        format!("!DeleteEdge {ID}"),
        format!("!RetractAssertion {{assertion: {ID}, reason: corrected}}"),
        format!("!DefineNodeType {{id: {ID}, name: item, lifecycle: {{initial: open, states: [open], transitions: [{{from: open, to: open}}]}}, operations: {{action: {{name: action, arguments: {{reason: {{value_kind: String}}}}, transition: {{from: open, to: open}}}}}}}}"),
        format!("!DefineEdgeType {{id: {ID}, name: relation, source_types: [{ID}], target_types: [{ID}]}}"),
        format!("!ModifyProperty {{owner: {ID}, property: {{id: {PROPERTY}, name: field, value_type: {{value_kind: Record, parameters: {{entry: {{value_kind: String}}}}}}}}}}"),
        format!("!MergeEntity {{absorbed: {ID}, into: {ID}}}"),
        format!("!Invoke {{node: {ID}, operation: action, arguments: {{reason: {{value_kind: String, value: ok}}}}}}"),
        format!("!SupersedeAssertion {{assertion: {ID}, by: {PROPERTY}, effective_from: 42}}"),
    ];
    for operation in operations {
        let direct: GraphOperation = serde_yaml_ng::from_str(&operation).unwrap();
        let parsed = TransactionDocument::parse(document(&operation).as_bytes()).unwrap();
        assert_eq!(parsed.transaction().operations, [direct]);
    }
}

#[test]
fn current_withdrawal_operations_require_complete_payloads() {
    for operation in [
        format!("!RetractAssertion {ID}"),
        format!("!RetractAssertion {{assertion: {ID}}}"),
        format!("!RetractAssertion {{assertion: {ID}, reason: corrected, ignored: true}}"),
        format!("!SupersedeAssertion {{assertion: {ID}, by: {PROPERTY}}}"),
        format!("!SupersedeAssertion {{assertion: {ID}, by: {PROPERTY}, effective_from: 42, ignored: true}}"),
    ] {
        assert!(TransactionDocument::parse(document(&operation).as_bytes()).is_err());
    }
}

#[test]
fn mixed_tag_and_coerced_string_accounting_is_exact_at_the_shared_limit() {
    let tag = format!("tag:ekr.test,2026:{}", "x".repeat(32_740));
    let full = format!("0.{}", "0".repeat(32_766));
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
    let remaining = 1_048_576 - header - 16 * (fields + tag.len()) - 15 * full.len();
    let operation = |name: &str| {
        format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, canonical_name: !<{tag}> {name}, properties: {{}}}}")
    };
    let input = |extra: usize| {
        let last = format!("0.{}", "0".repeat(remaining + extra - 2));
        format!("format: ekr.transaction-document/1\ntransaction: {{id: {ID}, proposer: {ID}, operations: [&item {}, {}, {}], evidence: []}}", operation(&full), vec!["*item";14].join(","), operation(&last))
    };
    let exact = input(0);
    let parsed = TransactionDocument::parse(exact.as_bytes()).unwrap();
    assert_eq!(parsed.bytes(), exact.as_bytes());
    assert_eq!(parsed.transaction().operations.len(), 16);
    refused_as(input(1).as_bytes(), DocumentLimit::TotalStringBytes);
}

#[test]
fn explicit_key_tags_also_share_the_expanded_string_budget() {
    let tag = format!("tag:ekr.test,2026:{}", "x".repeat(32_740));
    let text = format!("0.{}", "0".repeat(32_766));
    let operation = format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, ? !<{tag}> canonical_name : {text}, properties: {{}}}}");
    let direct: GraphOperation = serde_yaml_ng::from_str(&operation).unwrap();
    let GraphOperation::CreateNode(created) = direct else {
        panic!("wrong operation")
    };
    assert_eq!(created.canonical_name, text);
    TransactionDocument::parse(document(&operation).as_bytes()).unwrap();
    let expanded = format!("format: ekr.transaction-document/1\ntransaction: {{id: {ID}, proposer: {ID}, operations: [&item {operation}, {}], evidence: []}}", vec!["*item";19].join(","));
    refused_as(expanded.as_bytes(), DocumentLimit::TotalStringBytes);
}

#[test]
fn observable_local_tags_and_coerced_strings_are_counted_once_together() {
    let tag = format!("bounded{}", "x".repeat(32_740));
    let full = format!("0.{}", "0".repeat(32_766));
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
    let remaining = 1_048_576 - header - 16 * (fields + tag.len()) - 15 * full.len();
    let operation = |name: &str| {
        format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, canonical_name: !{tag} {name}, properties: {{}}}}")
    };
    let input = |extra: usize| {
        let last = format!("0.{}", "0".repeat(remaining + extra - 2));
        format!("format: ekr.transaction-document/1\ntransaction: {{id: {ID}, proposer: {ID}, operations: [&item {}, {}, {}], evidence: []}}", operation(&full), vec!["*item";14].join(","), operation(&last))
    };
    let exact = input(0);
    let parsed = TransactionDocument::parse(exact.as_bytes()).unwrap();
    assert_eq!(parsed.bytes(), exact.as_bytes());
    assert_eq!(parsed.transaction().operations.len(), 16);
    refused_as(input(1).as_bytes(), DocumentLimit::TotalStringBytes);
}

#[test]
fn builtin_tags_do_not_override_actual_typed_string_and_numeric_decoding() {
    for name in [
        "!!bool wrong",
        "!!int text",
        "!!float text",
        "!!null text",
        "!!str 0.0",
        "!sample 42",
    ] {
        let operation = format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, canonical_name: {name}, properties: {{}}}}");
        let direct: GraphOperation = serde_yaml_ng::from_str(&operation).unwrap();
        let parsed = TransactionDocument::parse(document(&operation).as_bytes()).unwrap();
        assert_eq!(parsed.transaction().operations, [direct]);
    }
    for value in ["!!str 42", "!!bool 42", "!<tag:ekr.test,2026:numeric> 42"] {
        let operation = format!("!UpdateProperty {{node: {ID}, property: {PROPERTY}, values: [{{value_kind: Integer, value: {value}}}]}}");
        let direct: GraphOperation = serde_yaml_ng::from_str(&operation).unwrap();
        let parsed = TransactionDocument::parse(document(&operation).as_bytes()).unwrap();
        assert_eq!(parsed.transaction().operations, [direct]);
    }
    for key in [
        "? !!bool canonical_name",
        "? !field canonical_name",
        "? !<tag:ekr.test,2026:field> canonical_name",
    ] {
        let operation = format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, {key} : node, properties: {{}}}}");
        let direct: GraphOperation = serde_yaml_ng::from_str(&operation).unwrap();
        let parsed = TransactionDocument::parse(document(&operation).as_bytes()).unwrap();
        assert_eq!(parsed.transaction().operations, [direct]);
    }
}

#[test]
fn a_numeric_lexeme_is_not_charged_as_a_fictitious_string() {
    let number = format!("0.{}", "0".repeat(65_535));
    let operation = format!("!UpdateProperty {{node: {ID}, property: {PROPERTY}, values: [{{value_kind: Float, value: !<tag:ekr.test,2026:number> {number}}}]}}");
    let direct: GraphOperation = serde_yaml_ng::from_str(&operation).unwrap();
    let parsed = TransactionDocument::parse(document(&operation).as_bytes()).unwrap();
    assert_eq!(parsed.transaction().operations, [direct]);
}

#[test]
fn tagged_key_totals_are_exact_for_global_and_local_tags() {
    for local in [false, true] {
        let tag = if local {
            format!("field{}", "x".repeat(32_740))
        } else {
            format!("tag:ekr.test,2026:{}", "x".repeat(32_740))
        };
        let spelling = if local {
            format!("!{tag}")
        } else {
            format!("!<{tag}>")
        };
        let full = format!("0.{}", "0".repeat(32_766));
        let remaining = 1_048_576 - 143 - 16 * (158 + tag.len()) - 15 * full.len();
        let node = |name: &str| {
            format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, ? {spelling} canonical_name : {name}, properties: {{}}}}")
        };
        let input = |extra: usize| {
            format!("format: ekr.transaction-document/1\ntransaction: {{id: {ID}, proposer: {ID}, operations: [&item {}, {}, {}], evidence: []}}", node(&full), vec!["*item";14].join(","), node(&format!("0.{}", "0".repeat(remaining + extra - 2))))
        };
        TransactionDocument::parse(input(0).as_bytes()).unwrap();
        refused_as(input(1).as_bytes(), DocumentLimit::TotalStringBytes);
    }
}

#[test]
fn ignored_value_and_key_tags_add_no_enum_nodes_or_depth() {
    for tag in ["!metadata", "!<tag:ekr.test,2026:metadata>"] {
        let mut nested = format!("{tag} {{value_kind: List, value: []}}");
        for _ in 0..12 {
            nested = format!("{{value_kind: List, value: [{nested}]}}");
        }
        let operation =
            format!("!UpdateProperty {{node: {ID}, property: {PROPERTY}, values: [{nested}]}}");
        let direct: GraphOperation = serde_yaml_ng::from_str(&operation).unwrap();
        let parsed = TransactionDocument::parse(document(&operation).as_bytes()).unwrap();
        assert_eq!(parsed.transaction().operations, [direct]); // depth exactly 32
    }
    for tag in ["!field", "!<tag:ekr.test,2026:field>"] {
        let left = vec!["*v"; 3_272].join(",");
        let right = vec!["*v"; 3_273].join(",");
        let record = format!("{{value_kind: Record, value: {{? {tag} a : {{value_kind: List, value: [&v {{value_kind: Integer, value: 1}}, {left}]}}, b: {{value_kind: List, value: [{right}]}}}}}}");
        // Exactly the same 32,768 syntax nodes as the original node-boundary
        // control: a requested String key's tag adds metadata, not an enum.
        TransactionDocument::parse(update(&record).as_bytes()).unwrap();
    }
}

#[test]
fn ignored_local_tag_on_a_requested_string_at_the_node_limit_is_metadata() {
    let left = vec!["*v"; 3_272].join(",");
    let right = vec!["*v"; 3_273].join(",");
    let record = format!("{{value_kind: Record, value: {{a: {{value_kind: List, value: [&v {{value_kind: Integer, value: 1}}, {left}]}}, b: {{value_kind: List, value: [{right}]}}}}}}");
    // The NodeId decoder requests String; its local tag is ignored exactly as
    // the separate canonical_name String parity controls demonstrate.
    let operation = format!(
        "!UpdateProperty {{node: !metadata {ID}, property: {PROPERTY}, values: [{record}]}}"
    );
    let direct: GraphOperation = serde_yaml_ng::from_str(&operation).unwrap();
    let parsed = TransactionDocument::parse(document(&operation).as_bytes()).unwrap();
    assert_eq!(parsed.transaction().operations, [direct]); // exactly 32,768 nodes
}

#[test]
fn ignored_collection_tags_and_directives_preserve_direct_typed_behavior() {
    for tag in ["!!map", "!fields", "!<tag:ekr.test,2026:fields>"] {
        let operation = format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, canonical_name: item, properties: {tag} {{}}}}");
        let direct: GraphOperation = serde_yaml_ng::from_str(&operation).unwrap();
        let parsed = TransactionDocument::parse(document(&operation).as_bytes()).unwrap();
        assert_eq!(parsed.transaction().operations, [direct]);
    }
    for tag in ["!!seq", "!values", "!<tag:ekr.test,2026:values>"] {
        let operation =
            format!("!UpdateProperty {{node: {ID}, property: {PROPERTY}, values: {tag} []}}");
        let direct: GraphOperation = serde_yaml_ng::from_str(&operation).unwrap();
        let parsed = TransactionDocument::parse(document(&operation).as_bytes()).unwrap();
        assert_eq!(parsed.transaction().operations, [direct]);
    }
    let input = format!("%TAG !e! tag:ekr.test,2026:\n---\n{}", document(&format!("!CreateNode {{id: {ID}, root_id: {ID}, type_id: {ID}, canonical_name: !e!name 42, properties: {{}}}}")));
    let parsed = TransactionDocument::parse(input.as_bytes()).unwrap();
    let GraphOperation::CreateNode(created) = &parsed.transaction().operations[0] else {
        panic!("operation")
    };
    assert_eq!(created.canonical_name, "42");
    assert_eq!(parsed.bytes(), input.as_bytes());
}
