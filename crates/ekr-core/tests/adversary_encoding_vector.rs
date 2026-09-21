//! A published vector for the canonical encoding itself, not only for SHA-256.
//!
//! `content_hash.rs` states the principle and then applies it to the wrong half: "a case that
//! hashes a value twice in one process and compares the two proves the function is a function,
//! not that it is SHA-256. A published vector proves it across runs, releases and machines." Its
//! two literal vectors are the FIPS 180-4 digests of `""` and `"abc"`, which pin `sha2`. Nothing
//! in the suite pins the bytes `ContentHash::of` hashes, so every tag byte, every length prefix
//! and the whole field order can change and the suite stays green — while every content address
//! the runtime has ever recorded silently becomes a different address.
//!
//! The bytes below are derived by hand from `canonical.rs`'s stated rules — tagged, eight
//! big-endian bytes of length, `BTreeMap` in key order — and the digest is `sha256sum` over them,
//! computed outside this crate. Neither number is read back from the implementation.

use std::collections::BTreeMap;

use ekr_core::canonical::Canonical;
use ekr_core::ContentHash;

/// `{"a": [], "b": [1u64]}`, encoded by hand:
///
/// ```text
/// 0a                                  MAP
/// 0000000000000002                    two entries
/// 06 0000000000000001 61              "a"
/// 08 0000000000000000                 []
/// 06 0000000000000001 62              "b"
/// 08 0000000000000001                 one element
/// 04 00000000000000000000000000000001 1
/// ```
const VECTOR_BYTES: &str = concat!(
    "0a",
    "0000000000000002",
    "06",
    "0000000000000001",
    "61",
    "08",
    "0000000000000000",
    "06",
    "0000000000000001",
    "62",
    "08",
    "0000000000000001",
    "04",
    "00000000000000000000000000000001",
);

/// `sha256sum` of the value domain label `ekr.value.v1` followed by the bytes above, computed
/// outside this crate.
///
/// Was `d22d66bd…878d`, `sha256sum` of those bytes alone. It moved on 2026-09-21 with the second
/// correction round: `ContentHash::of` now takes its digest over `ekr.value.v1` and the canonical
/// encoding, so that a raw payload cannot be handed a value's address. The encoding itself —
/// `VECTOR_BYTES` above — did not move, which is the point of pinning the two separately.
const VECTOR_DIGEST: &str = "be70591da07ce3141f631ba9c78f27a69fbca524fff7065e8ffada9e5bd2c8c3";

fn vector_value() -> BTreeMap<String, Vec<u64>> {
    [("b".to_owned(), vec![1u64]), ("a".to_owned(), Vec::new())]
        .into_iter()
        .collect()
}

#[test]
fn the_canonical_encoding_matches_its_published_bytes() {
    assert_eq!(
        hex::encode(vector_value().canonical_bytes()),
        VECTOR_BYTES,
        "the canonical encoding moved; every content address recorded before this change is stale"
    );
}

#[test]
fn a_content_address_matches_its_published_digest() {
    assert_eq!(
        ContentHash::of(&vector_value()).to_string(),
        VECTOR_DIGEST,
        "the address of a fixed value moved"
    );
}

#[test]
fn to_hex_is_the_form_from_str_reads() {
    // `ContentHash::to_hex` is public, is documented as "the sixty-four lowercase hex characters",
    // and is called by no case in the suite; `FromStr` refuses anything but lowercase.
    let hash = ContentHash::of_bytes(b"knowledge");
    let hex = hash.to_hex();

    assert_eq!(hex, hash.to_string(), "to_hex and Display disagree");
    assert_eq!(
        hex.parse::<ContentHash>().expect("to_hex output parses"),
        hash
    );
}
