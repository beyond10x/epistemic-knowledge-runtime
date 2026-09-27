//! Adversary cases, pass 2, for `story:fixture-records-become-observations`.

use ekr_observe::{observe_jsonl, observe_line, ObserveError};

const RECORD: &[u8] = br#"{"source":"fixture-source-a","source_native_id":"record-0001","captured_at":1767225600000,"text":"An assertion was proposed against the canonical root."}"#;

/// `observe_line` is public and documented as mapping "one source record line"; the module doc
/// says the content hash is over the line's bytes "without the `\n` that ends it". A caller that
/// hands `observe_line` a line with its terminator (what `BufRead::read_until(b'\n', ..)` yields)
/// must get the same observation `observe_jsonl` gives for that line, or a refusal. Today it gets
/// a second, different id for the same record: the JSON reader skips the `\n` as whitespace and
/// the hash covers it.
#[test]
fn observe_line_given_a_terminated_line_agrees_with_observe_jsonl_or_refuses() {
    let mut terminated = RECORD.to_vec();
    terminated.push(b'\n');

    let via_jsonl = observe_jsonl(&terminated).expect("one record");
    assert_eq!(via_jsonl.len(), 1);

    match observe_line(&terminated, 1) {
        Err(_) => {}
        Ok(observation) => assert_eq!(
            observation, via_jsonl[0],
            "the same record line gave two observations with two ids depending on the entry point"
        ),
    }
}

/// Kills the mutant that deletes the `bytes.is_empty()` branch of `observe_jsonl`: the doc says
/// "Empty input has no lines", and without the branch empty input is refused as a blank line 1.
/// No existing case maps empty input.
#[test]
fn empty_input_maps_to_no_observations() {
    let observations = observe_jsonl(b"").expect("empty input has no lines");
    assert!(observations.is_empty());
}

/// Kills the mutant that makes `SourceRecord::text` optional or defaulted: src/lib.rs states
/// "Required so a line without text is refused", and no existing case sends a line without it.
#[test]
fn a_line_without_text_is_refused() {
    let line = br#"{"source":"fixture-source-a","source_native_id":"record-0001","captured_at":0}"#;
    let error = observe_jsonl(line).expect_err("a line without text is not a record");
    assert!(
        matches!(error, ObserveError::NotARecord { line: 1, .. }),
        "{error:?}"
    );
}

/// Kills the mutant that drops `#[serde(deny_unknown_fields)]` from `SourceRecord`: no existing
/// case sends a field the fixture format does not name.
#[test]
fn a_line_with_an_unknown_field_is_refused() {
    let line = br#"{"source":"fixture-source-a","source_native_id":null,"captured_at":0,"text":"","kind":"FeedItem"}"#;
    let error = observe_jsonl(line).expect_err("an unknown field is refused");
    assert!(
        matches!(error, ObserveError::NotARecord { line: 1, .. }),
        "{error:?}"
    );
}
