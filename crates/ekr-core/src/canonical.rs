//! The canonical encoding: the deterministic bytes a [`ContentHash`] is
//! computed over.
//!
//! Design § 57 content-addresses immutable artifacts, and an address is only stable if the bytes
//! behind it are a function of the value alone — never of insertion order, iteration order, a
//! serialiser's field order, a locale, a clock or an allocation. `serde` is not that function:
//! its output depends on the format, and JSON in particular has no ordering rule for object keys
//! and no canonical number form. So the encoding is its own trait and its own bytes.
//!
//! # The rules the encoding keeps
//!
//! 1. **Tagged.** Every value starts with a byte naming its shape, so a string and a number that
//!    read the same do not encode the same.
//! 2. **Length-prefixed.** Every variable-length value carries its length as eight big-endian
//!    bytes before its content, so `["ab", "c"]` and `["a", "bc"]` cannot collide.
//! 3. **Ordered by the value, not by the writer.** A map encodes in key order and a set in
//!    element order, so two maps with the same entries encode identically whatever order they
//!    were built in. [`Encoder::map`] and [`Encoder::set`] *impose* that order rather than trust
//!    the iterator they are handed: a [`BTreeMap`] already arrives sorted, but an implementation
//!    over a `HashMap` field — which is what this trait is published for — would otherwise hand
//!    over its own iteration order, which is a function of the process's hash seed.
//! 4. **No floats.** `f32` and `f64` have no `Canonical` implementation — not in a key, and not
//!    anywhere else. `NaN` is not equal to itself, `0.0 == -0.0` holds for two different bit
//!    patterns, and no encoding of either can be both total and faithful to equality. A quantity
//!    that must be hashed is carried as an integer or a decimal string, which is also what
//!    `ekr.ontology.ValueKind` distinguishes `Float` from `Decimal` for.
//! 5. **Structural, not nominal.** A value encodes as the *shape* it has, and a newtype encodes
//!    as the thing it wraps: `RevisionNumber(7)` produces the bytes of `7u64`, and a `NodeId`
//!    produces the bytes of an `EdgeId` over the same UUID. Two distinct types over one shape
//!    therefore share a content address. Nothing in this crate is wrong today — no composite
//!    type exists yet to hold such a field — but a later struct that changes a field from `u64`
//!    to `RevisionNumber`, or from `NodeId` to `EdgeId`, would keep an address that ought to
//!    move. Whether the encoding grows a per-type discriminant is decided by
//!    `task:canonical-newtype-discriminant`, which blocks `story:commit-and-revision-lineage`;
//!    until it is, treat the address of a bare primitive as saying nothing about its type.
//!
//! The bytes are an internal format, not a wire format: nothing outside this runtime reads them,
//! and they are not a serialisation — there is no decoder, because a hash never needs one.

use std::collections::{BTreeMap, BTreeSet};

use crate::hash::ContentHash;
use crate::identity::RevisionNumber;

/// The tag byte that opens each shape's encoding.
mod tag {
    pub(super) const UNIT: u8 = 0x01;
    pub(super) const FALSE: u8 = 0x02;
    pub(super) const TRUE: u8 = 0x03;
    pub(super) const UNSIGNED: u8 = 0x04;
    pub(super) const SIGNED: u8 = 0x05;
    pub(super) const STRING: u8 = 0x06;
    pub(super) const BYTES: u8 = 0x07;
    pub(super) const LIST: u8 = 0x08;
    pub(super) const SET: u8 = 0x09;
    pub(super) const MAP: u8 = 0x0a;
    pub(super) const NONE: u8 = 0x0b;
    pub(super) const SOME: u8 = 0x0c;
    pub(super) const ID: u8 = 0x0d;
    pub(super) const HASH: u8 = 0x0e;
}

/// A value with one deterministic byte encoding.
///
/// Implement it by writing tagged, length-prefixed bytes into the buffer — the helpers on
/// [`Encoder`] are there so an implementation cannot forget a tag or a length. A composite value
/// encodes its fields in a fixed order that its own implementation fixes; two values of one type
/// must encode equally exactly when they are equal.
pub trait Canonical {
    /// Appends this value's canonical bytes to `out`.
    fn encode(&self, out: &mut Encoder);

    /// This value's canonical bytes, on their own.
    #[must_use]
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut encoder = Encoder::new();
        self.encode(&mut encoder);
        encoder.finish()
    }
}

/// The buffer a [`Canonical`] implementation writes into.
///
/// Every write goes through a method that emits the tag and the length prefix together, so the
/// two rules that keep the encoding unambiguous are kept in one place rather than at each call
/// site.
#[derive(Debug, Default)]
pub struct Encoder {
    bytes: Vec<u8>,
}

impl Encoder {
    /// An empty encoder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The bytes written so far.
    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        self.bytes
    }

    /// The bytes written so far, without consuming the encoder.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn tag(&mut self, tag: u8) {
        self.bytes.push(tag);
    }

    fn length(&mut self, length: usize) {
        self.bytes.extend_from_slice(&(length as u64).to_be_bytes());
    }

    /// A tagged, length-prefixed run of raw bytes.
    pub fn bytes(&mut self, bytes: &[u8]) {
        self.tag(tag::BYTES);
        self.length(bytes.len());
        self.bytes.extend_from_slice(bytes);
    }

    /// A tagged, length-prefixed string, as UTF-8.
    pub fn string(&mut self, text: &str) {
        self.tag(tag::STRING);
        self.length(text.len());
        self.bytes.extend_from_slice(text.as_bytes());
    }

    /// An unsigned integer, widened to sixteen big-endian bytes so that the same number encodes
    /// the same whichever width it was held in.
    pub fn unsigned(&mut self, value: u128) {
        self.tag(tag::UNSIGNED);
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    /// A signed integer, widened to sixteen big-endian bytes. A negative number and an unsigned
    /// one never share an encoding: the tag differs.
    pub fn signed(&mut self, value: i128) {
        self.tag(tag::SIGNED);
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    /// A boolean.
    pub fn boolean(&mut self, value: bool) {
        self.tag(if value { tag::TRUE } else { tag::FALSE });
    }

    /// The unit value — a field that is present and carries nothing.
    pub fn unit(&mut self) {
        self.tag(tag::UNIT);
    }

    /// An id: sixteen big-endian bytes of UUID.
    pub fn id(&mut self, bits: u128) {
        self.tag(tag::ID);
        self.bytes.extend_from_slice(&bits.to_be_bytes());
    }

    /// A content hash: its thirty-two bytes, which are already fixed-width.
    pub fn hash(&mut self, digest: &[u8; 32]) {
        self.tag(tag::HASH);
        self.bytes.extend_from_slice(digest);
    }

    /// An ordered sequence: the count, then each element in its own order.
    pub fn list<'a, T: Canonical + 'a>(&mut self, items: impl ExactSizeIterator<Item = &'a T>) {
        self.tag(tag::LIST);
        self.length(items.len());
        for item in items {
            item.encode(self);
        }
    }

    /// An unordered set: the count, then each element in ascending order of the element.
    ///
    /// The order is imposed here, not assumed of the caller. A [`BTreeSet`] already iterates in
    /// it, so its bytes are unchanged; an implementation over an unordered container — the case
    /// this trait is published for — gets the same bytes rather than its own iteration order,
    /// which for a `HashSet` is a function of the process's hash seed.
    pub fn set<'a, T: Canonical + Ord + 'a>(
        &mut self,
        items: impl ExactSizeIterator<Item = &'a T>,
    ) {
        let mut items: Vec<&T> = items.collect();
        items.sort_unstable();
        self.tag(tag::SET);
        self.length(items.len());
        for item in items {
            item.encode(self);
        }
    }

    /// A map: the count, then each key and value in ascending key order, and for entries sharing
    /// a key, in ascending order of the encoded value.
    ///
    /// Sorted here for the reason [`Encoder::set`] sorts, and sorted all the way down for the
    /// same reason: a repeated key is the one case in which ordering by key alone would leave the
    /// writer's order in the bytes, and rule 3 of this module holds without qualification or it
    /// is not a rule. Repeated keys are not deduplicated — an entry the caller passed twice is
    /// two entries, and a map cannot produce one at all.
    pub fn map<'a, K: Canonical + Ord + 'a, V: Canonical + 'a>(
        &mut self,
        entries: impl ExactSizeIterator<Item = (&'a K, &'a V)>,
    ) {
        let mut entries: Vec<(&K, Vec<u8>)> = entries
            .map(|(key, value)| {
                let mut encoded = Encoder::new();
                value.encode(&mut encoded);
                (key, encoded.finish())
            })
            .collect();
        entries.sort_by(|left, right| left.0.cmp(right.0).then_with(|| left.1.cmp(&right.1)));
        self.tag(tag::MAP);
        self.length(entries.len());
        for (key, value) in entries {
            key.encode(self);
            self.bytes.extend_from_slice(&value);
        }
    }

    /// An optional value. Absence is not emptiness and does not encode as it.
    pub fn option<T: Canonical>(&mut self, value: Option<&T>) {
        match value {
            None => self.tag(tag::NONE),
            Some(inner) => {
                self.tag(tag::SOME);
                inner.encode(self);
            }
        }
    }
}

macro_rules! canonical_unsigned {
    ($($type:ty),+ $(,)?) => {
        $(
            impl Canonical for $type {
                fn encode(&self, out: &mut Encoder) {
                    out.unsigned(u128::from(*self));
                }
            }
        )+
    };
}

macro_rules! canonical_signed {
    ($($type:ty),+ $(,)?) => {
        $(
            impl Canonical for $type {
                fn encode(&self, out: &mut Encoder) {
                    out.signed(i128::from(*self));
                }
            }
        )+
    };
}

canonical_unsigned!(u8, u16, u32, u64, u128);
canonical_signed!(i8, i16, i32, i64, i128);

impl Canonical for usize {
    fn encode(&self, out: &mut Encoder) {
        out.unsigned(*self as u128);
    }
}

impl Canonical for isize {
    fn encode(&self, out: &mut Encoder) {
        out.signed(*self as i128);
    }
}

impl Canonical for bool {
    fn encode(&self, out: &mut Encoder) {
        out.boolean(*self);
    }
}

impl Canonical for () {
    fn encode(&self, out: &mut Encoder) {
        out.unit();
    }
}

impl Canonical for str {
    fn encode(&self, out: &mut Encoder) {
        out.string(self);
    }
}

impl Canonical for String {
    fn encode(&self, out: &mut Encoder) {
        out.string(self);
    }
}

impl<T: Canonical> Canonical for Vec<T> {
    fn encode(&self, out: &mut Encoder) {
        out.list(self.iter());
    }
}

impl<T: Canonical> Canonical for [T] {
    fn encode(&self, out: &mut Encoder) {
        out.list(self.iter());
    }
}

impl<T: Canonical> Canonical for Option<T> {
    fn encode(&self, out: &mut Encoder) {
        out.option(self.as_ref());
    }
}

impl<T: Canonical + ?Sized> Canonical for &T {
    fn encode(&self, out: &mut Encoder) {
        (**self).encode(out);
    }
}

impl<T: Canonical + ?Sized> Canonical for Box<T> {
    fn encode(&self, out: &mut Encoder) {
        (**self).encode(out);
    }
}

impl<K: Canonical + Ord, V: Canonical> Canonical for BTreeMap<K, V> {
    /// In key order, which is the map's own order — insertion order never reaches the bytes.
    ///
    /// A key is a `Canonical + Ord` type, and no float is either, so a float key is a compile
    /// error rather than a rule someone has to remember.
    fn encode(&self, out: &mut Encoder) {
        out.map(self.iter());
    }
}

impl<T: Canonical + Ord> Canonical for BTreeSet<T> {
    /// In the set's own sorted order, for the same reason a map encodes in key order.
    fn encode(&self, out: &mut Encoder) {
        out.set(self.iter());
    }
}

impl Canonical for ContentHash {
    fn encode(&self, out: &mut Encoder) {
        out.hash(self.as_bytes());
    }
}

impl Canonical for RevisionNumber {
    fn encode(&self, out: &mut Encoder) {
        out.unsigned(u128::from(self.get()));
    }
}
