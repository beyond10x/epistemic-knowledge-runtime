//! Every public item of *every* crate is exercised by some case, and the list is not hand-kept.
//!
//! `crates/ekr-core/tests/public_surface.rs` does this for one crate, and cost two adversary
//! findings to get right: the first version counted a name appearing anywhere in the suite text,
//! and `Encoder::list` passed it on the English word "list" in a doc comment. The other four
//! crates had no such guard and no `tests/` directory at all, so from the ontology onward every
//! new public item would have arrived with nothing to notice it was untested. Copying the
//! per-crate guard four times would have re-paid those two findings four times, so the check is
//! lifted here instead, where `ekr` already depends on all five.
//!
//! The criterion is the one the per-crate guard settled on:
//!
//! * **prose does not count.** Comments are stripped before the suite is searched, so a name in a
//!   doc comment — including one written as `Type::name` — satisfies nothing.
//! * **the occurrence must be a use.** `::name` or `.name`, in a path or method position, not a
//!   bare word. A literal `(` is not required, because a `pub const` is used without one.
//!
//! It is not a coverage measurement: a case can name an item and assert nothing about it. What it
//! buys is that a *new* public item cannot arrive untested without turning this case red.
//!
//! # What it does not yet catch
//!
//! Only `pub fn`, `pub const fn` and `pub const` are read as declarations. A `pub struct`,
//! `pub enum`, `pub trait` or `pub type` is invisible to it, because a type is used as `Type::…`,
//! `&Type`, `: Type` or `Type {` — none of which the `::name`/`.name` criterion matches, so
//! widening the declaration side without widening the use side would report every type in the
//! workspace as untested. `task:public-surface-covers-types` carries that.

use std::path::{Path, PathBuf};

/// The workspace root: this crate's manifest directory is `<root>/crates/ekr`.
fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/ekr sits two levels below the workspace root")
        .to_path_buf()
}

/// Every `.rs` file at or below a directory, read. Recurses, so a later module directory does not
/// become a hole in the scan.
fn rust_sources(directory: &Path) -> Vec<(PathBuf, String)> {
    let mut found = Vec::new();
    if !directory.is_dir() {
        return found;
    }
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
/// A string literal containing `//` is cut too. That costs a false *positive* — an item named only
/// inside such a literal is reported untested — and never a false negative, which is the direction
/// that matters for a guard.
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

/// Every crate directory under `crates/`, sorted, so the report names them in a stable order.
fn crate_directories() -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = std::fs::read_dir(workspace_root().join("crates"))
        .expect("reading crates/")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| path.join("Cargo.toml").is_file())
        .collect();
    found.sort();
    found
}

#[test]
fn no_public_item_in_any_crate_is_untested() {
    let crates = crate_directories();
    assert!(
        crates.len() >= 6,
        "the crate scan is broken, not the workspace: {crates:?}"
    );

    // The suite is every crate's tests pooled: `ekr` depends on all five, so a case anywhere may
    // legitimately exercise an item declared anywhere.
    let read: Vec<(PathBuf, String)> = crates
        .iter()
        .flat_map(|directory| rust_sources(&directory.join("tests")))
        .collect();
    let raw: String = read.iter().map(|(_, text)| text.as_str()).collect();
    let suite: String = read
        .iter()
        .map(|(_, text)| without_comments(text))
        .collect();

    // A check that silently stops stripping is a check that is back to counting prose, so the
    // stripper is measured rather than trusted. The marker is assembled from two pieces here and
    // written whole in the comment two lines below, so this line is not itself a match.
    let marker = concat!("workspace-stripper", "-sentinel");
    // workspace-stripper-sentinel — the one occurrence, and it is inside a comment.
    assert!(raw.contains(marker), "this file is not in the scan");
    assert!(
        !suite.contains(marker),
        "the comment stripper did nothing: a marker written only in a comment survived it"
    );

    let mut untested: Vec<String> = Vec::new();
    let mut declared_total = 0usize;
    for directory in &crates {
        let name = directory
            .file_name()
            .expect("a crate directory has a name")
            .to_string_lossy()
            .into_owned();
        let mut declared: Vec<String> = rust_sources(&directory.join("src"))
            .iter()
            .flat_map(|(_, text)| text.lines())
            .filter_map(declared_public_name)
            .map(str::to_owned)
            .collect();
        declared.sort();
        declared.dedup();
        declared_total += declared.len();
        untested.extend(
            declared
                .into_iter()
                .filter(|item| !is_used(&suite, item))
                .map(|item| format!("{name}::{item}")),
        );
    }

    assert!(
        declared_total > 20,
        "the declaration scan is broken, not the suite: {declared_total} items across {} crates",
        crates.len()
    );
    assert!(
        untested.is_empty(),
        "public items no case uses outside a comment: {untested:?}"
    );
}
