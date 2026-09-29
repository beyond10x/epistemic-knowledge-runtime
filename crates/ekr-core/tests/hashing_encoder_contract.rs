//! Every public way an implementation can observe or take its encoder gives the same address on
//! the encoder [`ContentHash::of`] hashes through as on a buffering one, or refuses by name
//! (`story:commit-hashes-the-graph-once`, adversary finding on wave perf-02).
//!
//! [`ContentHash::of`] is the digest of the value domain followed by exactly the bytes
//! [`Canonical::canonical_bytes`] returns, for every `T: Canonical`. The hashing encoder holds
//! only a window of those bytes, so anything that reads what it holds — `as_bytes`, `finish`,
//! `Debug` — or moves it out of the `&mut Encoder` an implementation is handed would see less
//! than a buffering encoder does. The public surface of [`Encoder`] that is not a writer:
//!
//! | member | on a hashing encoder |
//! |---|---|
//! | `as_bytes` | refuses (`crates/ekr-core/tests/adversary_perf02_streaming_hash.rs`) |
//! | `finish`, reached by moving the encoder out | refuses |
//! | `Debug` | the same text on both |
//! | `Default`, through `std::mem::replace`/`take` | the address of what is left in its place |
//! | `std::mem::swap` there and back | unchanged |
//!
//! The writers (`bytes` … `option`) append the same bytes on both, which every vector holds.

use ekr_core::{Canonical, ContentHash, Encoder};
use sha2::{Digest, Sha256};

/// The value-domain digest over `value`'s buffered canonical bytes.
fn over_canonical_bytes<T: Canonical>(value: &T) -> ContentHash {
    let mut digest = Sha256::new();
    digest.update(ContentHash::VALUE_DOMAIN);
    digest.update(value.canonical_bytes());
    ContentHash::from_bytes(digest.finalize().into())
}

/// Takes the encoder it was handed, finishes it and writes those bytes into a fresh one.
struct Finishing;
impl Canonical for Finishing {
    fn encode(&self, out: &mut Encoder) {
        out.string("before");
        let bytes = std::mem::take(out).finish();
        out.bytes(&bytes);
    }
}

#[test]
#[should_panic(expected = "finish is unavailable on a hashing encoder")]
fn finishing_a_hashing_encoder_is_refused() {
    assert_eq!(Finishing.canonical_bytes().len(), 1 + 8 + (1 + 8 + 6));
    let _ = ContentHash::of(&Finishing);
}

/// Replaces the encoder it was handed with a fresh one and drops the original.
struct Replacing;
impl Canonical for Replacing {
    fn encode(&self, out: &mut Encoder) {
        out.string("dropped");
        drop(std::mem::replace(out, Encoder::new()));
        out.string("kept");
    }
}

#[test]
fn replacing_the_encoder_addresses_what_is_left_in_its_place() {
    assert_eq!(
        ContentHash::of(&Replacing),
        over_canonical_bytes(&Replacing)
    );
}

/// Writes its encoder's `Debug` text into it.
struct Debugging;
impl Canonical for Debugging {
    fn encode(&self, out: &mut Encoder) {
        out.string(&"x".repeat(70_000));
        let text = format!("{out:?}");
        out.string(&text);
    }
}

#[test]
fn an_encoder_reads_the_same_through_debug_on_both() {
    assert_eq!(
        ContentHash::of(&Debugging),
        over_canonical_bytes(&Debugging)
    );
}

/// Swaps a fresh encoder in to encode a part on its own, then swaps the original back.
struct Swapping;
impl Canonical for Swapping {
    fn encode(&self, out: &mut Encoder) {
        out.string("head");
        let mut part = Encoder::new();
        std::mem::swap(out, &mut part);
        out.string(&"p".repeat(70_000));
        std::mem::swap(out, &mut part);
        out.bytes(&part.finish());
    }
}

#[test]
fn swapping_an_encoder_there_and_back_changes_no_address() {
    assert_eq!(ContentHash::of(&Swapping), over_canonical_bytes(&Swapping));
}
