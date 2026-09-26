//! Adversary pass 2 on `story:store-open-semantics` (unit `p5-01-store-open`).
//!
//! Attacks the pass-1 corrections: seed admission now runs before the store is opened, and an
//! existing-only open maps a path "holding no store" to `store-not-found` by looking at the path
//! (`holds_something` in `crates/ekr-store/src/eventlog.rs`), not at whether a store is there.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const OTHER_OPERATOR: &str = "00000000-0000-4000-8000-000000000103";

fn fixture(name: &str) -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time");
    Path::new(&manifest)
        .join("tests/fixtures/retraction")
        .join(name)
}

fn command(host: &Path, backend: &str, store: &Path, verb: &[&str]) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    command
        .arg("--host")
        .arg(host)
        .arg("--store")
        .arg(store)
        .args(["--backend", backend])
        .args(verb)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn ekr(host: &Path, backend: &str, store: &Path, verb: &[&str]) -> Output {
    command(host, backend, store, verb).output().unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Every path under `root`, relative to it, with its length for files.
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

/// A copy of the fixture host with `from` replaced by `to` everywhere, written beside the store.
fn edited_host(directory: &Path, from: &str, to: &str) -> PathBuf {
    let host = std::fs::read_to_string(fixture("host.json")).unwrap();
    let edited = host.replace(from, to);
    assert_ne!(edited, host, "the host edit applies");
    let path = directory.join("other-host.json");
    std::fs::write(&path, edited).unwrap();
    path
}

/// `docs/cli.md` "The host document": "after `ekr seed`, a host document whose authority differs
/// is refused (`bootstrap-authority-mismatch`, exit 1)", and the refusal table lists that row for
/// "any store verb". Seed a store, then run the same `ekr seed seed.yaml` again under a host whose
/// authority differs. `head` under that host proves it is the documented mismatch; `seed` must
/// report the same, since it is a store verb.
fn seed_under_another_authority(from: &str, to: &str) {
    for backend in ["file", "sqlite"] {
        let directory = tempfile::tempdir().unwrap();
        let store = directory.path().join("store");
        let seed = fixture("seed.yaml").display().to_string();
        let first = ekr(&fixture("host.json"), backend, &store, &["seed", &seed]);
        assert_eq!(
            first.status.code(),
            Some(0),
            "{backend}: {}",
            stderr(&first)
        );
        let other = edited_host(directory.path(), from, to);

        let head = ekr(&other, backend, &store, &["head"]);
        assert_eq!(head.status.code(), Some(1), "{backend}: {}", stderr(&head));
        assert!(
            stderr(&head).contains("bootstrap-authority-mismatch"),
            "{backend}: the control is not the documented mismatch: {}",
            stderr(&head)
        );

        let again = ekr(&other, backend, &store, &["seed", &seed]);
        assert_eq!(
            (
                again.status.code(),
                stderr(&again).contains("bootstrap-authority-mismatch")
            ),
            (Some(1), true),
            "{backend}: `ekr seed` under a host whose authority differs is not the documented \
             bootstrap-authority-mismatch, exit 1; stderr {}",
            stderr(&again)
        );
    }
}

/// The host names a different operator (freshly minted, registered, the profile unchanged). The
/// seed's `extracted_by` names the old one, so `Runtime::admit_seed`, which now runs before the
/// store is opened, refuses it as `seed-attribution-mismatch` before the store is ever consulted.
#[test]
fn seed_under_a_host_naming_another_operator_is_bootstrap_authority_mismatch() {
    seed_under_another_authority(OPERATOR, OTHER_OPERATOR);
}

/// The host only renames the validator: the seed stays admissible, so this runs the path the
/// base commit ran for every such seed, with the store consulted first.
#[test]
fn seed_under_a_host_renaming_the_validator_is_bootstrap_authority_mismatch() {
    seed_under_another_authority("Runtime validator", "Runtime checker");
}

/// Page 1 of an empty WAL-mode SQLite database, as `SqliteEventStore::open` leaves the file after
/// `PRAGMA journal_mode=WAL` and before `create_tables` commits: the state a killed `ekr seed`
/// leaves behind, and the one a concurrent reader sees mid-seed. Byte-identical to
/// `sqlite3 f 'PRAGMA journal_mode=WAL;'` (sqlite 3.50).
fn empty_wal_database() -> Vec<u8> {
    let mut page = vec![0_u8; 4096];
    page[..16].copy_from_slice(b"SQLite format 3\0");
    page[16..28].copy_from_slice(&[
        0x10, 0x00, 0x02, 0x02, 0x00, 0x40, 0x20, 0x20, 0x00, 0x00, 0x00, 0x01,
    ]);
    page[28..32].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]);
    page[92..100].copy_from_slice(&[0x00, 0x00, 0x00, 0x01, 0x00, 0x2e, 0x95, 0xcc]);
    page[100..105].copy_from_slice(&[0x0d, 0x00, 0x00, 0x00, 0x00]);
    page[105] = 0x10;
    page
}

/// Acceptance 1: a read verb refuses "a path with no store with a named refusal, creating
/// nothing". A SQLite database holding no owner table holds no store; `holds_something` sees a
/// non-empty file and hands it to the provider, whose refusal is not the named one.
#[test]
fn a_read_verb_on_a_sqlite_database_with_no_tables_is_store_not_found() {
    let directory = tempfile::tempdir().unwrap();
    let store = directory.path().join("store.sqlite");
    std::fs::write(&store, empty_wal_database()).unwrap();
    let before = tree(directory.path());
    let output = ekr(&fixture("host.json"), "sqlite", &store, &["head"]);
    assert_eq!(tree(directory.path()), before, "head changed the path");
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        stderr(&output).starts_with("ekr: store-not-found: "),
        "a SQLite database with no store in it is not store-not-found: {}",
        stderr(&output)
    );
}

/// The file-provider counterpart: the directory holds only `writer.lock`, the first thing the
/// provider's open-or-create writes (`Journal::open_with_creation`) before `manifest.json`.
#[test]
fn a_read_verb_on_a_file_store_directory_holding_only_its_lock_is_store_not_found() {
    let directory = tempfile::tempdir().unwrap();
    let store = directory.path().join("store");
    std::fs::create_dir(&store).unwrap();
    std::fs::write(store.join("writer.lock"), b"").unwrap();
    let before = tree(directory.path());
    let output = ekr(&fixture("host.json"), "file", &store, &["head"]);
    assert_eq!(tree(directory.path()), before, "head changed the path");
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert!(
        stderr(&output).starts_with("ekr: store-not-found: "),
        "a directory holding only a lock file is not store-not-found: {}",
        stderr(&output)
    );
}

/// A read verb racing the first `ekr seed` on SQLite. Each `head` may see no store yet
/// (`store-not-found`), a provisioned store the seed has not reached (`NotSeeded`), or the seeded
/// head; any other answer is a state the documentation does not name.
#[test]
fn sqlite_head_racing_the_first_seed_answers_only_documented_states() {
    let seed = fixture("seed.yaml").display().to_string();
    let host = fixture("host.json");
    let mut undocumented = Vec::new();
    for round in 0..24_u64 {
        let directory = tempfile::tempdir().unwrap();
        let store = directory.path().join("store.sqlite");
        let seeding = command(&host, "sqlite", &store, &["seed", &seed])
            .spawn()
            .unwrap();
        let mut heads = Vec::new();
        for _ in 0..6 {
            heads.push(command(&host, "sqlite", &store, &["head"]).spawn().unwrap());
            std::thread::sleep(std::time::Duration::from_millis(round % 4));
        }
        let seeded = seeding.wait_with_output().unwrap();
        assert_eq!(seeded.status.code(), Some(0), "{}", stderr(&seeded));
        for head in heads {
            let output = head.wait_with_output().unwrap();
            let text = stderr(&output);
            let documented = output.status.success()
                || text.starts_with("ekr: store-not-found: ")
                || text.contains("the lineage has no seed");
            if !documented {
                undocumented.push(format!("round {round}: {:?} {text}", output.status.code()));
            }
        }
    }
    assert!(
        undocumented.is_empty(),
        "{} head(s) racing a seed answered an undocumented state:\n{}",
        undocumented.len(),
        undocumented.join("")
    );
}
