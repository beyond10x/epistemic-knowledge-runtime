//! A value's content address is taken over its canonical encoding as a stream
//! (`story:commit-hashes-the-graph-once`).
//!
//! A knowledge root is the address of a whole graph. Building that encoding in memory before
//! hashing it held the graph's bytes a second time and reallocated the buffer on the way up, which
//! the performance audit of 2026-09-29 measured at 12% of a commit's hashing. [`ContentHash::of`]
//! therefore hashes the bytes as they are written and holds only a bounded window of them.
//!
//! That the address does not move — SHA-256 over the value domain followed by exactly the bytes
//! [`Canonical::canonical_bytes`] returns, however many windows the encoding spans — is held
//! beside the hashing itself, in `crates/ekr-core/src/hash.rs`, and by the published vectors.

use std::cell::Cell;

use ekr_core::{Canonical, ContentHash, Encoder};

/// The most unhashed bytes an encoder may hold while [`ContentHash::of`] writes into it.
const BOUND: usize = 1 << 20;

/// A value that writes `chunks` strings of `chunk` bytes each, noting the most bytes its encoder
/// held at any point.
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
            self.held.set(self.held.get().max(out.as_bytes().len()));
        }
    }
}

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
    assert_eq!(large.canonical_bytes().len(), 4096 * (1 + 8 + 4096));
}
