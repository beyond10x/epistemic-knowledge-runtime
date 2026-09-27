//! `story:fixture-records-become-observations`: a synthetic JSONL fixture maps each record to
//! exactly one deterministic `ekr_graph::Observation`.

use std::collections::BTreeSet;
use std::path::PathBuf;

use ekr_core::{Canonical, ContentHash, Timestamp};
use ekr_graph::Observation;
use ekr_observe::{observe_jsonl, ObservationIdempotencyKey};

fn fixture() -> Vec<u8> {
    let root = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let path = PathBuf::from(root).join("tests/fixtures/source-records.jsonl");
    std::fs::read(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

/// The fixture's lines, read here independently of the crate under test.
fn lines(bytes: &[u8]) -> Vec<&[u8]> {
    let body = bytes.strip_suffix(b"\n").unwrap_or(bytes);
    body.split(|byte| *byte == b'\n').collect()
}

fn canonical(observations: &[Observation]) -> Vec<Vec<u8>> {
    observations
        .iter()
        .map(Canonical::canonical_bytes)
        .collect()
}

#[test]
fn each_fixture_line_becomes_exactly_one_observation_carrying_that_line() {
    let bytes = fixture();
    let expected = lines(&bytes);
    assert_eq!(expected.len(), 5, "the fixture has five records");

    let observations = observe_jsonl(&bytes).expect("the fixture maps");
    assert_eq!(
        observations.len(),
        expected.len(),
        "one observation per line"
    );

    for (line, observation) in expected.iter().zip(&observations) {
        let record: serde_json::Value = serde_json::from_slice(line).expect("line is JSON");

        assert_eq!(*observation.content_hash(), ContentHash::of_bytes(line));
        assert_eq!(observation.source, record["source"].as_str().unwrap());
        assert_eq!(
            observation.source_native_id.as_deref(),
            record["source_native_id"].as_str()
        );
        assert_eq!(
            observation.captured_at,
            Timestamp::from_millis(record["captured_at"].as_i64().unwrap())
        );

        let key = ObservationIdempotencyKey {
            source: observation.source.clone(),
            source_native_id: observation.source_native_id.clone(),
            content_hash: ContentHash::of_bytes(line),
        };
        assert_eq!(observation.id, key.observation_id());
    }

    let ids: BTreeSet<_> = observations.iter().map(|o| o.id).collect();
    assert_eq!(
        ids.len(),
        observations.len(),
        "distinct records, distinct ids"
    );
}

#[test]
fn mapping_the_fixture_twice_yields_byte_identical_observations() {
    let bytes = fixture();
    let first = observe_jsonl(&bytes).expect("the fixture maps");
    let second = observe_jsonl(&bytes).expect("the fixture maps again");

    assert_eq!(first, second);
    assert_eq!(canonical(&first), canonical(&second));
    let first_ids: Vec<_> = first.iter().map(|o| o.id).collect();
    let second_ids: Vec<_> = second.iter().map(|o| o.id).collect();
    assert_eq!(first_ids, second_ids);
}

#[test]
fn the_same_record_gives_the_same_id_and_a_changed_record_a_different_one() {
    let key = ObservationIdempotencyKey {
        source: "fixture-source-a".to_owned(),
        source_native_id: Some("record-0001".to_owned()),
        content_hash: ContentHash::of_bytes(b"one record"),
    };
    assert_eq!(key.observation_id(), key.clone().observation_id());

    let other_source = ObservationIdempotencyKey {
        source: "fixture-source-b".to_owned(),
        ..key.clone()
    };
    let no_native_id = ObservationIdempotencyKey {
        source_native_id: None,
        ..key.clone()
    };
    let other_bytes = ObservationIdempotencyKey {
        content_hash: ContentHash::of_bytes(b"another record"),
        ..key.clone()
    };
    let ids: BTreeSet<_> = [&key, &other_source, &no_native_id, &other_bytes]
        .into_iter()
        .map(ObservationIdempotencyKey::observation_id)
        .collect();
    assert_eq!(ids.len(), 4, "each field of the key reaches the id");
}

#[test]
fn a_line_that_is_not_a_record_is_refused_with_its_line_number() {
    let error = observe_jsonl(b"{\"source\":\"fixture-source-a\",\"source_native_id\":null,\"captured_at\":0,\"text\":\"\"}\nnot json\n")
        .expect_err("the second line is not a record");
    assert_eq!(error.line(), 2);

    let blank = observe_jsonl(b"\n").expect_err("a blank line is not a record");
    assert_eq!(blank.line(), 1);
}
