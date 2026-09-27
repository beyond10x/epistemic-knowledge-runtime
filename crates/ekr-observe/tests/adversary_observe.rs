//! Adversary cases for `story:fixture-records-become-observations`.

use ekr_graph::ObservationKind;
use ekr_observe::{observe_jsonl, ObserveError};

/// Coordinator decision, correction round 1: `observe.yaml:45-56` keys on source, source-native id
/// and content hash, and the story hashes the line's bytes, `captured_at` included. Two lines that
/// differ only in their timestamp are therefore two records, and get two ids.
#[test]
fn two_lines_differing_only_in_timestamp_are_two_records_with_two_ids() {
    let first = br#"{"source":"fixture-source-a","source_native_id":"record-0001","captured_at":1767225600000,"text":"An assertion was proposed against the canonical root."}"#;
    let again = br#"{"source":"fixture-source-a","source_native_id":"record-0001","captured_at":1767229200000,"text":"An assertion was proposed against the canonical root."}"#;

    let first = observe_jsonl(first).expect("maps");
    let again = observe_jsonl(again).expect("maps");

    assert_ne!(
        first[0].content_hash(),
        again[0].content_hash(),
        "story:fixture-records-become-observations hashes the whole line, timestamp included"
    );
    assert_ne!(
        first[0].id, again[0].id,
        "story:fixture-records-become-observations: a different timestamp is a different record, \
         so a different id (observe.yaml:45-56)"
    );
}

/// Kills the mutant that deletes the blank-line branch of `observe_line`: the implementor's test
/// checks only `line()`, which a `NotARecord` at line 1 satisfies too.
#[test]
fn a_blank_line_is_refused_as_blank_not_as_malformed_json() {
    let error = observe_jsonl(b"\n").expect_err("blank");
    assert!(
        matches!(error, ObserveError::Blank { line: 1 }),
        "{error:?}"
    );

    let record = br#"{"source":"s","source_native_id":null,"captured_at":0,"text":""}"#;
    let mut two = record.to_vec();
    two.extend_from_slice(b"\n  \n");
    let error = observe_jsonl(&two).expect_err("second line blank");
    assert!(
        matches!(error, ObserveError::Blank { line: 2 }),
        "{error:?}"
    );
}

/// Kills the mutant that swaps `Builder::from_custom_bytes` for any other builder: the doc at
/// src/lib.rs:54 promises a version 8 UUID and nothing asserts it.
#[test]
fn the_derived_id_is_a_version_8_rfc_uuid() {
    let root = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let path = std::path::PathBuf::from(root).join("tests/fixtures/source-records.jsonl");
    let bytes = std::fs::read(&path).expect("fixture reads");
    for observation in observe_jsonl(&bytes).expect("maps") {
        let uuid = observation.id.to_uuid();
        assert_eq!(uuid.get_version_num(), 8, "{uuid}");
        assert_eq!(uuid.get_variant(), uuid::Variant::RFC4122, "{uuid}");
        assert_eq!(observation.kind(), ObservationKind::FeedItem);
    }
}

/// A duplicated field is two statements of one value; last-wins would let a line say two things.
#[test]
fn a_line_with_a_duplicated_field_is_refused() {
    let line = br#"{"source":"a","source":"b","source_native_id":null,"captured_at":0,"text":""}"#;
    let error = observe_jsonl(line).expect_err("duplicate field");
    assert!(
        matches!(error, ObserveError::NotARecord { line: 1, .. }),
        "{error:?}"
    );
}
