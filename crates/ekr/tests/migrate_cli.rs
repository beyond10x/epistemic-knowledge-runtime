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

/// One `!AddEvidence` of `payload` under evidence id `id`, its hash and byte list printed by
/// `ekr hash`, as a YAML sequence item indented for `transaction.operations`.
fn add_evidence(directory: &Path, id: &str, payload: &str) -> String {
    let file = directory.join(format!("{id}.payload"));
    std::fs::write(&file, payload).unwrap();
    let hashed: Value = serde_json::from_str(&text(&["hash", file.to_str().unwrap()])).unwrap();
    format!(
        "  - !AddEvidence\n    evidence:\n      id: {id}\n      source: !HumanStatement\n        \
         identity: Runtime operator\n      content_hash: {}\n      extracted_by: \
         00000000-0000-4000-8000-000000000101\n      observed_at: 1577836800000\n      \
         confidence: 10000\n    payload: {}\n",
        hashed["content_hash"].as_str().unwrap(),
        hashed["payload_yaml"].as_str().unwrap(),
    )
}

const ADDED_FIRST: &str = "00000000-0000-4000-8000-0000000004a1";
const ADDED_SECOND: &str = "00000000-0000-4000-8000-0000000004a2";
const FIRST_STATEMENT: &str = "Alice said she leads the organization.";
const SECOND_STATEMENT: &str = "The organization's register names Alice as its chief executive.";

/// A store whose history holds evidence added after its seed: the seed example; a transaction
/// holding only an `!AddEvidence`; then one adding a second entry together with the transaction
/// example's assertion, made to cite both added entries and nothing seeded.
fn added_evidence_source(directory: &Path, backend: &str) -> (PathBuf, PathBuf) {
    let host = directory.join("host.json");
    std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
    let seed = directory.join("seed.yaml");
    std::fs::write(&seed, text(&["example", "ekr-seed/2"])).unwrap();
    let only_evidence = directory.join("only-evidence.yaml");
    std::fs::write(
        &only_evidence,
        format!(
            "format: ekr.transaction-document/2\ntransaction:\n  id: \
             00000000-0000-4000-8000-0000000006a1\n  proposer: \
             00000000-0000-4000-8000-000000000101\n  operations:\n{}  evidence: []\n",
            add_evidence(directory, ADDED_FIRST, FIRST_STATEMENT)
        ),
    )
    .unwrap();
    let example = text(&["example", "ekr.transaction-document/2"]);
    let seeded = "\n    - 00000000-0000-4000-8000-000000000401\n";
    assert_eq!(example.matches(seeded).count(), 1, "{example}");
    let cited = format!("\n    - {ADDED_FIRST}\n    - {ADDED_SECOND}\n");
    let with_assertion = example
        .replacen(
            "00000000-0000-4000-8000-000000000601",
            "00000000-0000-4000-8000-0000000006a2",
            1,
        )
        .replacen(seeded, &cited, 1)
        .replacen(
            "  evidence:\n  - 00000000-0000-4000-8000-000000000401\n",
            &format!("  evidence:\n  - {ADDED_FIRST}\n  - {ADDED_SECOND}\n"),
            1,
        )
        .replacen(
            "  operations:\n",
            &format!(
                "  operations:\n{}",
                add_evidence(directory, ADDED_SECOND, SECOND_STATEMENT)
            ),
            1,
        );
    let with_assertion_path = directory.join("with-assertion.yaml");
    std::fs::write(&with_assertion_path, with_assertion).unwrap();
    let store = match backend {
        "file" => directory.join("source"),
        _ => directory.join("source.db"),
    };
    let path = |file: &PathBuf| file.to_str().unwrap().to_owned();
    document(&run(&host, &store, backend, &["seed", &path(&seed)]));
    for transaction in [&only_evidence, &with_assertion_path] {
        let id = document(&run(
            &host,
            &store,
            backend,
            &["propose", &path(transaction)],
        ))["transaction_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let validated = document(&run(&host, &store, backend, &["validate", &id]));
        assert_eq!(validated["kind"], "Validated", "{backend}: {validated}");
        let committed = document(&run(&host, &store, backend, &["commit", &id]));
        assert_eq!(committed["kind"], "Committed", "{backend}: {committed}");
    }
    (host, store)
}

/// `task:migrate-reads-a-current-store`: a store that took evidence after its seed migrates, on
/// both providers, keeping every added evidence entry and its bytes, and `ekr explain` of the
/// assertion citing that evidence answers the same before and after.
#[test]
fn a_store_holding_evidence_added_after_its_seed_migrates_on_both_providers() {
    for backend in ["file", "sqlite"] {
        let directory = tempfile::tempdir().unwrap();
        let (host, source) = added_evidence_source(directory.path(), backend);
        let destination = match backend {
            "file" => directory.path().join("migrated"),
            _ => directory.path().join("migrated.db"),
        };
        let assertion = "00000000-0000-4000-8000-000000000501";
        let explain = ["explain", assertion, "--documents"];
        let before = document(&run(&host, &source, backend, &explain));
        let snapshot = document(&run(&host, &source, backend, &["snapshot"]));
        for id in [ADDED_FIRST, ADDED_SECOND] {
            assert!(
                snapshot["graph"]["graph"]["evidence"].get(id).is_some(),
                "{backend}: the source holds {id}"
            );
        }

        let report = document(&run(
            &host,
            &source,
            backend,
            &["migrate", "--to", destination.to_str().unwrap()],
        ));
        assert_eq!(report["format"], "ekr.store-migration/1", "{backend}");
        assert_eq!(
            report["occurrences"].as_array().unwrap().len(),
            7,
            "{backend}: seed, then a proposal, validation and commit for each transaction"
        );

        let after = document(&run(&host, &destination, backend, &explain));
        assert_eq!(after, before, "{backend}");
        let texts: Vec<&str> = after["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|link| link["kind"] == "Evidence")
            .map(|link| link["text"].as_str().unwrap())
            .collect();
        assert_eq!(
            texts
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>(),
            [FIRST_STATEMENT, SECOND_STATEMENT].into(),
            "{backend}: every added entry's bytes, read from the migrated store"
        );
        assert_eq!(
            document(&run(&host, &destination, backend, &["snapshot"]))["graph"],
            snapshot["graph"],
            "{backend}"
        );
        assert_eq!(
            document(&run(&host, &destination, backend, &["head"])),
            document(&run(&host, &source, backend, &["head"])),
            "{backend}"
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
