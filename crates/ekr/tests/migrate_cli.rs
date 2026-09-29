//! `ekr migrate --to <path>`: the verb over the kernel's preserving migration (design § 100.3), on
//! both providers.
//!
//! The source is a store this binary writes itself — the seed and transaction examples, one
//! transaction committed and one left proposed — so it is already `ekr-seed-envelope/3` and its
//! history re-publishes record for record. The conversion of an `ekr-seed-envelope/2` store is
//! held by `crates/ekr-kernel/tests/migrate_store.rs`, which can write one: this crate declares no
//! `ekr-store` and so cannot (AGENTS.md invariant 1).

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn run(host: &Path, store: &Path, backend: &str, args: &[&str]) -> Output {
    ekr()
        .arg("--host")
        .arg(host)
        .arg("--store")
        .arg(store)
        .args(["--backend", backend])
        .args(args)
        .output()
        .unwrap()
}

fn document(output: &Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

/// The printed transaction example under another transaction identity.
fn second_transaction(example: &str) -> String {
    let id = example
        .lines()
        .find_map(|line| line.trim().strip_prefix("id: "))
        .unwrap()
        .to_owned();
    example.replacen(&id, "00000000-0000-4000-8000-0000000006ff", 1)
}

/// A store written through the binary: the seed example, the transaction example committed, and
/// a second proposal of it left `Proposed`.
fn source(directory: &Path, backend: &str) -> (PathBuf, PathBuf) {
    let host = directory.join("host.json");
    std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
    let seed = directory.join("seed.yaml");
    std::fs::write(&seed, text(&["example", "ekr-seed/2"])).unwrap();
    let example = text(&["example", "ekr.transaction-document/2"]);
    let first = directory.join("first.yaml");
    std::fs::write(&first, &example).unwrap();
    let second = directory.join("second.yaml");
    std::fs::write(&second, second_transaction(&example)).unwrap();
    let store = match backend {
        "file" => directory.join("source"),
        _ => directory.join("source.db"),
    };
    let path = |file: &PathBuf| file.to_str().unwrap().to_owned();
    document(&run(&host, &store, backend, &["seed", &path(&seed)]));
    let id = document(&run(&host, &store, backend, &["propose", &path(&first)]))["transaction_id"]
        .as_str()
        .unwrap()
        .to_owned();
    document(&run(&host, &store, backend, &["validate", &id]));
    document(&run(&host, &store, backend, &["commit", &id]));
    document(&run(&host, &store, backend, &["propose", &path(&second)]));
    (host, store)
}

#[test]
fn migrate_writes_a_store_at_the_new_path_with_the_same_history_on_both_providers() {
    for backend in ["file", "sqlite"] {
        let directory = tempfile::tempdir().unwrap();
        let (host, source) = source(directory.path(), backend);
        let destination = match backend {
            "file" => directory.path().join("migrated"),
            _ => directory.path().join("migrated.db"),
        };
        let before = document(&run(&host, &source, backend, &["snapshot"]));

        let report = document(&run(
            &host,
            &source,
            backend,
            &["migrate", "--to", destination.to_str().unwrap()],
        ));
        assert_eq!(report["format"], "ekr.store-migration/1", "{backend}");
        // A current store re-publishes record for record: nothing is replaced.
        assert_eq!(
            report["destination_seed_hash"], report["source_seed_hash"],
            "{backend}"
        );
        let occurrences = report["occurrences"].as_array().unwrap();
        assert_eq!(
            occurrences.len(),
            5,
            "{backend}: seed, 2 proposals, validation, commit"
        );
        for occurrence in occurrences {
            assert_eq!(
                occurrence["source_record_hash"], occurrence["destination_record_hash"],
                "{backend}: {occurrence}"
            );
        }
        assert_eq!(report["legacy_objects"], serde_json::json!([]), "{backend}");

        let after = document(&run(&host, &destination, backend, &["snapshot"]));
        for field in ["nodes", "edges", "assertions", "evidence"] {
            assert_eq!(
                after["graph"][field], before["graph"][field],
                "{backend} {field}"
            );
        }
        assert_eq!(
            document(&run(&host, &destination, backend, &["head"])),
            document(&run(&host, &source, backend, &["head"])),
            "{backend}"
        );
        assert_eq!(
            document(&run(&host, &destination, backend, &["transactions"])),
            document(&run(&host, &source, backend, &["transactions"])),
            "{backend}"
        );

        // A second migration into the same path refuses and writes nothing.
        let again = run(
            &host,
            &source,
            backend,
            &["migrate", "--to", destination.to_str().unwrap()],
        );
        assert_eq!(again.status.code(), Some(1), "{backend}");
        let stderr = String::from_utf8_lossy(&again.stderr);
        assert!(
            stderr.contains("migrate-destination-not-empty"),
            "{backend}: {stderr}"
        );
        // And --to naming --store itself is refused before it is opened twice.
        let itself = run(
            &host,
            &source,
            backend,
            &["migrate", "--to", source.to_str().unwrap()],
        );
        assert_eq!(itself.status.code(), Some(1), "{backend}");
        let stderr = String::from_utf8_lossy(&itself.stderr);
        assert!(
            stderr.contains("migrate-destination-is-source"),
            "{backend}: {stderr}"
        );
    }
}

#[test]
fn ekr_help_lists_migrate() {
    let output = ekr().arg("--help").output().unwrap();
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(
        help.lines()
            .any(|line| line.trim_start().starts_with("migrate ")),
        "{help}"
    );
}
