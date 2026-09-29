//! Adversary pass, wave ingest-02 unit V: `ekr migrate --to` against `docs/cli.md` ("Nothing is
//! ever written to `--store`"), and `ekr seed` / `ekr session --create` writing
//! `ekr-seed-envelope/3`, over stores the binary writes itself.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

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

fn ok(output: &Output) {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn bytes_under(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, into: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(root, &path, into);
            } else {
                into.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut into = BTreeMap::new();
    visit(root, root, &mut into);
    into
}

/// The host, seed and transaction example files under `directory`.
fn inputs(directory: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let host = directory.join("host.json");
    std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
    let seed = directory.join("seed.yaml");
    std::fs::write(&seed, text(&["example", "ekr-seed/2"])).unwrap();
    let transaction = directory.join("transaction.yaml");
    std::fs::write(
        &transaction,
        text(&["example", "ekr.transaction-document/2"]),
    )
    .unwrap();
    (host, seed, transaction)
}

/// A seeded file store written through the binary.
fn file_store(directory: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let (host, seed, transaction) = inputs(directory);
    let store = directory.join("source");
    ok(&run(
        &host,
        &store,
        "file",
        &["seed", seed.to_str().unwrap()],
    ));
    (host, store, transaction)
}

#[test]
fn migrate_to_a_path_inside_the_file_store_leaves_the_store_exactly_as_it_was() {
    let directory = tempfile::tempdir().unwrap();
    let (host, store, _) = file_store(directory.path());
    let before = bytes_under(&store);
    let to = store.join("v3");
    let migrated = run(
        &host,
        &store,
        "file",
        &["migrate", "--to", to.to_str().unwrap()],
    );
    let after = bytes_under(&store);
    let added: Vec<_> = after
        .keys()
        .filter(|path| !before.contains_key(*path))
        .collect();
    assert!(
        migrated.status.code() != Some(0) || after == before,
        "`ekr migrate --to {}` exited 0 and wrote {} new files inside --store: {added:?}",
        to.display(),
        added.len()
    );
}

#[test]
fn migrate_into_the_file_store_blob_directory_leaves_the_store_writable() {
    let directory = tempfile::tempdir().unwrap();
    let (host, store, transaction) = file_store(directory.path());
    assert!(
        store.join("blobs").is_dir(),
        "the file provider keeps blobs/"
    );
    let to = store.join("blobs").join("v3");
    let migrated = run(
        &host,
        &store,
        "file",
        &["migrate", "--to", to.to_str().unwrap()],
    );
    let proposed = run(
        &host,
        &store,
        "file",
        &["propose", transaction.to_str().unwrap()],
    );
    assert_eq!(
        proposed.status.code(),
        Some(0),
        "after `ekr migrate --to {}` (exit {:?}), the source store refuses a write: {}",
        to.display(),
        migrated.status.code(),
        String::from_utf8_lossy(&proposed.stderr)
    );
}

#[test]
fn migrate_to_a_symlink_of_the_store_is_refused_as_the_source_on_both_providers() {
    for backend in ["file", "sqlite"] {
        let directory = tempfile::tempdir().unwrap();
        let (host, seed, _) = inputs(directory.path());
        let store = match backend {
            "file" => directory.path().join("source"),
            _ => directory.path().join("source.db"),
        };
        ok(&run(
            &host,
            &store,
            backend,
            &["seed", seed.to_str().unwrap()],
        ));
        let link = directory.path().join("link");
        std::os::unix::fs::symlink(&store, &link).unwrap();
        let before = bytes_under(directory.path());
        let refused = run(
            &host,
            &store,
            backend,
            &["migrate", "--to", link.to_str().unwrap()],
        );
        assert_eq!(refused.status.code(), Some(1), "{backend}");
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert!(
            stderr.contains("migrate-destination-is-source"),
            "{backend}: {stderr}"
        );
        assert_eq!(bytes_under(directory.path()), before, "{backend}");
    }
}

/// Every retained byte of a store: the files of a file store, the database and its side files for
/// SQLite.
fn store_bytes(store: &Path, backend: &str) -> Vec<u8> {
    if backend == "file" {
        return bytes_under(store).into_values().flatten().collect();
    }
    let mut bytes = Vec::new();
    for suffix in ["", "-wal", "-journal"] {
        let path = PathBuf::from(format!("{}{suffix}", store.display()));
        if let Ok(read) = std::fs::read(path) {
            bytes.extend(read);
        }
    }
    bytes
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

#[test]
fn seed_and_session_create_both_retain_seed_envelope_three_on_both_providers() {
    for backend in ["file", "sqlite"] {
        let directory = tempfile::tempdir().unwrap();
        let (host, seed, _) = inputs(directory.path());
        let name = |stem: &str| match backend {
            "file" => directory.path().join(stem),
            _ => directory.path().join(format!("{stem}.db")),
        };

        let seeded = name("seeded");
        ok(&run(
            &host,
            &seeded,
            backend,
            &["seed", seed.to_str().unwrap()],
        ));

        let session = name("session");
        let mut child = ekr()
            .arg("--host")
            .arg(&host)
            .arg("--store")
            .arg(&session)
            .args(["--backend", backend, "session", "--create"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let request = serde_json::json!({"argv": ["seed", seed.to_str().unwrap()]});
        writeln!(child.stdin.take().unwrap(), "{request}").unwrap();
        let output = child.wait_with_output().unwrap();
        ok(&output);
        let answer: serde_json::Value =
            serde_json::from_slice(output.stdout.split(|b| *b == b'\n').next().unwrap()).unwrap();
        assert_eq!(answer["exit"], 0, "{backend}: {answer}");

        for store in [&seeded, &session] {
            let bytes = store_bytes(store, backend);
            assert!(
                contains(&bytes, b"ekr-seed-envelope/3"),
                "{backend} {}: no /3 envelope retained",
                store.display()
            );
            assert!(
                !contains(&bytes, b"ekr-seed-envelope/2"),
                "{backend} {}: a /2 envelope retained",
                store.display()
            );
        }
    }
}
