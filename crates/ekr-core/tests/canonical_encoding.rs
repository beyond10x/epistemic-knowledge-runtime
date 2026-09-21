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

/// Two variants of one sum type, carrying byte-identical payloads.
///
/// `canonical.rs` rule 5: "two variants carrying the same payload shape would collide, and nothing
/// about their position distinguishes them, so a sum type carries a variant tag written by
/// `Encoder::variant` and by nothing else." This is that case, in its smallest form — the whole
/// payload of each variant is one id, and the two ids are the same one.
///
/// `RevisionEvent` in `ekr-graph` is the runtime's first sum type and the reason the writer exists;
/// the case is written here because the rule and the writer are this crate's.
#[derive(Debug)]
enum Either {
    Left(NodeId),
    Right(NodeId),
}

impl Canonical for Either {
    fn encode(&self, out: &mut ekr_core::Encoder) {
        match self {
            Self::Left(node) => {
                out.variant(0);
                node.encode(out);
            }
            Self::Right(node) => {
                out.variant(1);
                node.encode(out);
            }
        }
    }
}

/// The bytes a variant marker occupies: the tag, then the index as four big-endian bytes.
const MARKER: usize = 5;

#[test]
fn two_variants_with_byte_identical_payloads_encode_differently() {
    let node = NodeId::mint();
    let left = Either::Left(node).canonical_bytes();
    let right = Either::Right(node).canonical_bytes();

    assert_eq!(
        &left[MARKER..],
        &right[MARKER..],
        "the case is only worth anything if the payloads really are byte-identical"
    );
    assert_ne!(
        left, right,
        "two variants of one sum type share an encoding: the variant tag is not being written"
    );
    assert_ne!(
        &left[..MARKER],
        &right[..MARKER],
        "the marker is what separates them, and it does not"
    );
}

#[test]
fn a_variant_marker_is_its_own_shape() {
    let mut encoder = ekr_core::Encoder::new();
    encoder.variant(0);
    let marker = encoder.finish();
    assert_eq!(
        marker.len(),
        MARKER,
        "the marker is a tag and four index bytes"
    );

    // A variant index is not an integer, an id or anything else the encoder writes: rule 1 says
    // every value starts with a byte naming its shape, and a variant marker has its own.
    for write in [
        (|out: &mut ekr_core::Encoder| out.unsigned(0)) as fn(&mut ekr_core::Encoder),
        |out: &mut ekr_core::Encoder| out.signed(0),
        |out: &mut ekr_core::Encoder| out.id(0),
        |out: &mut ekr_core::Encoder| out.unit(),
        |out: &mut ekr_core::Encoder| out.boolean(false),
        |out: &mut ekr_core::Encoder| out.string(""),
        |out: &mut ekr_core::Encoder| out.bytes(b""),
        |out: &mut ekr_core::Encoder| out.option(None::<&u64>),
        |out: &mut ekr_core::Encoder| out.list([&0u64; 0].into_iter()),
    ] {
        let mut other = ekr_core::Encoder::new();
        write(&mut other);
        let other = other.finish();
        assert_ne!(
            marker[0], other[0],
            "the variant tag collides with another shape"
        );
    }

    // Index zero and index one are different markers, or an enum's first two variants collide.
    let mut one = ekr_core::Encoder::new();
    one.variant(1);
    assert_ne!(marker, one.finish());
}
