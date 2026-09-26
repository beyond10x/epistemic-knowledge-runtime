//! Adversary pass 1 on `story:store-open-semantics` (unit `p5-01-store-open`).
//!
//! Acceptance 1 says a refused seed leaves no store behind, and `docs/cli.md` says of
//! `store-not-found` that "a seed the kernel refuses creates none". `Store::open_to_seed` runs seed
//! admission only when `symlink_metadata(store)` fails, so every path that already *exists* but
//! holds no store — an empty directory, an empty file, a dangling symlink — skips admission and goes
//! straight to the open-or-create constructor. A seed refused by the provider open itself (an
//! invalid tenant, checked in `EventlogStore::assemble` after the provider has created its store) is
//! not covered by `Runtime::admit_seed` at all.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn fixture(name: &str) -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time");
    Path::new(&manifest)
        .join("tests/fixtures/retraction")
        .join(name)
}

fn ekr(host: &Path, backend: &str, store: &Path, verb: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ekr"))
        .arg("--host")
        .arg(host)
        .arg("--store")
        .arg(store)
        .args(["--backend", backend])
        .args(verb)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

/// Every path under `root`, relative to it, with its length for files, directories included.
fn tree(root: &Path) -> BTreeSet<(PathBuf, u64)> {
    fn walk(root: &Path, at: &Path, into: &mut BTreeSet<(PathBuf, u64)>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            let meta = std::fs::symlink_metadata(&path).unwrap();
            into.insert((
                path.strip_prefix(root).unwrap().to_path_buf(),
                if meta.is_file() { meta.len() } else { 0 },
            ));
            if meta.is_dir() {
                walk(root, &path, into);
            }
        }
    }
    let mut all = BTreeSet::new();
    walk(root, root, &mut all);
    all
}

/// A seed the kernel refuses at admission: one payload byte no longer hashes to its key.
fn refused_seed(directory: &Path) -> PathBuf {
    let seed = std::fs::read_to_string(fixture("seed.yaml")).unwrap();
    let refused = seed.replacen("\n  - 65\n", "\n  - 66\n", 1);
    assert_ne!(refused, seed, "the payload edit applies");
    let path = directory.join("refused.yaml");
    std::fs::write(&path, refused).unwrap();
    path
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// File provider: `--store` names a directory that exists and is empty (`mktemp -d`). A refused
/// seed must leave it empty; it must not mint `writer.lock`, `manifest.json` and `events.jsonl`.
#[test]
fn file_a_refused_seed_into_an_existing_empty_directory_leaves_no_store() {
    let directory = tempfile::tempdir().unwrap();
    let refused = refused_seed(directory.path());
    let store = directory.path().join("store");
    std::fs::create_dir(&store).unwrap();
    let before = tree(directory.path());
    let output = ekr(
        &fixture("host.json"),
        "file",
        &store,
        &["seed", &refused.display().to_string()],
    );
    assert_eq!(output.status.code(), Some(2), "stderr {}", stderr(&output));
    assert!(
        stderr(&output).starts_with("ekr: ekr.kernel.InvalidSeed: seed-evidence-payload-mismatch"),
        "stderr {}",
        stderr(&output)
    );
    assert_eq!(
        tree(directory.path()),
        before,
        "a refused seed created a file store inside the existing empty directory"
    );
}

/// SQLite provider: `--store` names a file that exists and is empty (`touch`). A refused seed must
/// leave it empty; it must not create the owner tables, `-wal` or `-shm`.
#[test]
fn sqlite_a_refused_seed_into_an_existing_empty_file_leaves_no_store() {
    let directory = tempfile::tempdir().unwrap();
    let refused = refused_seed(directory.path());
    let store = directory.path().join("store.sqlite");
    std::fs::write(&store, b"").unwrap();
    let before = tree(directory.path());
    let output = ekr(
        &fixture("host.json"),
        "sqlite",
        &store,
        &["seed", &refused.display().to_string()],
    );
    assert_eq!(output.status.code(), Some(2), "stderr {}", stderr(&output));
    assert_eq!(
        tree(directory.path()),
        before,
        "a refused seed provisioned a SQLite store in the existing empty file"
    );
}

/// SQLite provider: `--store` is a symlink whose target does not exist yet. `symlink_metadata`
/// sees the link, so admission is skipped, and SQLite follows the link and creates the target.
#[test]
fn sqlite_a_refused_seed_through_a_dangling_symlink_creates_nothing() {
    let directory = tempfile::tempdir().unwrap();
    let refused = refused_seed(directory.path());
    let target = directory.path().join("target.sqlite");
    let store = directory.path().join("store.sqlite");
    std::os::unix::fs::symlink(&target, &store).unwrap();
    let before = tree(directory.path());
    let output = ekr(
        &fixture("host.json"),
        "sqlite",
        &store,
        &["seed", &refused.display().to_string()],
    );
    assert_eq!(output.status.code(), Some(2), "stderr {}", stderr(&output));
    assert!(
        std::fs::symlink_metadata(&target).is_err(),
        "a refused seed created the symlink's target"
    );
    assert_eq!(tree(directory.path()), before, "a refused seed wrote files");
}

/// Both providers: a host document whose tenant the provider refuses (`""`; the host decoder
/// accepts it, `crates/ekr/src/host.rs` pins that). The seed document is admissible, so admission
/// passes, the open-or-create constructor provisions the store, and only then does
/// `EventlogStore::assemble` refuse the tenant. The seed is refused (exit 1) and must leave
/// nothing behind.
#[test]
fn a_seed_refused_for_its_tenant_leaves_no_store() {
    for backend in ["file", "sqlite"] {
        let directory = tempfile::tempdir().unwrap();
        let mut host: serde_json::Value =
            serde_json::from_slice(&std::fs::read(fixture("host.json")).unwrap()).unwrap();
        host["tenant"] = serde_json::Value::String(String::new());
        let host_path = directory.path().join("host.json");
        std::fs::write(&host_path, serde_json::to_vec(&host).unwrap()).unwrap();
        let before = tree(directory.path());
        let store = directory.path().join("store");
        let output = ekr(
            &host_path,
            backend,
            &store,
            &["seed", &fixture("seed.yaml").display().to_string()],
        );
        assert_ne!(
            output.status.code(),
            Some(0),
            "{backend}: a seed under an empty tenant was accepted"
        );
        assert!(
            std::fs::symlink_metadata(&store).is_err(),
            "{backend}: the seed refused for its tenant left a store at the path; stderr {}",
            stderr(&output)
        );
        assert_eq!(tree(directory.path()), before, "{backend}: files created");
    }
}

/// `docs/cli.md` `store-not-found`: "`--store` names a path that holds nothing". An existing empty
/// directory (file) or empty file (SQLite) holds nothing, and a read verb must say so by name, and
/// create nothing there.
#[test]
fn a_read_verb_on_an_existing_empty_path_is_store_not_found() {
    for backend in ["file", "sqlite"] {
        let directory = tempfile::tempdir().unwrap();
        let store = directory.path().join("store");
        if backend == "file" {
            std::fs::create_dir(&store).unwrap();
        } else {
            std::fs::write(&store, b"").unwrap();
        }
        let before = tree(directory.path());
        let output = ekr(&fixture("host.json"), backend, &store, &["head"]);
        assert_eq!(
            tree(directory.path()),
            before,
            "{backend}: head created files"
        );
        assert_eq!(
            output.status.code(),
            Some(1),
            "{backend}: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).starts_with("ekr: store-not-found: "),
            "{backend}: an existing empty path is not named store-not-found: {}",
            stderr(&output)
        );
    }
}
