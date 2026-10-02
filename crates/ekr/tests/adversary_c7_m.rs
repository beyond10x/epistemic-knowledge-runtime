//! Adversarial pass on `task:migrate-reads-a-current-store` (wave correct-07, unit M), CLI side:
//! the snapshot comparison `migrate_cli.rs` makes after `ekr migrate`.
//!
//! `migrate_writes_a_store_at_the_new_path_with_the_same_history_on_both_providers` compares
//! `after["graph"][field]` with `before["graph"][field]` for `nodes`, `edges`, `assertions` and
//! `evidence` (`migrate_cli.rs:127-131`). `ekr snapshot` prints those fields under
//! `["graph"]["graph"]`, so each comparison is `null == null` and would pass for a destination that
//! held none of the source's graph. The comparison now reads `["graph"]["graph"]`.

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

fn store_path(directory: &Path, name: &str, backend: &str) -> PathBuf {
    match backend {
        "file" => directory.join(name),
        _ => directory.join(format!("{name}.db")),
    }
}

/// A store seeded with the seed example, and the transaction example committed if `commit`.
fn store(directory: &Path, name: &str, backend: &str, commit: bool) -> (PathBuf, PathBuf) {
    let host = directory.join("host.json");
    std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
    let seed = directory.join("seed.yaml");
    std::fs::write(&seed, text(&["example", "ekr-seed/2"])).unwrap();
    let first = directory.join("first.yaml");
    std::fs::write(&first, text(&["example", "ekr.transaction-document/2"])).unwrap();
    let store = store_path(directory, name, backend);
    let path = |file: &PathBuf| file.to_str().unwrap().to_owned();
    document(&run(&host, &store, backend, &["seed", &path(&seed)]));
    if commit {
        let id = document(&run(&host, &store, backend, &["propose", &path(&first)]))
            ["transaction_id"]
            .as_str()
            .unwrap()
            .to_owned();
        document(&run(&host, &store, backend, &["validate", &id]));
        document(&run(&host, &store, backend, &["commit", &id]));
    }
    (host, store)
}

const FIELDS: [&str; 4] = ["nodes", "edges", "assertions", "evidence"];

/// The comparison shape of `migrate_cli.rs:127-131`, held against two stores whose graphs differ
/// (a seed only, and the same seed with a committed transaction): it must tell them apart, or it
/// cannot catch a migration that drops graph content.
#[test]
fn the_migrate_cli_snapshot_comparison_tells_a_seed_only_store_from_a_committed_one() {
    for backend in ["file", "sqlite"] {
        let directory = tempfile::tempdir().unwrap();
        let (host, seeded) = store(directory.path(), "seeded", backend, false);
        let (_, committed) = store(directory.path(), "committed", backend, true);
        let seeded = document(&run(&host, &seeded, backend, &["snapshot"]));
        let committed = document(&run(&host, &committed, backend, &["snapshot"]));
        assert_ne!(
            seeded["graph"]["graph"]["assertions"], committed["graph"]["graph"]["assertions"],
            "{backend}: the two stores' graphs differ"
        );
        // Each field the comparison reads holds content on both sides, and the comparison as a
        // whole (every field equal) fails between the two stores.
        let blind: Vec<&str> = FIELDS
            .into_iter()
            .filter(|field| {
                seeded["graph"]["graph"][field].is_null()
                    || committed["graph"]["graph"][field].is_null()
            })
            .collect();
        assert!(
            blind.is_empty(),
            "{backend}: the comparison at migrate_cli.rs:127-131 reads null for {blind:?}"
        );
        assert!(
            FIELDS
                .into_iter()
                .any(|field| seeded["graph"]["graph"][field] != committed["graph"]["graph"][field]),
            "{backend}: the comparison at migrate_cli.rs:127-131 passes between a seed-only store \
             and a committed one"
        );
    }
}

/// The comparison that test meant: every graph field of the migrated store's snapshot, at the
/// path `ekr snapshot` prints it, equals the source's, and is not empty where the source's is not.
#[test]
fn a_migrated_store_holds_every_graph_field_of_its_source() {
    for backend in ["file", "sqlite"] {
        let directory = tempfile::tempdir().unwrap();
        let (host, source) = store(directory.path(), "source", backend, true);
        let destination = store_path(directory.path(), "migrated", backend);
        let before = document(&run(&host, &source, backend, &["snapshot"]));
        document(&run(
            &host,
            &source,
            backend,
            &["migrate", "--to", destination.to_str().unwrap()],
        ));
        let after = document(&run(&host, &destination, backend, &["snapshot"]));
        for field in FIELDS {
            let source_field = &before["graph"]["graph"][field];
            assert!(
                source_field.is_object(),
                "{backend} {field}: {source_field}"
            );
            assert_eq!(
                after["graph"]["graph"][field], *source_field,
                "{backend} {field}"
            );
        }
        for field in ["nodes", "assertions", "evidence"] {
            assert!(
                !before["graph"]["graph"][field]
                    .as_object()
                    .unwrap()
                    .is_empty(),
                "{backend} {field}: the source holds content to lose"
            );
        }
    }
}
