//! Adversary, wave p1-06 pass 1: the guard AGENTS.md invariant 1 was rewritten to lean on.
//!
//! `AGENTS.md` invariant 1 now reads, in this unit's own wording:
//!
//! > `ekr-kernel` is the only crate that declares `ekr-store` or implements that trait in a
//! > `src/`. Both of those are **read off this tree** by `crates/ekr/tests/story_contract.rs`, by a
//! > case and not by the compiler.
//!
//! So the sentence that used to be carried by a type is now carried by one scan. These two cases
//! read that scan as the specification the invariant says it is: the first runs **the guard's own
//! code** over a tree with a second authority planted in it, and the second holds the guard to
//! locating this tree at run time.

mod authority_scan;

use std::path::{Path, PathBuf};

use tempfile::TempDir;

/// The workspace root, located at **run time**.
///
/// `std::env::var` and not `env!`, which is `AGENTS.md`'s own rule and the subject of the second
/// case below.
fn workspace_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    Path::new(&manifest)
        .parent()
        .and_then(Path::parent)
        .expect("crates/ekr has a workspace root two levels up")
        .to_path_buf()
}

/// A crate directory under `root` holding one `.rs` file with `contents`.
fn plant(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().expect("a planted file has a directory"))
        .expect("the planted directory is creatable");
    std::fs::write(&path, contents).expect("the planted file is writable");
}

/// **The guard sees a path-qualified head in a `src/`, and does not report one in a `tests/`.**
///
/// Written as `the_commit_authority_guard_sees_every_implementation_in_the_tree` and re-aimed,
/// because the first version could not answer the question its name asks. It re-derived the guard's
/// *needle* — the literal `impl CommitAuthority for` — and asserted that no line in the workspace
/// lacked it, so it was green exactly when one spelling disappeared and it never ran the guard at
/// all. Changing that spelling would have made it green vacuously; the guard's blindness is closed
/// by reading a head **by shape**, which is [`authority_scan::implements_commit_authority`], and
/// this case is what holds the guard to it.
///
/// One body of code, two callers: `story_contract.rs` runs [`authority_scan::implementations`] over
/// the real tree, and this runs it over a tree with a second authority planted in it — the mutation
/// that has to go red, written down so that it runs every time rather than once by hand.
///
/// Both halves matter, and the second is why the real tree needs no edit:
///
/// * a head in a `src/`, written through the trait's path, is **named**;
/// * a head in a `tests/`, written the same way, is **not** — which is the rule and not an
///   exemption, and is why `ekr-store`'s `Attesting` is not a finding.
#[test]
fn the_commit_authority_guard_sees_a_second_authority_in_a_source_file() {
    let name = authority_scan::trait_name();

    // The matcher's own answer, first: the spelling the old needle could not see.
    let qualified = format!("impl ekr_store::{name} for Planted {{");
    assert!(
        authority_scan::implements_commit_authority(&qualified),
        "a head written through the trait's path is a head: {qualified}"
    );

    // And the guard's, over a tree that holds one.
    let planted = TempDir::new().expect("a temporary directory");
    let root = planted.path();
    let implementation = |who: &str| {
        format!("use ekr_store::RecordedValidation;\n\nimpl ekr_store::{name} for {who} {{\n    fn attests(&self, _v: &RecordedValidation) -> bool {{\n        true\n    }}\n}}\n")
    };
    plant(
        root,
        "crates/planted/src/authority.rs",
        &implementation("Shipped"),
    );
    plant(
        root,
        "crates/planted/tests/authority.rs",
        &implementation("Stub"),
    );
    plant(
        root,
        "crates/ekr-kernel/src/commit.rs",
        &implementation("Validations"),
    );
    // A near miss, so that "found three" is not satisfied by a scan matching anything.
    plant(
        root,
        "crates/planted/src/other.rs",
        &format!("impl My{name} for Shipped {{}}\n"),
    );

    let (found, visited) = authority_scan::implementations(root);
    assert_eq!(visited, 4, "the walk read every file this case planted");
    assert_eq!(
        found,
        vec![
            "crates/ekr-kernel/src/commit.rs:3".to_owned(),
            "crates/planted/src/authority.rs:3".to_owned(),
            "crates/planted/tests/authority.rs:3".to_owned(),
        ],
        "the scan finds the three heads and not the trait whose name merely ends with this one"
    );

    assert_eq!(
        authority_scan::second_authorities(&found),
        vec!["crates/planted/src/authority.rs:3".to_owned()],
        "AGENTS.md invariant 1 and ADR 0007 say ekr-kernel is the only crate that implements \
         CommitAuthority in a src/. A second one written through the trait's path is what the \
         guard's old literal needle could not see, and is what it must name; the kernel's own and \
         a suite's stub are not second authorities, which is why an implementation in a test file \
         is not a finding and does not have to move"
    );
}

/// The guard locates the repository with the compile-time macro `AGENTS.md` forbids, one commit
/// after `AGENTS.md` forbade it.
///
/// `AGENTS.md` § The gate, added in `c4e436e` — the commit this branch is forked from:
///
/// > **A test that reads this repository's own source must not locate it with
/// > `env!("CARGO_MANIFEST_DIR")`.** … While both trees exist the guard passes while reading a
/// > different checkout's source … so one reading the wrong tree has stopped checking anything and
/// > says nothing.
///
/// `crates/ekr/tests/story_contract.rs:44-50` does exactly that, and this unit added
/// `only_the_kernel_implements_the_commit_authority` on top of it and then rewrote invariant 1 to
/// say the two facts are "read off **this** tree" by that file. With the shared
/// `CARGO_TARGET_DIR` the same document mandates, "this tree" is whichever checkout compiled the
/// binary — and the guard's own floor is `CRATES.len() + 1`, seven files, which any checkout of
/// this repository clears.
#[test]
fn the_file_that_carries_invariant_one_locates_this_tree_at_run_time() {
    let root = workspace_root();
    let guard = root.join("crates/ekr/tests/story_contract.rs");
    let text = std::fs::read_to_string(&guard).expect("story_contract.rs is readable");

    // Assembled so that this case does not report itself if it is ever moved beside the guard.
    let forbidden = format!("env{}(\"CARGO_MANIFEST_DIR\")", "!");
    let at = text
        .lines()
        .enumerate()
        .filter(|(_, line)| line.contains(&forbidden))
        .map(|(number, line)| format!("{}: {}", number + 1, line.trim()))
        .collect::<Vec<_>>();

    assert!(
        at.is_empty(),
        "AGENTS.md forbids a test locating this repository with the compile-time macro, and \
         AGENTS.md invariant 1 now says this file reads its two facts off *this* tree. It locates \
         the tree at compile time, so a binary built in one checkout keeps scanning that checkout \
         while another runs it — and a scan of the wrong tree reports clean: {at:?}"
    );
}
