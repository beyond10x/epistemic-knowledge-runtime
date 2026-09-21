//! Every public item of this crate is exercised by some case, and the list is not hand-kept.
//!
//! The adversary's finding 2 was an instance of a class: `ContentHash::to_hex` was public,
//! documented lowercase, and called by no case, so `hex::encode` could have become
//! `hex::encode_upper` with every case green. A hand-written list of "items to remember to test"
//! is the same defect one level up — it needs an adversary to extend it. So the class is checked
//! by machine: `no_public_item_is_untested` reads the crate's own source for `pub fn`, `pub const
//! fn` and `pub const` declarations and asserts each name appears in some file of this suite.
//!
//! The first version of this check counted a name that appeared *anywhere* in the suite text,
//! and the adversary found that the tree already contained an item satisfying it from prose:
//! `Encoder::list`, called by no case, passed on the English word "list" in three doc comments
//! and an assertion message. Two things were wrong with it and both are fixed here:
//!
//! * **prose does not count.** Comments are stripped before the suite is searched, so a name in a
//!   doc comment — including one written as `Type::name` — satisfies nothing.
//! * **the occurrence must be a use.** `::name` or `.name`, in a path or method position, not a
//!   bare word. This is the criterion `adversary2_public_surface.rs` states; a literal `(` is not
//!   required, because a `pub const` is used without one and a check no item can satisfy is
//!   worse than a loose one.
//!
//! It is still not a coverage measurement — a case can name an item and assert nothing about it —
//! so the cases below do the exercising, each with an assertion that fails if the item behaves
//! differently. What the check buys is that a *new* public item cannot arrive untested without
//! turning a case red.

use std::path::{Path, PathBuf};

use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{ContentHash, NodeId, RevisionNumber};

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Every `.rs` file at or below a directory, read — `src/` is flat today and a later module
/// directory must not become a hole in the scan.
fn rust_sources(directory: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    let entries = std::fs::read_dir(directory)
        .unwrap_or_else(|e| panic!("reading {}: {e}", directory.display()));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            found.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
            found.push((path, text));
        }
    }
    found
}

/// `text` with its comments removed: `//` to end of line, and `/* … */` including nesting.
///
/// String literals containing `//` would be cut too. That costs a false *positive* — an item
/// named only inside such a literal would be reported untested — and never a false negative,
/// which is the direction that matters for a guard.
fn without_comments(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(text.len());
    let mut at = 0;
    let mut depth = 0usize;
    while at < bytes.len() {
        if depth == 0 && bytes[at..].starts_with(b"//") {
            while at < bytes.len() && bytes[at] != b'\n' {
                at += 1;
            }
        } else if bytes[at..].starts_with(b"/*") {
            depth += 1;
            at += 2;
        } else if depth > 0 && bytes[at..].starts_with(b"*/") {
            depth -= 1;
            at += 2;
        } else {
            if depth == 0 {
                out.push(bytes[at]);
            }
            at += 1;
        }
    }
    // Every delimiter cut is ASCII, so no multi-byte character was split.
    String::from_utf8(out).expect("stripping ASCII delimiters keeps the text valid UTF-8")
}

/// Whether `code` uses `name` in a path or method position — `::name` or `.name`, with no
/// identifier character after it.
fn is_used(code: &str, name: &str) -> bool {
    let bytes = code.as_bytes();
    let mut from = 0;
    while let Some(offset) = code[from..].find(name) {
        let at = from + offset;
        from = at + 1;
        let after = at + name.len();
        if bytes
            .get(after)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
        {
            continue;
        }
        let before = &code[..at];
        if before.ends_with('.') || before.ends_with("::") {
            return true;
        }
    }
    false
}

/// The name declared by a `pub fn` / `pub const fn` / `pub const` line, if the line is one.
fn declared_public_name(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let rest = line
        .strip_prefix("pub const fn ")
        .or_else(|| line.strip_prefix("pub fn "))
        .or_else(|| line.strip_prefix("pub const "))?;
    let name = rest
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .next()?;
    (!name.is_empty()).then_some(name)
}

#[test]
fn no_public_item_is_untested() {
    let sources = rust_sources(&crate_root().join("src"));
    assert!(!sources.is_empty(), "the source scan found nothing");

    let read = rust_sources(&crate_root().join("tests"));
    let raw: String = read.iter().map(|(_, text)| text.as_str()).collect();
    let suite: String = read
        .iter()
        .map(|(_, text)| without_comments(text))
        .collect();

    // A check that silently stops stripping is a check that is back to counting prose, so the
    // stripper is measured rather than trusted. The marker is assembled from two pieces here and
    // written whole in the comment two lines below, so this line is not itself a match.
    let marker = concat!("stripper", "-sentinel");
    // stripper-sentinel — the one occurrence, and it is inside a comment.
    assert!(raw.contains(marker), "this file is not in the scan");
    assert!(
        !suite.contains(marker),
        "the comment stripper did nothing: a marker written only in a comment survived it"
    );

    let mut declared: Vec<String> = sources
        .iter()
        .flat_map(|(_, text)| text.lines())
        .filter_map(declared_public_name)
        .map(str::to_owned)
        .collect();
    declared.sort();
    declared.dedup();
    assert!(
        declared.len() > 20,
        "the declaration scan is broken, not the suite: {declared:?}"
    );

    let untested: Vec<&String> = declared
        .iter()
        .filter(|name| !is_used(&suite, name))
        .collect();

    assert!(
        untested.is_empty(),
        "public items no case uses outside a comment: {untested:?}"
    );
}

#[test]
fn an_id_carries_its_bits_through_uuid_and_back() {
    let id = NodeId::mint();

    assert_eq!(id.to_uuid().as_u128(), id.as_u128());
    assert_eq!(NodeId::from_uuid(id.to_uuid()), id);
    assert_eq!(id.to_uuid().get_version_num(), 7, "minted as UUIDv7");
    assert_eq!(id.to_string(), id.to_uuid().as_hyphenated().to_string());
}

#[test]
fn a_revision_number_counts_from_the_seed_and_stops_at_the_end() {
    assert_eq!(RevisionNumber::SEED.get(), 0);

    let first = RevisionNumber::SEED.next().expect("a seed has a successor");
    assert_eq!(first.get(), 1);
    assert_eq!(first, RevisionNumber::new(1));
    assert!(RevisionNumber::SEED < first, "the lineage is ordered");

    assert_eq!(
        RevisionNumber::new(u64::MAX).next(),
        None,
        "the counter must refuse to wrap: revision 0 already exists"
    );
}

#[test]
fn the_encoder_writes_each_shape_it_publishes() {
    // Each primitive writer, and the assertion that distinguishes it from its neighbours: the
    // tag byte is part of the encoding, so no two of these agree.
    let mut every_shape = Vec::new();
    for write in [
        (|out: &mut Encoder| out.unit()) as fn(&mut Encoder),
        |out: &mut Encoder| out.boolean(true),
        |out: &mut Encoder| out.boolean(false),
        |out: &mut Encoder| out.unsigned(1),
        |out: &mut Encoder| out.signed(1),
        |out: &mut Encoder| out.signed(-1),
        |out: &mut Encoder| out.string("a"),
        |out: &mut Encoder| out.bytes(b"a"),
        |out: &mut Encoder| out.id(1),
        |out: &mut Encoder| out.hash(&[0u8; 32]),
        |out: &mut Encoder| out.option(None::<&u64>.copied().as_ref()),
        |out: &mut Encoder| out.option(Some(&1u64)),
        // The three container writers, which the first version of this case omitted — and
        // `list` was the item the guard above was passing on an English word.
        |out: &mut Encoder| out.list([&1u64, &2u64].into_iter()),
        |out: &mut Encoder| out.set([&1u64, &2u64].into_iter()),
        |out: &mut Encoder| out.map([(&1u64, &2u64)].into_iter()),
        // The variant marker, added with `RevisionEvent` — the first sum type the runtime
        // encodes. Two indices, because a sum type's first two variants must not collide either.
        |out: &mut Encoder| out.variant(0),
        |out: &mut Encoder| out.variant(1),
    ] {
        let mut encoder = Encoder::new();
        write(&mut encoder);
        assert!(!encoder.as_bytes().is_empty(), "a shape wrote nothing");
        every_shape.push(encoder.finish());
    }

    let mut distinct = every_shape.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        every_shape.len(),
        "two shapes share an encoding"
    );

    // The writers agree with the `Canonical` impls that are built on them.
    let mut encoder = Encoder::new();
    encoder.list([&1u64, &2u64].into_iter());
    assert_eq!(encoder.finish(), vec![1u64, 2u64].canonical_bytes());

    let mut encoder = Encoder::new();
    encoder.set([&2u64, &1u64].into_iter());
    assert_eq!(
        encoder.finish(),
        std::collections::BTreeSet::from([1u64, 2u64]).canonical_bytes(),
        "the set writer and the BTreeSet impl must agree, handed opposite orders"
    );

    let mut encoder = Encoder::new();
    encoder.map([(&"b".to_owned(), &2u64), (&"a".to_owned(), &1u64)].into_iter());
    assert_eq!(
        encoder.finish(),
        std::collections::BTreeMap::from([("a".to_owned(), 1u64), ("b".to_owned(), 2u64)])
            .canonical_bytes(),
        "the map writer and the BTreeMap impl must agree, handed opposite orders"
    );

    let mut encoder = Encoder::new();
    encoder.unsigned(7);
    assert_eq!(encoder.finish(), 7u64.canonical_bytes());

    let mut encoder = Encoder::new();
    encoder.signed(-7);
    assert_eq!(encoder.finish(), (-7i64).canonical_bytes());

    let mut encoder = Encoder::new();
    encoder.option(Some(&1u64));
    assert_eq!(encoder.finish(), Some(1u64).canonical_bytes());

    let mut encoder = Encoder::new();
    encoder.hash(ContentHash::of_bytes(b"knowledge").as_bytes());
    assert_eq!(
        encoder.finish(),
        ContentHash::of_bytes(b"knowledge").canonical_bytes()
    );
}

#[test]
fn a_refusal_carries_the_text_it_refused() {
    let id = "urn:uuid:01a0c3a0-7889-7395-b687-b771f5ae3aa7"
        .parse::<NodeId>()
        .expect_err("a second spelling is refused");
    assert_eq!(id.text(), "urn:uuid:01a0c3a0-7889-7395-b687-b771f5ae3aa7");

    let hash = "ABC".parse::<ContentHash>().expect_err("not a hash");
    assert_eq!(hash.text(), "ABC");

    let number = "+7"
        .parse::<RevisionNumber>()
        .expect_err("a second spelling is refused");
    assert_eq!(number.text(), "+7");
}
