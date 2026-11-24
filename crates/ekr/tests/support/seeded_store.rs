//! A store a case seeds through the real `ekr` binary, seeded once per build and copied into
//! every case that asks for the same one.
//!
//! nextest runs each case in a process of its own, so a cache in memory would be filled once per
//! case. This one lives on disk, under the build's own directory
//! (`target/<profile>/ekr-test-stores/<digest>`). The digest covers the `ekr` binary's size and
//! modification time and every input the seeding reads, so a rebuilt binary or a changed fixture
//! seeds again (AGENTS.md, "Tests stay fast").

#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Where seeded stores are kept: beside the `ekr` binary the cases run.
pub(crate) fn root() -> PathBuf {
    Path::new(env!("CARGO_BIN_EXE_ekr"))
        .parent()
        .expect("the ekr binary sits in the build's profile directory")
        .join("ekr-test-stores")
}

/// The digest a store is kept under: the binary's identity, then each input with its length.
pub(crate) fn digest(inputs: &[&[u8]]) -> String {
    let binary = std::fs::metadata(env!("CARGO_BIN_EXE_ekr")).expect("the ekr binary");
    let modified = binary
        .modified()
        .expect("a modification time")
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_nanos();
    let mut bytes = format!("{} {modified}\n", binary.len()).into_bytes();
    for input in inputs {
        bytes.extend_from_slice(&(input.len() as u64).to_le_bytes());
        bytes.extend_from_slice(input);
    }
    ekr_core::ContentHash::of_bytes(&bytes)
        .to_string()
        .replace(':', "-")
}

/// Fills `into`, an empty directory, with what `seed` writes there for `inputs`: a copy of the
/// store an earlier case kept, or, the first time, `seed`'s own result, which is then kept. Two
/// cases that seed at once both keep it; the first rename wins and the other copy is dropped.
pub(crate) fn seeded(inputs: &[&[u8]], into: &Path, seed: impl FnOnce()) {
    let kept = root().join(digest(inputs));
    if kept.is_dir() {
        copy_tree(&kept, into);
        return;
    }
    seed();
    // One name per process and thread: `cargo test` runs a binary's cases as threads of one
    // process, and two of them seeding the same store at once wrote into one directory.
    let thread = format!("{:?}", std::thread::current().id());
    let building = root().join(format!(
        "{}.building-{}-{}",
        kept.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("store"),
        std::process::id(),
        thread.trim_start_matches("ThreadId(").trim_end_matches(')')
    ));
    let _ = std::fs::remove_dir_all(&building);
    copy_tree(into, &building);
    if std::fs::rename(&building, &kept).is_err() {
        let _ = std::fs::remove_dir_all(&building);
        assert!(
            kept.is_dir(),
            "{} was neither kept nor renamed",
            kept.display()
        );
    }
}

/// Copies the directories and regular files below `from` into `to`.
pub(crate) fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap_or_else(|e| panic!("{}: {e}", to.display()));
    for entry in std::fs::read_dir(from).unwrap_or_else(|e| panic!("{}: {e}", from.display())) {
        let entry = entry.expect("a directory entry");
        let kind = entry.file_type().expect("a file type");
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            assert!(
                kind.is_file(),
                "{} is neither a directory nor a regular file",
                entry.path().display()
            );
            std::fs::copy(entry.path(), &target).unwrap_or_else(|e| {
                panic!("{} to {}: {e}", entry.path().display(), target.display())
            });
        }
    }
}
