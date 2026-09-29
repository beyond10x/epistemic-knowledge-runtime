//! Adversary, wave perf-02, `story:commit-hashes-the-graph-once`: the streamed address against
//! an oracle written from the module's rules rather than from the encoder.
//!
//! `ContentHash::of` documents itself as "the digest of `VALUE_DOMAIN` followed by exactly the
//! bytes `Canonical::canonical_bytes` returns". The unit's own tests compare the streamed digest
//! with the buffering encoder, and both now run the rewritten `map`, `ordered_map` and
//! `ordered_set`. The oracle here spells the bytes out by the tag table and rule 3 of
//! `ekr_core::canonical` (a map in key order, repeated keys by their encoded value), so a change
//! both encoders share cannot pass unseen.

use std::collections::BTreeMap;

use ekr_core::{Canonical, ContentHash, Encoder};
use sha2::{Digest, Sha256};

const STRING: u8 = 0x06;
const UNSIGNED: u8 = 0x04;
const LIST: u8 = 0x08;
const SET: u8 = 0x09;
const MAP: u8 = 0x0a;

/// A value tree that reaches every container path the unit rewrote.
#[derive(Clone, Debug)]
enum V {
    Str(String),
    U(u64),
    List(Vec<V>),
    /// Handed to `Encoder::set` in the order held, duplicates included.
    Set(Vec<String>),
    /// Handed to `Encoder::map` in the order held, repeated keys included.
    Map(Vec<(String, V)>),
    /// Encoded through the `BTreeMap` implementation.
    Tree(BTreeMap<String, V>),
}

impl Canonical for V {
    fn encode(&self, out: &mut Encoder) {
        match self {
            Self::Str(text) => out.string(text),
            Self::U(n) => out.unsigned(u128::from(*n)),
            Self::List(items) => out.list(items.iter()),
            Self::Set(items) => out.set(items.iter()),
            Self::Map(entries) => out.map(entries.iter().map(|(key, value)| (key, value))),
            Self::Tree(map) => map.encode(out),
        }
    }
}

fn length(out: &mut Vec<u8>, n: usize) {
    out.extend_from_slice(&(n as u64).to_be_bytes());
}

/// The bytes the module's rules give `value`, computed without `Encoder`.
fn spec(value: &V) -> Vec<u8> {
    let mut out = Vec::new();
    match value {
        V::Str(text) => {
            out.push(STRING);
            length(&mut out, text.len());
            out.extend_from_slice(text.as_bytes());
        }
        V::U(n) => {
            out.push(UNSIGNED);
            out.extend_from_slice(&u128::from(*n).to_be_bytes());
        }
        V::List(items) => {
            out.push(LIST);
            length(&mut out, items.len());
            for item in items {
                out.extend(spec(item));
            }
        }
        V::Set(items) => {
            let mut items = items.clone();
            items.sort();
            out.push(SET);
            length(&mut out, items.len());
            for item in items {
                out.extend(spec(&V::Str(item)));
            }
        }
        V::Map(entries) => {
            let mut entries: Vec<(String, Vec<u8>)> = entries
                .iter()
                .map(|(key, value)| (key.clone(), spec(value)))
                .collect();
            entries.sort();
            out.push(MAP);
            length(&mut out, entries.len());
            for (key, value) in entries {
                out.extend(spec(&V::Str(key)));
                out.extend(value);
            }
        }
        V::Tree(map) => {
            out.push(MAP);
            length(&mut out, map.len());
            for (key, value) in map {
                out.extend(spec(&V::Str(key.clone())));
                out.extend(spec(value));
            }
        }
    }
    out
}

fn value_address(bytes: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(ContentHash::VALUE_DOMAIN);
    digest.update(bytes);
    hex::encode(digest.finalize())
}

/// A fixed-seed generator, so the cases are the same on every run.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    /// A key from a small alphabet, so maps repeat keys often.
    fn key(&mut self) -> String {
        ["a", "b", "c", "ab", ""][usize::try_from(self.below(5)).unwrap()].to_owned()
    }
    /// A string whose length sits around the 64 KiB window more often than chance would put it.
    fn text(&mut self) -> String {
        let window = 64 * 1024;
        let len = match self.below(6) {
            0 => 0,
            1 => usize::try_from(self.below(40)).unwrap(),
            2 => window - 9 - usize::try_from(self.below(3)).unwrap(),
            3 => window + usize::try_from(self.below(3)).unwrap(),
            4 => window - 1,
            _ => usize::try_from(self.below(3_000)).unwrap(),
        };
        let byte = b'a' + u8::try_from(self.below(26)).unwrap();
        String::from_utf8(vec![byte; len]).unwrap()
    }
    fn value(&mut self, depth: u32) -> V {
        let pick = if depth == 0 {
            self.below(2)
        } else {
            self.below(6)
        };
        match pick {
            0 => V::Str(self.text()),
            1 => V::U(self.next()),
            2 => V::List((0..self.below(4)).map(|_| self.value(depth - 1)).collect()),
            3 => V::Set((0..self.below(5)).map(|_| self.key()).collect()),
            4 => V::Map(
                (0..self.below(5))
                    .map(|_| (self.key(), self.value(depth - 1)))
                    .collect(),
            ),
            _ => V::Tree(
                (0..self.below(5))
                    .map(|_| (self.key(), self.value(depth - 1)))
                    .collect(),
            ),
        }
    }
}

#[test]
fn every_container_path_encodes_and_streams_to_the_documented_bytes() {
    let mut rng = Lcg(0x5eed_0002);
    for case in 0..400 {
        let value = rng.value(3);
        let expected = spec(&value);
        assert!(
            value.canonical_bytes() == expected,
            "case {case}: canonical_bytes differ from the rules"
        );
        assert_eq!(
            ContentHash::of(&value).to_hex(),
            value_address(&expected),
            "case {case}: the streamed address differs from the address over the rules' bytes"
        );
    }
}

#[test]
fn repeated_keys_with_values_larger_than_the_window_order_by_value() {
    // Two entries under one key, each value longer than a window, handed largest-first: the
    // repeated-key path encodes them into their own buffers and writes each in one run.
    let big = |byte: u8| V::Str(String::from_utf8(vec![byte; 70_000]).unwrap());
    let value = V::Map(vec![
        ("k".to_owned(), big(b'z')),
        ("k".to_owned(), big(b'a')),
        ("j".to_owned(), V::Tree(BTreeMap::new())),
        ("k".to_owned(), V::Map(Vec::new())),
    ]);
    let expected = spec(&value);
    assert!(value.canonical_bytes() == expected);
    assert_eq!(ContentHash::of(&value).to_hex(), value_address(&expected));
}

/// An implementation that reads its encoder while it writes: the trait's documentation does not
/// forbid it, and `Encoder::as_bytes` is public.
struct Introspecting;
impl Canonical for Introspecting {
    fn encode(&self, out: &mut Encoder) {
        out.string(&"x".repeat(70_000));
        let held = out.as_bytes().len();
        out.unsigned(held as u128);
    }
}

/// `ContentHash::of` promises, for every `T: Canonical`, the digest of the value domain followed
/// by exactly the bytes `canonical_bytes` returns. Streaming keeps that promise only for an
/// implementation that never looks at `as_bytes`, so on the hashing encoder `as_bytes` refuses by
/// name instead of answering with part of the encoding.
#[test]
#[should_panic(expected = "as_bytes is unavailable on a hashing encoder")]
fn the_address_is_the_digest_over_canonical_bytes_for_every_implementation() {
    let _ = ContentHash::of(&Introspecting);
}
