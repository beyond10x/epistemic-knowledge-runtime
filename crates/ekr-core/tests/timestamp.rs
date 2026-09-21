//! `Timestamp`: the one point-in-time scalar the whole stack shares.
//!
//! `architecture-decision-record:0004-timestamp-in-ekr-core` puts it here rather than in
//! `ekr-graph`, because four of the five ESS domains declare a `Timestamp` field — `kernel.yaml`,
//! `graph.yaml`, `ontology.yaml`, `store.yaml` — and `ekr-ontology` does not depend on
//! `ekr-graph`, so a `Timestamp` above the ontology would leave one domain scalar with two
//! unrelated Rust types.
//!
//! What the ADR settles and these cases hold it to:
//!
//! * **milliseconds since the Unix epoch, UTC, signed.** Valid time in design § 13–14 is the time
//!   a fact was true in the world, which for an organisational memory is routinely before 1970 and
//!   is routinely outside the 1678–2262 window `i64` nanoseconds spans.
//! * **one text per value.** The same rule `RevisionNumber` is held to: `+7`, `007` and `-0` are
//!   numbers `i64::from_str` reads and a `Timestamp` does not, because a value with several texts
//!   is a value a cache key, a diff or a stored document sees change when nothing changed.
//! * **structural under `Canonical`.** A newtype encodes as the thing it wraps —
//!   `crates/ekr-core/src/canonical.rs` rule 5 — so a `Timestamp` produces the bytes of its
//!   `i64` and carries no discriminant.
//! * **no clock, no calendar.** Nothing here reads the system time and nothing formats a date; P1
//!   declares no time crate and the ADR does not add one.

use ekr_core::canonical::Canonical;
use ekr_core::{Timestamp, TimestampParseError};

/// 2026-03-12T00:00:00Z, the instant design § 65 hands the chair over on.
const HANDOVER_MILLIS: i64 = 1_773_273_600_000;

#[test]
fn a_timestamp_is_milliseconds_since_the_unix_epoch() {
    assert_eq!(Timestamp::EPOCH.millis(), 0);
    assert_eq!(
        Timestamp::from_millis(HANDOVER_MILLIS).millis(),
        HANDOVER_MILLIS
    );

    // Signed: valid time before 1970 is ordinary — a claim about when a company was founded is
    // one — so the type orders across the epoch rather than bottoming out at it.
    let founded = Timestamp::from_millis(-2_208_988_800_000);
    assert!(founded < Timestamp::EPOCH);
    assert!(Timestamp::EPOCH < Timestamp::from_millis(HANDOVER_MILLIS));
}

#[test]
fn the_range_is_far_wider_than_nanoseconds_would_give() {
    // The ADR's reason for milliseconds rather than nanoseconds: `i64` nanoseconds spans
    // 1678–2262, which does not contain the valid time of ordinary organisational facts.
    let millennium_ago = Timestamp::from_millis(-31_556_952_000_000);
    let far_future = Timestamp::from_millis(31_556_952_000_000_000);
    assert!(millennium_ago < Timestamp::EPOCH);
    assert!(Timestamp::EPOCH < far_future);
    assert_eq!(far_future.millis(), 31_556_952_000_000_000);
}

#[test]
fn one_value_has_one_text() {
    for millis in [0, 1, -1, HANDOVER_MILLIS, i64::MIN, i64::MAX] {
        let value = Timestamp::from_millis(millis);
        assert_eq!(value.to_string(), millis.to_string());
        assert_eq!(
            value
                .to_string()
                .parse::<Timestamp>()
                .expect("its own text"),
            value
        );
    }
}

#[test]
fn the_text_form_refuses_every_other_spelling() {
    for text in [
        "", "+7", "007", "-0", "-007", " 7", "7 ", "7.0", "7ms", "0x7", "--7", "nine",
    ] {
        let refused = text
            .parse::<Timestamp>()
            .expect_err("a second text for one value, or no value at all");
        assert_eq!(refused.text(), text);
        assert!(
            refused.to_string().contains(&format!("{text:?}")),
            "the refusal names the text it refused: {refused}"
        );
    }

    // The type it is not: `std::num::ParseIntError` accepts the first two of those.
    assert!("+7".parse::<i64>().is_ok() && "007".parse::<i64>().is_ok());
}

#[test]
fn a_refusal_carries_the_text_and_nothing_else() {
    let refused: TimestampParseError = "later".parse::<Timestamp>().unwrap_err();
    assert_eq!(refused.text(), "later");
    assert_eq!(refused, "later".parse::<Timestamp>().unwrap_err());
}

#[test]
fn a_timestamp_encodes_as_the_integer_it_wraps() {
    // Rule 5 of `canonical.rs`: a newtype is structural. So the bytes are the `i64`'s bytes, with
    // no discriminant — the same contract `RevisionNumber` and the id types carry.
    for millis in [0i64, 1, -1, HANDOVER_MILLIS] {
        assert_eq!(
            Timestamp::from_millis(millis).canonical_bytes(),
            millis.canonical_bytes(),
            "a newtype grew a discriminant"
        );
    }

    assert_ne!(
        Timestamp::from_millis(1).canonical_bytes(),
        Timestamp::from_millis(2).canonical_bytes()
    );
}

#[test]
fn a_timestamp_crosses_serde_as_the_number_it_is() {
    let value = Timestamp::from_millis(HANDOVER_MILLIS);
    let json = serde_json::to_string(&value).expect("a timestamp serialises");

    // Transparent, as `RevisionNumber` is: a document that carried `created_at: 0` before the
    // newtype existed reads back unchanged after it.
    assert_eq!(json, HANDOVER_MILLIS.to_string());
    assert_eq!(
        serde_json::from_str::<Timestamp>(&json).expect("and reads back"),
        value
    );
    assert_eq!(
        serde_json::from_str::<Timestamp>("0").expect("a bare zero is a timestamp"),
        Timestamp::EPOCH
    );
}
