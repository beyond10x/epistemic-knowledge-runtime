//! `Encoder::map` and `Encoder::set` promise an order they do not impose.
//!
//! `canonical.rs` states rule 3 as a property of the encoding — "Ordered by the value, not by the
//! writer" — and documents `Encoder::map` as "the count, then each key and value **in key
//! order**". `Encoder` exists, in its own words, "so an implementation cannot forget a tag or a
//! length"; forgetting the order is the one mistake it still permits, and it is the only one of
//! the three that silently changes a content address between two runs of the same program.
//!
//! The `BTreeMap` implementation in this crate happens to pass a sorted iterator, so the suite
//! never asks the question. A later crate writing `impl Canonical for MyStruct` over a `HashMap`
//! field — which is what this trait is published for — passes an unsorted one, and gets bytes
//! that depend on the process's hash seed.
//!
//! These cases hand `Encoder` an unsorted iterator directly: they construct the state rather than
//! reach it through a caller in this tree, and the finding is sized accordingly.

use ekr_core::{Canonical, Encoder};

fn encoded_map(entries: &[(String, u64)]) -> Vec<u8> {
    let mut encoder = Encoder::new();
    encoder.map(entries.iter().map(|(key, value)| (key, value)));
    encoder.finish()
}

fn encoded_set(items: &[String]) -> Vec<u8> {
    let mut encoder = Encoder::new();
    encoder.set(items.iter());
    encoder.finish()
}

#[test]
fn a_map_encodes_in_key_order_whatever_order_it_is_handed() {
    let sorted = [("alpha".to_owned(), 1u64), ("beta".to_owned(), 2)];
    let unsorted = [("beta".to_owned(), 2u64), ("alpha".to_owned(), 1)];

    assert_eq!(
        encoded_map(&sorted),
        encoded_map(&unsorted),
        "Encoder::map wrote the writer's order, not the key order it documents"
    );
}

#[test]
fn a_set_encodes_in_sorted_order_whatever_order_it_is_handed() {
    let sorted = ["alpha".to_owned(), "beta".to_owned()];
    let unsorted = ["beta".to_owned(), "alpha".to_owned()];

    assert_eq!(
        encoded_set(&sorted),
        encoded_set(&unsorted),
        "Encoder::set wrote the writer's order, not the sorted order it documents"
    );
}

#[test]
fn the_btreemap_path_is_not_what_makes_the_encoding_ordered() {
    // The same two entries, encoded the way a `BTreeMap` encodes them and the way a downstream
    // implementation holding an unordered container would. Equal values, so equal bytes, or the
    // trait's own contract — "two values of one type must encode equally exactly when they are
    // equal" — holds only for the containers this crate happens to implement.
    let map: std::collections::BTreeMap<String, u64> =
        [("alpha".to_owned(), 1u64), ("beta".to_owned(), 2)]
            .into_iter()
            .collect();
    let unsorted = [("beta".to_owned(), 2u64), ("alpha".to_owned(), 1)];

    assert_eq!(map.canonical_bytes(), encoded_map(&unsorted));
}
