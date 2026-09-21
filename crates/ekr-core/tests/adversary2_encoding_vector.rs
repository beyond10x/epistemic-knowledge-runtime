//! The ten tag bytes no vector pins, and two things the encoding's own rules say about addresses.
//!
//! `adversary_encoding_vector.rs` closed half of pass 1's finding 2: it pins `MAP`, `LIST`,
//! `STRING` and `UNSIGNED`. Ten of the fourteen tags are still unpinned, and two of them —
//! `tag::ID` and `tag::HASH` — are the encodings of the only two types this crate exists to
//! publish. Nothing in the suite compares an id's or a hash's canonical bytes to a literal, so
//! either constant can change with every case green while every recorded address that contains an
//! id or a sub-hash moves.
//!
//! The vector below is derived by hand from `canonical.rs`'s stated rules and its `mod tag`
//! constants; its digest is `sha256sum` over those bytes, computed outside this crate. Neither
//! number is read back from the implementation. It also pins the `set` sort the correction added,
//! by handing `set` its elements in the order they are not written in.

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{ContentHash, NodeId};

/// The UUID the id half of the vector is built from.
const VECTOR_UUID: &str = "01a0c3a0-7889-7395-b687-b771f5ae3aa7";

/// Every tag `adversary_encoding_vector.rs` does not reach, written in one buffer:
///
/// ```text
/// 01                                  UNIT
/// 02                                  FALSE
/// 03                                  TRUE
/// 05 ff*16                            SIGNED -1
/// 07 0000000000000002 6162            BYTES "ab"
/// 09 0000000000000002                 SET, two elements, in element order
///    06 0000000000000001 61             "a"
///    06 0000000000000001 62             "b"
/// 0b                                  NONE
/// 0c 04 ...0001                       SOME 1u64
/// 0d 01a0c3a0...3aa7                  ID
/// 0e 56ea9de6...17a2                  HASH, of_bytes(b"abc") under the payload domain
/// ```
const VECTOR_BYTES: &str = concat!(
    "01",
    "02",
    "03",
    "05",
    "ffffffffffffffffffffffffffffffff",
    "07",
    "0000000000000002",
    "6162",
    "09",
    "0000000000000002",
    "06",
    "0000000000000001",
    "61",
    "06",
    "0000000000000001",
    "62",
    "0b",
    "0c",
    "04",
    "00000000000000000000000000000001",
    "0d",
    "01a0c3a078897395b687b771f5ae3aa7",
    "0e",
    "56ea9de676a04bfddaa30bf81ab69df4839b9b56aeb3a5d8899bcb2edbde17a2",
);

/// `sha256sum` of the payload domain label `ekr.payload.v1` followed by the bytes above,
/// computed outside this crate.
///
/// Was `9aeb0c0b…e1b3` over those bytes alone, and the HASH element inside them was the FIPS
/// 180-4 digest of `abc`. Both moved on 2026-09-21 with the second correction round: every
/// address is now taken over its domain label, so `ContentHash::of_bytes` — the value inside the
/// vector as well as the digest over it — is no longer a bare SHA-256.
const VECTOR_DIGEST: &str = "1ca7f1f9c9e3899df39a849a94bbe722b72bd786bbb6c7a6af90f7e34cb189ce";

fn vector_encoder() -> Encoder {
    let node: NodeId = VECTOR_UUID.parse().expect("the one text form of an id");
    let unsorted = ["b".to_owned(), "a".to_owned()];

    let mut encoder = Encoder::new();
    encoder.unit();
    encoder.boolean(false);
    encoder.boolean(true);
    encoder.signed(-1);
    encoder.bytes(b"ab");
    encoder.set(unsorted.iter());
    encoder.option(None::<&u64>);
    encoder.option(Some(&1u64));
    encoder.id(node.as_u128());
    encoder.hash(ContentHash::of_bytes(b"abc").as_bytes());
    encoder
}

#[test]
fn the_remaining_tags_match_their_published_bytes() {
    assert_eq!(
        hex::encode(vector_encoder().finish()),
        VECTOR_BYTES,
        "a tag byte, a length prefix or the set order moved; every address that contains one of \
         these shapes is stale"
    );
}

#[test]
fn a_content_address_over_the_remaining_tags_matches_its_published_digest() {
    assert_eq!(
        ContentHash::of_bytes(vector_encoder().as_bytes()).to_string(),
        VECTOR_DIGEST,
        "the address of a fixed encoding moved"
    );
}

/// An id's own canonical bytes, pinned — the encoding of the type the story is named after.
#[test]
fn an_id_encodes_as_its_published_bytes() {
    let node: NodeId = VECTOR_UUID.parse().expect("the one text form of an id");

    assert_eq!(
        hex::encode(node.canonical_bytes()),
        concat!("0d", "01a0c3a078897395b687b771f5ae3aa7"),
        "tag::ID or the id's byte order moved"
    );
}

/// A content hash's own canonical bytes, pinned.
#[test]
fn a_content_hash_encodes_as_its_published_bytes() {
    assert_eq!(
        hex::encode(ContentHash::of_bytes(b"abc").canonical_bytes()),
        concat!(
            "0e",
            "56ea9de676a04bfddaa30bf81ab69df4839b9b56aeb3a5d8899bcb2edbde17a2"
        ),
        "tag::HASH moved"
    );
}

/// Rule 3 against `Encoder::map`'s own doc, in the one case where they say opposite things.
///
/// Rule 3 of `canonical.rs`: "A map encodes in key order and a set in element order, so two maps
/// with the same entries encode identically whatever order they were built in." `Encoder::map`:
/// "Duplicate keys are not a map and are not deduplicated; the sort is stable, so entries sharing
/// a key keep the order they were handed in." For a repeated key the writer's order therefore
/// reaches the bytes, which is the thing rule 3 says cannot happen.
#[test]
fn a_map_with_a_repeated_key_encodes_the_same_whatever_order_it_is_handed() {
    fn encoded(entries: &[(String, u64)]) -> Vec<u8> {
        let mut encoder = Encoder::new();
        encoder.map(entries.iter().map(|(key, value)| (key, value)));
        encoder.finish()
    }

    let one = [("a".to_owned(), 1u64), ("a".to_owned(), 2u64)];
    let other = [("a".to_owned(), 2u64), ("a".to_owned(), 1u64)];

    assert_eq!(
        hex::encode(encoded(&one)),
        hex::encode(encoded(&other)),
        "the writer's order reached the bytes, which rule 3 says it cannot"
    );
}

/// A raw payload and a structured value must not share a content address.
///
/// Rule 1 of `canonical.rs`: "Every value starts with a byte naming its shape, so a string and a
/// number that read the same do not encode the same." `ContentHash::of_bytes` is the entry point
/// that writes no tag at all, and the story's Notes name it as the path for observations and
/// evidence — "content-derived for observations and evidence (which are what they hash)". Those
/// payloads come from outside the runtime. A payload whose bytes are some value's canonical
/// encoding therefore takes that value's address, in one address space, with nothing separating
/// the two domains.
#[test]
fn a_raw_payload_never_shares_an_address_with_a_canonical_value() {
    let value = "knowledge".to_owned();
    let payload = value.canonical_bytes();

    assert_ne!(
        ContentHash::of_bytes(&payload),
        ContentHash::of(&value),
        "a raw payload took a structured value's content address"
    );
}
