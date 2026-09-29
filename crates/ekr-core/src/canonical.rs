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
//! 5. **Structural for newtypes, tagged for sum types.** A value encodes as the *shape* it has,
//!    so a newtype encodes as the thing it wraps: `RevisionNumber(7)` produces the bytes of
//!    `7u64`, and a `NodeId` produces the bytes of an `EdgeId` over the same UUID. That is the
//!    contract rather than an oversight. Every artefact this runtime content-addresses — an
//!    observation, a piece of evidence, an assertion, a transaction, a revision root — is a
//!    structured value whose own encoding carries its field structure, so a field's position
//!    already distinguishes it from a field of another type in the same position, and no artefact
//!    is a bare newtype. A per-type discriminant would cost a stable tag per type forever, and a
//!    rename or a reordering of that registry would silently move every address the type ever
//!    reached — a durable cost against a collision the runtime cannot reach. A **sum type** is
//!    the other way round: two variants carrying the same payload shape would collide, and
//!    nothing about their position distinguishes them, so a sum type carries a variant tag
//!    written by [`Encoder::variant`] and by nothing else. Settled on 2026-09-21 by
//!    `task:canonical-newtype-discriminant`; the writer landed with the first sum type that needed
//!    it, `ekr-graph`'s `RevisionEvent`.
//!
//! The bytes are an internal format, not a wire format: nothing outside this runtime reads them,
//! and they are not a serialisation — there is no decoder, because a hash never needs one.

use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest, Sha256};

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
    pub(super) const VARIANT: u8 = 0x0f;
}

/// A value with one deterministic byte encoding.
///
/// Implement it by writing tagged, length-prefixed bytes into the buffer — the helpers on
/// [`Encoder`] are there so an implementation cannot forget a tag or a length. A composite value
/// encodes its fields in a fixed order that its own implementation fixes; two values of one type
/// must encode equally exactly when they are equal.
pub trait Canonical {
    /// Appends this value's canonical bytes to `out`.
    ///
    /// An implementation writes; it does not read `out`. [`ContentHash::of`] hands it an encoder
    /// that hashes the bytes as they arrive and holds only a window of them, so there
    /// [`Encoder::as_bytes`] and [`Encoder::finish`] refuse rather than answer with part of the
    /// encoding, and `Debug` shows no content on any encoder. Encoding a part on its own takes an
    /// [`Encoder::new`] of its own. What `ContentHash::of` addresses is the encoder left in `out`
    /// when this returns, which is what [`Canonical::canonical_bytes`] returns too.
    fn encode(&self, out: &mut Encoder);

    /// This value's canonical bytes, on their own.
    #[must_use]
    fn canonical_bytes(&self) -> Vec<u8> {
        let mut encoder = Encoder::new();
        self.encode(&mut encoder);
        encoder.finish()
    }
}

/// How many unhashed bytes an encoder that hashes as it writes holds before it hashes them.
const WINDOW: usize = 64 * 1024;

/// The buffer a [`Canonical`] implementation writes into.
///
/// Every write goes through a method that emits the tag and the length prefix together, so the
/// two rules that keep the encoding unambiguous are kept in one place rather than at each call
/// site.
///
/// The encoder [`ContentHash::of`] writes through hashes the bytes as they arrive and holds at
/// most a window of them, so an address over a whole graph never holds that graph's encoding in
/// memory. It produces the digest of exactly the bytes a buffering encoder would hold.
#[derive(Default)]
pub struct Encoder {
    bytes: Vec<u8>,
    /// Where the bytes go on an encoder that hashes as it writes: the digest of its domain and of
    /// every byte written before `bytes`. `None` on a buffering encoder, which keeps them all.
    digest: Option<Sha256>,
}

/// The same text for every encoder: what one holds is not part of it, because the hashing encoder
/// holds only a window and an implementation that formatted it would change its own address.
impl std::fmt::Debug for Encoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Encoder").finish_non_exhaustive()
    }
}

impl Encoder {
    /// An empty encoder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An empty encoder that hashes [`ContentHash::VALUE_DOMAIN`] and then every byte written
    /// into it, holding at most a window of them.
    pub(crate) fn hashing_values() -> Self {
        let mut digest = Sha256::new();
        digest.update(ContentHash::VALUE_DOMAIN);
        Self {
            bytes: Vec::new(),
            digest: Some(digest),
        }
    }

    /// The value-domain digest of every byte written into this encoder. An implementation that
    /// put a buffering encoder in the hashing one's place (`std::mem::replace`) left that one's
    /// bytes to be addressed, which are the bytes a buffering encoder would hold too.
    pub(crate) fn value_digest(self) -> [u8; 32] {
        let mut digest = self.digest.unwrap_or_else(|| {
            let mut digest = Sha256::new();
            digest.update(ContentHash::VALUE_DOMAIN);
            digest
        });
        digest.update(&self.bytes);
        digest.finalize().into()
    }

    /// Refuses on the hashing encoder, which no longer holds what it hashed: a partial answer
    /// would give the implementation that read it a different address.
    fn buffering(&self, what: &str) {
        assert!(
            self.digest.is_none(),
            "{what} is unavailable on a hashing encoder"
        );
    }

    /// The bytes written so far.
    ///
    /// # Panics
    ///
    /// On the encoder [`ContentHash::of`] writes through, moved out of the `&mut Encoder` an
    /// implementation is handed: `finish is unavailable on a hashing encoder`.
    #[must_use]
    pub fn finish(self) -> Vec<u8> {
        self.buffering("finish");
        self.bytes
    }

    /// The bytes written so far, without consuming the encoder.
    ///
    /// # Panics
    ///
    /// On the encoder [`ContentHash::of`] writes through: `as_bytes is unavailable on a hashing
    /// encoder`.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        self.buffering("as_bytes");
        &self.bytes
    }

    /// Appends `data`: to the buffer, or on a hashing encoder to the window, hashing the window
    /// once it is full. A run as long as a window is hashed where it lies, never copied.
    fn write(&mut self, data: &[u8]) {
        match &mut self.digest {
            None => self.bytes.extend_from_slice(data),
            Some(digest) if data.len() >= WINDOW => {
                digest.update(&self.bytes);
                self.bytes.clear();
                digest.update(data);
            }
            Some(digest) => {
                self.bytes.extend_from_slice(data);
                if self.bytes.len() >= WINDOW {
                    digest.update(&self.bytes);
                    self.bytes.clear();
                }
            }
        }
    }

    fn tag(&mut self, tag: u8) {
        self.write(&[tag]);
    }

    fn length(&mut self, length: usize) {
        self.write(&(length as u64).to_be_bytes());
    }

    /// A tagged, length-prefixed run of raw bytes.
    pub fn bytes(&mut self, bytes: &[u8]) {
        self.tag(tag::BYTES);
        self.length(bytes.len());
        self.write(bytes);
    }

    /// A tagged, length-prefixed string, as UTF-8.
    pub fn string(&mut self, text: &str) {
        self.tag(tag::STRING);
        self.length(text.len());
        self.write(text.as_bytes());
    }

    /// An unsigned integer, widened to sixteen big-endian bytes so that the same number encodes
    /// the same whichever width it was held in.
    pub fn unsigned(&mut self, value: u128) {
        self.tag(tag::UNSIGNED);
        self.write(&value.to_be_bytes());
    }

    /// A signed integer, widened to sixteen big-endian bytes. A negative number and an unsigned
    /// one never share an encoding: the tag differs.
    pub fn signed(&mut self, value: i128) {
        self.tag(tag::SIGNED);
        self.write(&value.to_be_bytes());
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
        self.write(&bits.to_be_bytes());
    }

    /// A content hash: its thirty-two bytes, which are already fixed-width.
    pub fn hash(&mut self, digest: &[u8; 32]) {
        self.tag(tag::HASH);
        self.write(digest);
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
        self.ordered_set(items.into_iter());
    }

    /// [`Encoder::set`] over elements already in ascending order, as a [`BTreeSet`] yields them.
    fn ordered_set<'a, T: Canonical + 'a>(&mut self, items: impl ExactSizeIterator<Item = &'a T>) {
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
    ///
    /// Where no key repeats, key order alone is that order, and each value is encoded where it
    /// lies instead of into a buffer of its own; only a repeated key has its values encoded first,
    /// to be ordered by their bytes.
    pub fn map<'a, K: Canonical + Ord + 'a, V: Canonical + 'a>(
        &mut self,
        entries: impl ExactSizeIterator<Item = (&'a K, &'a V)>,
    ) {
        let mut entries: Vec<(&K, &V)> = entries.collect();
        entries.sort_by(|left, right| left.0.cmp(right.0));
        if entries.windows(2).all(|pair| pair[0].0 != pair[1].0) {
            self.ordered_map(entries.into_iter());
            return;
        }
        let mut entries: Vec<(&K, Vec<u8>)> = entries
            .into_iter()
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
            self.write(&value);
        }
    }

    /// [`Encoder::map`] over entries whose keys are already strictly ascending, as a [`BTreeMap`]
    /// yields them: each value is encoded where it lies.
    fn ordered_map<'a, K: Canonical + 'a, V: Canonical + 'a>(
        &mut self,
        entries: impl ExactSizeIterator<Item = (&'a K, &'a V)>,
    ) {
        self.tag(tag::MAP);
        self.length(entries.len());
        for (key, value) in entries {
            key.encode(self);
            value.encode(self);
        }
    }

    /// The marker that opens a sum type's variant: the tag, then `index` as four big-endian
    /// bytes. The variant's own fields follow it.
    ///
    /// This is the one exception to rule 5, and the only path in the workspace that writes a
    /// discriminant. A newtype is structural because its field's position already distinguishes
    /// it inside the value that holds it; two variants of one sum type have *the same* position,
    /// so two variants carrying the same payload shape would collide and a hash equality would be
    /// a lie. Settled by `task:canonical-newtype-discriminant` on 2026-09-21.
    ///
    /// `index` identifies the variant, and it is part of the contract: **changing a variant's
    /// number moves every content address that contains it.** It is whatever the implementing
    /// type passes — this encoder derives nothing from a declaration order and cannot, so a type
    /// whose numbering and whose variant list disagree is a type with a silent defect rather than
    /// a compile error. The obligation is on the implementor to pin its own mapping;
    /// `crates/ekr-graph/tests/revision_events.rs` does that for `RevisionEvent`, both by
    /// transcribing the six numbers and by reading its source for the declaration order.
    pub fn variant(&mut self, index: u32) {
        self.tag(tag::VARIANT);
        self.write(&index.to_be_bytes());
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
    ///
    /// The map yields its keys strictly ascending under that same `Ord`, which is the order
    /// [`Encoder::map`] imposes, so the entries are written as they come: the bytes are the ones
    /// `Encoder::map` would write, without collecting the entries first.
    fn encode(&self, out: &mut Encoder) {
        out.ordered_map(self.iter());
    }
}

impl<T: Canonical + Ord> Canonical for BTreeSet<T> {
    /// In the set's own sorted order, for the same reason a map encodes in key order: the order
    /// [`Encoder::set`] imposes, so the elements are written as they come.
    fn encode(&self, out: &mut Encoder) {
        out.ordered_set(self.iter());
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

#[cfg(test)]
mod tests {
    use super::{Canonical, Encoder};
    use crate::ContentHash;
    use std::cell::Cell;

    /// The most unhashed bytes an encoder may hold while [`ContentHash::of`] writes into it.
    const BOUND: usize = 1 << 20;

    /// `chunks` strings of `chunk` bytes each, noting the most bytes its encoder held at any
    /// point. It reads the encoder's own field: `as_bytes` refuses on the hashing encoder.
    struct Large {
        chunks: usize,
        chunk: usize,
        held: Cell<usize>,
    }
    impl Canonical for Large {
        fn encode(&self, out: &mut Encoder) {
            for n in 0..self.chunks {
                let byte = b'a' + u8::try_from(n % 26).expect("a letter");
                out.string(&String::from_utf8(vec![byte; self.chunk]).expect("ascii"));
                self.held.set(self.held.get().max(out.bytes.len()));
            }
        }
    }

    /// `story:commit-hashes-the-graph-once`: a knowledge root is the address of a whole graph,
    /// and building that encoding in memory before hashing it held the graph's bytes a second
    /// time and reallocated the buffer on the way up (12% of a commit's hashing in the
    /// performance audit of 2026-09-29). [`ContentHash::of`] holds only a bounded window.
    #[test]
    fn hashing_a_value_holds_a_bounded_window_of_its_encoding() {
        // Sixteen MiB of strings: the whole encoding, held at once, is sixteen times the bound.
        let large = Large {
            chunks: 4096,
            chunk: 4096,
            held: Cell::new(0),
        };
        let _ = ContentHash::of(&large);
        assert!(
            large.held.get() <= BOUND,
            "ContentHash::of held {} bytes of the encoding at once; the bound is {BOUND}",
            large.held.get()
        );
        // A buffering encoder still holds every byte, which is what `canonical_bytes` returns.
        let buffered = Large {
            chunks: 4096,
            chunk: 4096,
            held: Cell::new(0),
        };
        assert_eq!(buffered.canonical_bytes().len(), 4096 * (1 + 8 + 4096));
        assert_eq!(buffered.held.get(), 4096 * (1 + 8 + 4096));
    }
}
