//! Adversary pass on wave correct-07 unit G: `task:divergence-is-a-typed-store-error`.
//!
//! Acceptance: "No CLI code matches a provider message string for divergence (a source guard)."
//! The unit's guard (`no_cli_source_matches_a_provider_message_for_divergence`,
//! `crates/ekr/src/cli/session/tests.rs`) looks for one needle, the provider's sentence from
//! "diverged from this handle" on. A CLI matching a shorter part of that sentence — `"diverged"`,
//! as eventlog-file's own tests at `fe8a0a7` do (`message.contains("diverged")`) — matches a
//! provider message string for divergence and passes that guard.

use std::path::{Path, PathBuf};

/// The rule of the unit's guard after correction 1
/// (`no_cli_source_holds_a_string_literal_naming_divergence`), built as it builds it: a string
/// literal that names divergence, in any case.
fn unit_rule(literal: &str) -> bool {
    literal.to_ascii_lowercase().contains("diverge")
}

fn sources(directory: &Path, found: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(directory).expect("a source directory") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

/// The string literals of `source` outside line comments, roughly: enough for a guard.
fn literals(source: &str) -> Vec<String> {
    let mut found = Vec::new();
    for line in source.lines() {
        let code = match line.find("//") {
            Some(at) if !line[..at].contains('"') => &line[..at],
            _ => line,
        };
        let mut parts = code.split('"');
        parts.next();
        while let Some(inside) = parts.next() {
            found.push(inside.to_owned());
            parts.next();
        }
    }
    found
}

/// The CLI non-test sources under `root` holding a string literal that names divergence.
fn naming_divergence(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    sources(root, &mut found);
    found
        .into_iter()
        .filter(|path| path.file_name().is_some_and(|name| name != "tests.rs"))
        .filter(|path| {
            literals(&std::fs::read_to_string(path).expect("a UTF-8 source"))
                .iter()
                .any(|literal| literal.to_ascii_lowercase().contains("diverge"))
        })
        .collect()
}

/// The regression the acceptance's guard exists for, written as eventlog-file's own tests match
/// the condition: a CLI reader telling divergence by part of the provider's message.
const REGRESSION: &str = "pub(super) fn diverged(message: &str) -> bool {\n    \
                          message.contains(\"diverged\")\n}\n";

/// The unit's guard must fail on a CLI that matches the provider's divergence message by a part
/// of it. Its first needle was the sentence's tail, so `contains("diverged")` passed; its rule is
/// now the literal rule below.
#[test]
fn adversary_x7_g_the_unit_guard_catches_a_partial_provider_message_match() {
    assert!(
        literals(REGRESSION)
            .iter()
            .any(|literal| unit_rule(literal)),
        "the unit's guard does not find this match of the provider's divergence message in a \
         CLI source:\n{REGRESSION}"
    );
}

/// GUARD (green on the unit's tree). No CLI source outside its test modules holds a string
/// literal naming divergence. Set `ADVERSARY_X7_G_SRC` to another tree's `src` to run it there.
#[test]
fn adversary_x7_g_no_cli_source_holds_a_divergence_literal() {
    let root = std::env::var_os("ADVERSARY_X7_G_SRC").map_or_else(
        || {
            let manifest =
                std::env::var_os("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
            Path::new(&manifest).join("src")
        },
        PathBuf::from,
    );
    let mut all = Vec::new();
    sources(&root, &mut all);
    assert!(all.len() > 10, "the CLI's sources are found under {root:?}");
    let found = naming_divergence(&root);
    assert!(
        found.is_empty(),
        "these CLI sources hold a string literal naming divergence: {found:?}"
    );
}

/// GUARD (green). The stronger guard above does find the regression the unit's guard passes.
#[test]
fn adversary_x7_g_the_literal_guard_finds_the_partial_match() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    std::fs::write(directory.path().join("reader.rs"), REGRESSION).unwrap();
    assert_eq!(
        naming_divergence(directory.path()),
        vec![directory.path().join("reader.rs")]
    );
}
