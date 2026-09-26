//! `story:store-open-semantics`, acceptance 1, through fresh `ekr` processes on both providers:
//! a verb that cannot succeed on an empty store opens an existing store only, and refuses a path
//! holding none as `store-not-found`, the named configuration fault (exit 1), creating no file and
//! no directory; a seed refused at admission leaves no store behind. Only an admissible `seed`
//! creates a store.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

const ALICE: &str = "00000000-0000-4000-8000-000000000501";
const T_ALICE: &str = "00000000-0000-4000-8000-000000000601";
const BACKENDS: [&str; 2] = ["file", "sqlite"];

fn fixture(name: &str) -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time");
    Path::new(&manifest)
        .join("tests/fixtures/retraction")
        .join(name)
}

fn arg(name: &str) -> String {
    fixture(name).display().to_string()
}

fn ekr(backend: &str, store: &Path, verb: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ekr"))
        .arg("--host")
        .arg(fixture("host.json"))
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

/// Every path under `root`, relative to it, directories included.
fn tree(root: &Path) -> BTreeSet<PathBuf> {
    fn walk(root: &Path, at: &Path, into: &mut BTreeSet<PathBuf>) {
        for entry in std::fs::read_dir(at).unwrap() {
            let path = entry.unwrap().path();
            into.insert(path.strip_prefix(root).unwrap().to_path_buf());
            if path.is_dir() {
                walk(root, &path, into);
            }
        }
    }
    let mut all = BTreeSet::new();
    walk(root, root, &mut all);
    all
}

/// Every verb that opens the store and cannot succeed on a store that does not exist yet.
fn non_seed_verbs() -> Vec<Vec<String>> {
    [
        vec!["snapshot"],
        vec!["explain", ALICE],
        vec!["head"],
        vec!["transactions"],
        vec!["ontology"],
        vec!["validate", T_ALICE, "--against", "0"],
        vec!["commit", T_ALICE],
    ]
    .into_iter()
    .map(|verb| verb.into_iter().map(str::to_owned).collect())
    .chain(std::iter::once(vec![
        "propose".to_owned(),
        arg("propose-alice.yaml"),
    ]))
    .collect()
}

#[test]
fn a_verb_other_than_seed_refuses_a_path_with_no_store_and_creates_nothing() {
    for backend in BACKENDS {
        for verb in non_seed_verbs() {
            let verb: Vec<&str> = verb.iter().map(String::as_str).collect();
            for missing in ["nostore", "missing/nested/store"] {
                let directory = tempfile::tempdir().unwrap();
                let before = tree(directory.path());
                let store = directory.path().join(missing);
                let output = ekr(backend, &store, &verb);
                let stderr = String::from_utf8_lossy(&output.stderr);
                let what = format!("{backend} {verb:?} at {missing}");
                assert_eq!(
                    output.status.code(),
                    Some(1),
                    "{what}: a store configuration fault exits 1; stderr {stderr}"
                );
                assert!(
                    stderr.starts_with("ekr: store-not-found: "),
                    "{what}: stderr {stderr:?}"
                );
                assert!(output.stdout.is_empty(), "{what}: a refusal wrote a result");
                assert!(
                    std::fs::symlink_metadata(&store).is_err(),
                    "{what}: the refused verb left something at the store path"
                );
                assert_eq!(
                    tree(directory.path()),
                    before,
                    "{what}: the refused verb created a file or directory"
                );
            }
        }
    }
}

/// A seed the kernel refuses at admission — after the document parsed — leaves no store behind,
/// and the same path then seeds with an admissible document.
#[test]
fn a_seed_refused_at_admission_leaves_no_store_behind() {
    let seed = std::fs::read_to_string(fixture("seed.yaml")).unwrap();
    let refusals = [
        (
            "seed-evidence-payload-mismatch",
            seed.replacen("\n  - 65\n", "\n  - 66\n", 1),
        ),
        (
            "seed-attribution-mismatch",
            seed.replacen(
                "extracted_by: 00000000-0000-4000-8000-000000000101",
                "extracted_by: 00000000-0000-4000-8000-000000000102",
                1,
            ),
        ),
    ];
    for backend in BACKENDS {
        for (code, document) in &refusals {
            assert_ne!(*document, seed, "{code}: the edit applies");
            let directory = tempfile::tempdir().unwrap();
            let refused = directory.path().join("refused.yaml");
            std::fs::write(&refused, document).unwrap();
            let before = tree(directory.path());
            let store = directory.path().join("store");
            let output = ekr(backend, &store, &["seed", &refused.display().to_string()]);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let what = format!("{backend} {code}");
            assert_eq!(output.status.code(), Some(2), "{what}: stderr {stderr}");
            assert!(
                stderr.starts_with(&format!("ekr: ekr.kernel.InvalidSeed: {code}")),
                "{what}: stderr {stderr:?}"
            );
            assert!(
                std::fs::symlink_metadata(&store).is_err(),
                "{what}: the refused seed left a store at the path"
            );
            assert_eq!(
                tree(directory.path()),
                before,
                "{what}: something was created"
            );

            let seeded = ekr(backend, &store, &["seed", &arg("seed.yaml")]);
            assert_eq!(
                seeded.status.code(),
                Some(0),
                "{what}: the admissible seed then creates the store: {}",
                String::from_utf8_lossy(&seeded.stderr)
            );
            let head = ekr(backend, &store, &["head"]);
            assert_eq!(
                head.status.code(),
                Some(0),
                "{what}: a read verb opens the store the seed created: {}",
                String::from_utf8_lossy(&head.stderr)
            );
        }
    }
}
