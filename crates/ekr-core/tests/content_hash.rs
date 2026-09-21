//! The same canonical bytes produce the same `ContentHash` on every run, and the hex round-trips.
//!
//! The first of `story:kernel-identity-and-hashing`'s "Tests the story ships". Design § 57:
//! immutable artifacts are content-addressed, which is only true if the address is a function of
//! the content and of nothing else — not of a process, a seed, an allocation or a clock.
//!
//! "On every run" is why the vectors below are literal: a case that hashes a value twice in one
//! process and compares the two proves the function is a function, not that it is SHA-256. A
//! published vector proves it across runs, releases and machines.
//!
//! The vectors moved on 2026-09-21, in the second correction round: `ContentHash` now takes each
//! digest over a domain label followed by the bytes, so neither entry point is the bare SHA-256
//! of its input any more. Each literal below is `sha256sum` over the label and the input
//! concatenated, computed outside this crate, and the FIPS 180-4 digest it replaces is named
//! beside it so the derivation can be walked by hand.

use std::collections::BTreeMap;

use ekr_core::canonical::Canonical;
use ekr_core::ContentHash;
use proptest::prelude::*;

/// `sha256sum` of `ekr.payload.v1` — the payload domain with nothing after it. Was
/// `e3b0c442…b855`, the FIPS 180-4 digest of the empty input, before the domain existed.
const EMPTY: &str = "b4a9c652b005eb23f33bb3edd978ec727d93c07d4c2c005b86801c7ce23096d1";
/// `sha256sum` of `ekr.payload.v1abc`. Was `ba7816bf…15ad`, the FIPS 180-4 digest of `abc`.
const ABC: &str = "56ea9de676a04bfddaa30bf81ab69df4839b9b56aeb3a5d8899bcb2edbde17a2";
/// `sha256sum` of `ekr.value.v1` followed by the canonical encoding of the `String`
/// `"knowledge"` — `06` `0000000000000009` `6b6e6f776c65646765`, by the rules of `canonical.rs`.
const KNOWLEDGE_AS_A_VALUE: &str =
    "a0073b7989ee4ef15c9e6a89ef0416e7e8e27c057438327347d03f89a5a3af3b";
/// `sha256sum` of `ekr.payload.v1knowledge` — the same nine bytes arriving as a payload.
const KNOWLEDGE_AS_A_PAYLOAD: &str =
    "6b96ce8883b869247cb3eb6ef32d555b302cd1748b72976b1d81eddf5312e243";

#[test]
fn the_hash_is_sha256_of_its_domain_and_the_bytes_it_is_given() {
    assert_eq!(ContentHash::of_bytes(b"").to_string(), EMPTY);
    assert_eq!(ContentHash::of_bytes(b"abc").to_string(), ABC);
}

#[test]
fn a_value_and_a_payload_are_addressed_in_different_domains() {
    let value = "knowledge".to_owned();

    // Each side is pinned to a digest computed outside the crate, so the case states what the two
    // addresses *are*, not merely that they differ.
    assert_eq!(ContentHash::of(&value).to_string(), KNOWLEDGE_AS_A_VALUE);
    assert_eq!(
        ContentHash::of_bytes(b"knowledge").to_string(),
        KNOWLEDGE_AS_A_PAYLOAD
    );

    // The composition this file asserted until 2026-09-21 — `of(&v) == of_bytes(&v.canonical_bytes())`
    // — is what let a payload take a value's address. It must not hold.
    assert_ne!(
        ContentHash::of(&value),
        ContentHash::of_bytes(&value.canonical_bytes()),
        "a payload carrying a value's canonical bytes took the value's address"
    );
}

#[test]
fn no_payload_can_reach_a_value_address() {
    // The separation is total rather than probable, and this is why: neither domain label is a
    // prefix of the other, so the two hashed inputs differ at a position no payload can reach.
    assert!(
        !ContentHash::VALUE_DOMAIN.starts_with(ContentHash::PAYLOAD_DOMAIN)
            && !ContentHash::PAYLOAD_DOMAIN.starts_with(ContentHash::VALUE_DOMAIN),
        "one domain label is a prefix of the other, so some payload shares a value's hashed input"
    );
    assert_ne!(ContentHash::VALUE_DOMAIN, ContentHash::PAYLOAD_DOMAIN);
}

#[test]
fn the_same_canonical_bytes_hash_the_same() {
    let mut forwards = BTreeMap::new();
    for (key, value) in [("alpha", 1u64), ("beta", 2), ("gamma", 3)] {
        forwards.insert(key.to_owned(), value);
    }
    let mut backwards = BTreeMap::new();
    for (key, value) in [("gamma", 3u64), ("beta", 2), ("alpha", 1)] {
        backwards.insert(key.to_owned(), value);
    }

    assert_eq!(forwards.canonical_bytes(), backwards.canonical_bytes());
    assert_eq!(
        ContentHash::of(&forwards),
        ContentHash::of(&backwards),
        "equal canonical bytes must give one address"
    );
}

#[test]
fn the_hex_form_round_trips() {
    let hash = ContentHash::of_bytes(b"knowledge");
    let text = hash.to_string();

    assert_eq!(text.len(), 64, "32 bytes as lowercase hex");
    assert!(
        text.chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
        "lowercase hex only: {text}"
    );
    assert_eq!(text.parse::<ContentHash>().expect("parses"), hash);

    let json = serde_json::to_string(&hash).expect("serialises");
    assert_eq!(json, format!("\"{text}\""));
    assert_eq!(
        serde_json::from_str::<ContentHash>(&json).expect("deserialises"),
        hash
    );
}

#[test]
fn a_malformed_hex_form_is_refused() {
    // Each malformation is built by position rather than by matching a character, so that the
    // case keeps saying the same thing when the pinned digest changes — which it did on
    // 2026-09-21, and `EMPTY.trim_end_matches('5')` silently became a no-op that fed the parser a
    // *well*-formed hash and asserted it was refused.
    let too_short = &EMPTY[..EMPTY.len() - 1];
    let too_long = format!("{EMPTY}0");
    let not_hex = format!("g{}", &EMPTY[1..]);
    let uppercase = EMPTY.to_ascii_uppercase();
    assert_ne!(uppercase, EMPTY, "the pinned digest is all digits");

    for malformed in ["", "abc", too_short, &too_long, &not_hex, &uppercase] {
        assert!(
            malformed.parse::<ContentHash>().is_err(),
            "parsing {malformed:?} must be refused"
        );
        assert!(
            serde_json::from_str::<ContentHash>(&format!("\"{malformed}\"")).is_err(),
            "deserialising {malformed:?} must be refused"
        );
    }

    for malformed in ["42", "null", "[]"] {
        assert!(
            serde_json::from_str::<ContentHash>(malformed).is_err(),
            "deserialising {malformed} must be refused"
        );
    }
}

proptest! {
    /// Equal content, equal address; different content, different address.
    #[test]
    fn the_address_is_a_function_of_the_content(
        left in proptest::collection::vec(any::<u8>(), 0..256),
        right in proptest::collection::vec(any::<u8>(), 0..256),
    ) {
        prop_assert_eq!(ContentHash::of_bytes(&left), ContentHash::of_bytes(&left));
        if left == right {
            prop_assert_eq!(ContentHash::of_bytes(&left), ContentHash::of_bytes(&right));
        } else {
            prop_assert_ne!(ContentHash::of_bytes(&left), ContentHash::of_bytes(&right));
        }
    }

    /// Every hash reached through any route reads back from its own text.
    #[test]
    fn any_hash_round_trips_through_its_hex(bytes in proptest::collection::vec(any::<u8>(), 0..64)) {
        let hash = ContentHash::of_bytes(&bytes);
        prop_assert_eq!(hash.to_string().parse::<ContentHash>().unwrap(), hash);
        prop_assert_eq!(ContentHash::from_bytes(*hash.as_bytes()), hash);
    }
}
