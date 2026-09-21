//! The canonical encoding is a function of the value, not of how the value was built.
//!
//! The second of `story:kernel-identity-and-hashing`'s "Tests the story ships". These bytes are
//! what a `ContentHash` is computed over, so two runs that hold the same knowledge must produce
//! the same bytes or every content address the runtime records is noise.
//!
//! Two halves, and both are load-bearing:
//!
//! * **order-independence where order is not meaning** — a `BTreeMap` is a set of entries, so
//!   insertion order may not reach the bytes;
//! * **injectivity where it is** — a list keeps its order, and two different values may not share
//!   an encoding, or a hash equality would be a lie.

use std::collections::BTreeMap;

use ekr_core::canonical::Canonical;
use ekr_core::{ContentHash, NodeId};
use proptest::prelude::*;

#[test]
fn two_btreemaps_with_the_same_entries_encode_to_the_same_bytes() {
    let mut forwards = BTreeMap::new();
    for (key, value) in [("alpha", 1u64), ("beta", 2), ("gamma", 3), ("delta", 4)] {
        forwards.insert(key.to_owned(), value);
    }

    let mut backwards = BTreeMap::new();
    for (key, value) in [("delta", 4u64), ("gamma", 3), ("beta", 2), ("alpha", 1)] {
        backwards.insert(key.to_owned(), value);
    }

    assert_eq!(forwards, backwards);
    assert_eq!(
        forwards.canonical_bytes(),
        backwards.canonical_bytes(),
        "insertion order reached the bytes"
    );
}

#[test]
fn a_map_that_differs_in_one_value_encodes_differently() {
    let one: BTreeMap<String, u64> = [("alpha".to_owned(), 1u64)].into_iter().collect();
    let other: BTreeMap<String, u64> = [("alpha".to_owned(), 2u64)].into_iter().collect();

    assert_ne!(one.canonical_bytes(), other.canonical_bytes());
}

#[test]
fn list_order_is_meaning_and_survives() {
    let one = vec!["a".to_owned(), "b".to_owned()];
    let other = vec!["b".to_owned(), "a".to_owned()];

    assert_ne!(
        one.canonical_bytes(),
        other.canonical_bytes(),
        "a list is ordered; its encoding must say so"
    );
}

#[test]
fn field_boundaries_are_not_ambiguous() {
    let one = vec!["ab".to_owned(), "c".to_owned()];
    let other = vec!["a".to_owned(), "bc".to_owned()];
    assert_ne!(
        one.canonical_bytes(),
        other.canonical_bytes(),
        "concatenation without a length prefix"
    );

    let map: BTreeMap<String, String> = [("ab".to_owned(), "c".to_owned())].into_iter().collect();
    let other_map: BTreeMap<String, String> =
        [("a".to_owned(), "bc".to_owned())].into_iter().collect();
    assert_ne!(map.canonical_bytes(), other_map.canonical_bytes());
}

#[test]
fn a_type_is_part_of_the_encoding() {
    assert_ne!(
        "1".to_owned().canonical_bytes(),
        1u64.canonical_bytes(),
        "a string and a number that print the same must not encode the same"
    );
    assert_ne!(
        Some(1u64).canonical_bytes(),
        1u64.canonical_bytes(),
        "an optional value and a present one must not encode the same"
    );
    assert_ne!(
        None::<u64>.canonical_bytes(),
        Vec::<u64>::new().canonical_bytes(),
        "absence and emptiness are different"
    );
}

#[test]
fn the_primitives_of_this_crate_encode() {
    let id = NodeId::mint();
    assert_eq!(id.canonical_bytes(), id.canonical_bytes());
    assert_ne!(id.canonical_bytes(), NodeId::mint().canonical_bytes());

    let hash = ContentHash::of_bytes(b"knowledge");
    assert_eq!(hash.canonical_bytes(), hash.canonical_bytes());
    assert_ne!(
        hash.canonical_bytes(),
        ContentHash::of_bytes(b"other").canonical_bytes()
    );

    let nested: BTreeMap<String, Vec<Option<i64>>> = [
        ("present".to_owned(), vec![Some(-1), Some(0), Some(1)]),
        ("absent".to_owned(), vec![None]),
    ]
    .into_iter()
    .collect();
    assert_eq!(nested.canonical_bytes(), nested.canonical_bytes());
}

proptest! {
    /// Whatever the entries and whatever the order they arrive in, the bytes are the map's.
    #[test]
    fn insertion_order_never_reaches_the_bytes(
        mut entries in proptest::collection::vec(("[a-z]{1,8}", any::<u64>()), 0..24),
    ) {
        let forwards: BTreeMap<String, u64> = entries.iter().cloned().collect();
        entries.reverse();
        let backwards: BTreeMap<String, u64> = entries.into_iter().collect();

        // Reversing changes which duplicate key wins, so only compare equal maps.
        prop_assume!(forwards == backwards);
        prop_assert_eq!(forwards.canonical_bytes(), backwards.canonical_bytes());
    }

    /// Equal values encode equally and unequal values do not — the property a content address
    /// rests on.
    #[test]
    fn encoding_distinguishes_exactly_what_equality_does(
        left in proptest::collection::btree_map("[a-z]{1,4}", any::<i64>(), 0..8),
        right in proptest::collection::btree_map("[a-z]{1,4}", any::<i64>(), 0..8),
    ) {
        if left == right {
            prop_assert_eq!(left.canonical_bytes(), right.canonical_bytes());
        } else {
            prop_assert_ne!(left.canonical_bytes(), right.canonical_bytes());
        }
    }
}
