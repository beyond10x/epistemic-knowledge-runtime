//! Adversary, wave 2026-10-07c, unit C (`task:postgres-source-copy`, commit 3c96c31e52): the
//! documents a reader and the specification read about the PostgreSQL inventory, held to the
//! source that now answers it. The unit removed `postgres-inventory-requires-capture` from the
//! runtime (`EventlogStore::inventory` takes one provider capture instead).
use std::path::{Path, PathBuf};

/// The workspace root: the nearest directory above the invoking crate that holds `Cargo.lock`,
/// read at run time so a binary built in another checkout reads this one.
fn workspace_root() -> PathBuf {
    let mut at = std::env::current_dir().unwrap();
    while !at.join("Cargo.lock").is_file() {
        assert!(at.pop(), "no Cargo.lock above the test's working directory");
    }
    at
}

fn read(relative: &str) -> String {
    std::fs::read_to_string(workspace_root().join(relative)).unwrap()
}

/// Every `.rs` file under the runtime's source directories, concatenated.
fn sources() -> String {
    fn walk(at: &Path, into: &mut String) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, into);
            } else if path.extension().is_some_and(|e| e == "rs") {
                into.push_str(&std::fs::read_to_string(&path).unwrap());
            }
        }
    }
    let mut all = String::new();
    for directory in [
        "crates/ekr/src",
        "crates/ekr-kernel/src",
        "crates/ekr-store/src",
    ] {
        walk(&workspace_root().join(directory), &mut all);
    }
    all
}

/// Whether `source` holds `name` as a whole string literal, or as the literal a refusal with a
/// detail starts with — the matcher `crates/ekr/tests/docs_cli.rs` uses.
fn emits(source: &str, name: &str) -> bool {
    source.contains(&format!("\"{name}\"")) || source.contains(&format!("\"{name}: "))
}

const REMOVED: &str = "postgres-inventory-requires-capture";

/// `docs/cli.md`, the user reference, tells a reader the low-level PostgreSQL inventory refuses
/// `postgres-inventory-requires-capture`. No runtime source emits it after this unit, and
/// `docs_cli.rs` checks only the page's refusal table, so nothing else holds the sentence.
#[test]
fn docs_cli_names_no_inventory_refusal_the_runtime_no_longer_emits() {
    let page = read("docs/cli.md");
    let source = sources();
    assert!(
        !emits(&source, REMOVED),
        "the runtime emits {REMOVED} again"
    );
    let stale: Vec<&str> = page.lines().filter(|line| line.contains(REMOVED)).collect();
    assert!(
        stale.is_empty(),
        "docs/cli.md names a refusal no runtime source emits: {stale:?}"
    );
}

/// `systems/ekr/domains/cli.yaml` says of `ekr.cli.BeginStage` that the PostgreSQL capture is one
/// "which the inventory refuses today" and that unit C adds; after this unit the inventory takes
/// it.
#[test]
fn the_begin_command_does_not_say_the_inventory_refuses_a_capture_today() {
    let specification = read("systems/ekr/domains/cli.yaml");
    let stale: Vec<&str> = specification
        .lines()
        .filter(|line| line.contains("which the inventory refuses today") || line.contains(REMOVED))
        .collect();
    assert!(
        stale.is_empty(),
        "systems/ekr/domains/cli.yaml describes the removed inventory refusal as current: {stale:?}"
    );
}
