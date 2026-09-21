//! The coverage check added for pass 1's finding 2, driven against its own claim.
//!
//! `public_surface.rs` says of itself: "What the check buys is that a *new* public item cannot
//! arrive untested without turning a case red." Its criterion is that the item's name appears as a
//! word somewhere in `tests/*.rs`, and it is candid that "a name mentioned only in a comment would
//! satisfy it". What the candour does not say is that the tree already contains such an item, so
//! the check's first false negative shipped with it.
//!
//! This case states the criterion the claim needs: a public item's name must appear in a position
//! where it is being *used* — after a `.` or a `::` — and not merely as an English word in a doc
//! comment or an assertion message.
//!
//! The scan deliberately skips this file. Its own prose names the item it is about, and a check
//! that its own wording satisfies is the defect it is reporting.

use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Every `.rs` file at or below a directory, read, with its file name.
///
/// Recursive since the second correction round: this is the sibling of the `rust_sources` the
/// adversary's own finding 5 reported for reading `src/*.rs` only, and one copy of a defect is not
/// the class. `src/` is flat today, so the case asserts exactly what it asserted before.
fn rust_sources(directory: &Path) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let entries = std::fs::read_dir(directory)
        .unwrap_or_else(|e| panic!("reading {}: {e}", directory.display()));
    for entry in entries {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            found.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let name = path
                .file_name()
                .expect("a file name")
                .to_string_lossy()
                .into_owned();
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
            found.push((name, text));
        }
    }
    found
}

/// The name declared by a `pub fn` / `pub const fn` / `pub const` line, if the line is one. The
/// same three forms `public_surface.rs` scans, so the two cases are comparing the same set.
fn declared_public_name(line: &str) -> Option<&str> {
    let rest = line
        .trim_start()
        .strip_prefix("pub const fn ")
        .or_else(|| line.trim_start().strip_prefix("pub fn "))
        .or_else(|| line.trim_start().strip_prefix("pub const "))?;
    let name = rest
        .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .next()?;
    (!name.is_empty()).then_some(name)
}

/// Whether `text` uses `name` as a path segment or a method — `::name` or `.name`, with no
/// identifier character after it.
fn is_used(text: &str, name: &str) -> bool {
    let bytes = text.as_bytes();
    let mut from = 0;
    while let Some(offset) = text[from..].find(name) {
        let at = from + offset;
        from = at + 1;
        let after = at + name.len();
        let follows_identifier = bytes
            .get(after)
            .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_');
        if follows_identifier {
            continue;
        }
        let before = &text[..at];
        if before.ends_with('.') || before.ends_with("::") {
            return true;
        }
    }
    false
}

#[test]
fn every_public_item_is_used_by_a_case_and_not_only_named_in_prose() {
    let sources = rust_sources(&crate_root().join("src"));
    assert!(!sources.is_empty(), "the source scan found nothing");

    let this_file = "adversary2_public_surface.rs";
    let suite: String = rust_sources(&crate_root().join("tests"))
        .into_iter()
        .filter(|(name, _)| name != this_file)
        .map(|(_, text)| text)
        .collect();
    assert!(!suite.is_empty(), "the suite scan found nothing");

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

    let named_but_never_used: Vec<&String> = declared
        .iter()
        .filter(|name| !is_used(&suite, name))
        .collect();

    assert_eq!(
        named_but_never_used,
        Vec::<&String>::new(),
        "public items no case uses; `no_public_item_is_untested` passes for each of them on the \
         strength of an English word in a doc comment or an assertion message"
    );
}
