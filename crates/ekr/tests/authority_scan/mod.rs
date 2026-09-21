//! The scan the second half of `AGENTS.md` invariant 1 rests on.
//!
//! `architecture-decision-record:0007-the-commit-path-is-the-kernels`. `ekr-store`'s fold moves
//! canonical state only for a validation its injected `CommitAuthority` stands behind, and that
//! trait is declared *below* `ekr-kernel`, so Rust cannot say "only that other crate implements
//! it". What holds instead is that the only implementation in any crate's `src/` is the kernel's —
//! and this module is what reads it off the tree.
//!
//! # Why it is a module and not a function in one file
//!
//! `story_contract.rs` is the guard: it runs this over the real tree and refuses a second
//! implementation. `adversary_p1_06_authority_guard.rs` is the case that reads the guard as the
//! specification the invariant says it is: it runs this over a tree it plants, where a second
//! implementation *does* exist, and holds it to naming that one and not the test file beside it.
//! Two binaries over **one** body of code, so the case cannot pass for a scan the guard does not
//! run — which is exactly what happened when the case re-derived the guard's needle instead.
//!
//! Declared by both, and every item here is used by both: an integration test binary compiles each
//! shared module it declares, and `-D warnings` makes a helper with no caller in one of them a
//! build failure.

use std::path::{Path, PathBuf};

/// The directories a scan of a checkout never descends into.
const SKIP: [&str; 2] = [".git", "target"];

/// The one crate whose `src/` may implement the trait.
const KERNEL_SOURCE: &str = "crates/ekr-kernel/src/";

/// Whether `line` is the head of an implementation of `ekr-store`'s `CommitAuthority`.
///
/// **Structural, and deliberately not a spelling.** The needle was the literal
/// `impl CommitAuthority for`, and the same implementation written through the trait's path —
/// `impl ekr_store::CommitAuthority for Attesting`, which `ekr-store`'s own suite already writes —
/// does not contain it. A rule enumerated by its instances has a next instance, and for a guard
/// that carries an invariant the next instance is the one that reports clean. Three things make
/// this a head rather than a mention:
///
/// * the line begins `impl`, so a doc comment, a `use` and a `trait` declaration are not heads;
/// * it names `CommitAuthority for`, at an identifier boundary, so `MyCommitAuthority` is not this
///   trait and `ekr_store::CommitAuthority` is;
/// * whatever generics or path qualifier sit either side of the name are not read at all.
///
/// The trait name is assembled rather than written out: the rule is lexical and this file is inside
/// the tree it walks, so a line spelling it out would be a head of its own.
pub fn implements_commit_authority(line: &str) -> bool {
    if !line.trim_start().starts_with("impl") {
        return false;
    }
    let trailing = format!("{} for ", trait_name());
    line.match_indices(&trailing).any(|(at, _)| {
        // What precedes the name is what separates this trait from one whose name ends with it:
        // a path separator or whitespace, never an identifier character.
        line[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_')
    })
}

/// The trait's name, assembled for the reason [`implements_commit_authority`] gives.
pub fn trait_name() -> String {
    ["Commit", "Authority"].concat()
}

/// Every implementation head under `root`, as `<path relative to root>:<line>`, and how many `.rs`
/// files were read to find them.
///
/// The count is returned rather than discarded because a walk that reaches nothing reports clean,
/// and "clean" is the answer this scan gives when it is broken. Both callers assert against it: the
/// guard against the workspace's own floor, the case against the tree it planted.
pub fn implementations(root: &Path) -> (Vec<String>, usize) {
    let mut found = Vec::new();
    let mut visited = 0usize;
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let entries = std::fs::read_dir(&directory)
            .unwrap_or_else(|e| panic!("reading {}: {e}", directory.display()));
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if !SKIP.iter().any(|skip| entry.file_name() == *skip) {
                    stack.push(path);
                }
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "rs") {
                continue;
            }
            visited += 1;
            let text = std::fs::read_to_string(&path).expect("a source file is readable");
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .display()
                .to_string();
            for (number, line) in text.lines().enumerate() {
                if implements_commit_authority(line) {
                    found.push(format!("{relative}:{}", number + 1));
                }
            }
        }
    }
    found.sort();
    (found, visited)
}

/// The heads of `found` that are a second thing the fold will commit for.
///
/// **An implementation in a `tests/` directory is not one**, and that is the rule rather than an
/// exemption: a suite that could not write one could not test the fold rules that turn on it, and a
/// test's authority is reachable from no shipped path. `ekr-store`'s own suite has two and
/// `ekr-kernel`'s reads a third. What this names is an implementation in a crate's `src/` outside
/// `ekr-kernel`'s — code the runtime ships, holding a second answer to the one question the fold
/// asks before it moves canonical state.
pub fn second_authorities(found: &[String]) -> Vec<String> {
    found
        .iter()
        .filter(|at| !at.contains("/tests/") && !at.starts_with(KERNEL_SOURCE))
        .cloned()
        .collect()
}
