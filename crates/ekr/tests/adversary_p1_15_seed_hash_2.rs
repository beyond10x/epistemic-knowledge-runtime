//! Adversary pass 2 on `story:seed-evidence-content-hash` (wave p1-15): the correction's
//! `payload_yaml`, the `seed-evidence-payload-missing` reason, and the guide's store sentences.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];

const ALICE_ID: &str = "00000000-0000-4000-8000-000000000401";
const ALICE_HASH: &str = "2f954f8f77731e11a4dca21e0bb6c566f719aae4116838f3095f60649e5429db";
const BOB_HASH: &str = "bd1d4dc9ba5012df5e43505418aa7728c15bca052a1ad43b1a2d00d04b75bdd6";
const MARKER: &str = "keys no evidence entry names: ";

fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
        command.env_remove(var);
    }
    command
}

fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn json(args: &[&str]) -> Value {
    serde_json::from_str(&text(args)).unwrap()
}

/// `ekr hash <path>`: (content_hash, payload_yaml) exactly as printed.
fn hash_file(path: &Path) -> (String, String) {
    let printed = json(&["hash", &path.display().to_string()]);
    (
        printed["content_hash"].as_str().unwrap().to_owned(),
        printed["payload_yaml"].as_str().unwrap().to_owned(),
    )
}

/// Runs `args` against a store of `backend` at `store`, with the example host, in `cwd`.
fn at(cwd: &Path, backend: &str, store: &Path, args: &[&str]) -> Output {
    let host = cwd.join("host.json");
    if !host.exists() {
        std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
    }
    ekr()
        .current_dir(cwd)
        .arg("--host")
        .arg(&host)
        .arg("--store")
        .arg(store)
        .args(["--backend", backend])
        .args(args)
        .output()
        .unwrap()
}

/// Seeds `document` into a fresh store; returns (seed output, head output).
fn seed_fresh(backend: &str, document: &str) -> (Output, Output) {
    let directory = tempfile::tempdir().unwrap();
    let seed = directory.path().join("seed.yaml");
    std::fs::write(&seed, document).unwrap();
    let store = match backend {
        "file" => directory.path().join("store"),
        _ => directory.path().join("state.db"),
    };
    let seeded = at(
        directory.path(),
        backend,
        &store,
        &["seed", &seed.display().to_string()],
    );
    let head = at(directory.path(), backend, &store, &["head"]);
    (seeded, head)
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The list the reason gives after "keys no evidence entry names: ", up to the end of its line.
fn unnamed_keys(stderr: &str) -> Vec<String> {
    let start = stderr
        .find(MARKER)
        .unwrap_or_else(|| panic!("no {MARKER:?} in {stderr}"));
    let rest = &stderr[start + MARKER.len()..];
    let line = rest.lines().next().unwrap_or("").trim();
    line.split(", ").map(str::to_owned).collect()
}

/// The guide, ADDING EVIDENCE TO A SEED, steps 2-4, with `payload_yaml` pasted exactly as
/// `ekr hash` printed it: every byte value 0-255 (in order and reversed), an empty payload, and
/// one of 1 MiB + 1 bytes.
///
/// Asserts: on both providers `ekr seed` exits 0 and `ekr head` then exits 0 — the kernel's own
/// hash check at seed is the proof that the YAML form round-tripped to the exact bytes.
#[test]
fn pasted_payload_yaml_seeds_every_byte_value_an_empty_and_a_large_payload() {
    let host: Value = serde_json::from_str(&text(&["example", "ekr.cli-host/1"])).unwrap();
    let operator = host["context"]["operator"].as_str().unwrap().to_owned();
    let printed = text(&["example", "ekr-seed/2"]);
    assert_eq!(printed.matches("\n    evidence:\n").count(), 1);
    assert!(printed.ends_with('\n'));
    let every: Vec<u8> = (0..=255u8).collect();
    let reversed: Vec<u8> = (0..=255u8).rev().collect();
    let large: Vec<u8> = (0..(1024 * 1024 + 1))
        .map(|i: u32| (i % 256) as u8)
        .collect();
    let directory = tempfile::tempdir().unwrap();
    let mut refused = Vec::new();
    for (name, bytes) in [
        ("every", every),
        ("reversed", reversed),
        ("empty", Vec::new()),
        ("large", large),
    ] {
        let path = directory.path().join(format!("{name}.bin"));
        std::fs::write(&path, &bytes).unwrap();
        let (hash, payload_yaml) = hash_file(&path);
        let evidence = json(&["mint", "evidence"])["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let entry = format!(
            "\n    evidence:\n      {evidence}:\n        id: {evidence}\n        source: !HumanStatement\n          identity: Runtime operator\n        content_hash: {hash}\n        extracted_by: {operator}\n        observed_at: 1773273600000\n        confidence: 10000\n"
        );
        let mut document = printed.replace("\n    evidence:\n", &entry);
        document.push_str(&format!("  {hash}: {payload_yaml}\n"));
        for backend in BACKENDS {
            let (seeded, head) = seed_fresh(backend, &document);
            if seeded.status.code() != Some(0) || head.status.code() != Some(0) {
                refused.push(format!(
                    "{name} ({} bytes) {backend}: seed exit {:?} {}; head exit {:?} {}",
                    bytes.len(),
                    seeded.status.code(),
                    stderr(&seeded).trim_end(),
                    head.status.code(),
                    stderr(&head).trim_end()
                ));
            }
        }
    }
    assert!(refused.is_empty(), "{refused:#?}");
}

/// The guide: "An entry whose `content_hash` is not a key of `evidence_payloads` is refused as
/// seed-evidence-payload-missing, naming that hash and the keys no entry names." Read as the
/// contract, on inputs with several entries and an extra key, so that a reason listing *every*
/// key (the filter at seed.rs:95-99 dropped) is told apart from one listing the unnamed keys.
///
/// Asserts, on both providers, exit 2 and exactly this key list:
/// - Alice's entry hash changed: the unnamed keys are exactly [Alice's].
/// - Alice's and Bob's entry hashes changed: exactly [Alice's, Bob's] (sorted), and the reason
///   names the first refused entry with its own hash.
/// - Alice's entry hash changed, plus a valid uncited extra key: exactly [Alice's, extra] sorted.
/// - Alice's key removed only (entry intact, Bob intact): the list reads `none`.
#[test]
fn payload_missing_names_exactly_the_keys_no_entry_names() {
    let printed = text(&["example", "ekr-seed/2"]);
    let directory = tempfile::tempdir().unwrap();
    let other_path = directory.path().join("other.txt");
    std::fs::write(&other_path, "another statement").unwrap();
    let (other, _) = hash_file(&other_path);
    let second_path = directory.path().join("second.txt");
    std::fs::write(&second_path, "a second statement").unwrap();
    let (second, _) = hash_file(&second_path);
    let extra_path = directory.path().join("extra.txt");
    std::fs::write(&extra_path, "an extra retained payload").unwrap();
    let (extra, extra_yaml) = hash_file(&extra_path);

    let alice_entry = format!("content_hash: {ALICE_HASH}");
    let bob_entry = format!("content_hash: {BOB_HASH}");
    assert_eq!(printed.matches(&alice_entry).count(), 1);
    assert_eq!(printed.matches(&bob_entry).count(), 1);
    let alice_moved = printed.replace(&alice_entry, &format!("content_hash: {other}"));
    let both_moved = alice_moved.replace(&bob_entry, &format!("content_hash: {second}"));
    let with_extra = format!("{alice_moved}  {extra}: {extra_yaml}\n");
    // Alice's key line removed; her entry still names ALICE_HASH, which is now no key.
    let alice_key_line = printed
        .lines()
        .find(|line| line.starts_with(&format!("  {ALICE_HASH}: ")))
        .unwrap()
        .to_owned();
    let key_removed = printed.replace(&format!("{alice_key_line}\n"), "");

    let mut sorted_both = vec![ALICE_HASH.to_owned(), BOB_HASH.to_owned()];
    sorted_both.sort();
    let mut sorted_extra = vec![ALICE_HASH.to_owned(), extra.clone()];
    sorted_extra.sort();

    let cases: Vec<(&str, String, Vec<String>, Vec<String>)> = vec![
        (
            "alice entry moved",
            alice_moved.clone(),
            vec![ALICE_HASH.to_owned()],
            vec![format!("evidence {ALICE_ID} has content_hash {other}")],
        ),
        (
            "both entries moved",
            both_moved,
            sorted_both,
            vec![format!("evidence {ALICE_ID} has content_hash {other}")],
        ),
        (
            "alice moved plus extra key",
            with_extra,
            sorted_extra,
            vec![format!("evidence {ALICE_ID} has content_hash {other}")],
        ),
        (
            "alice key removed",
            key_removed,
            vec!["none".to_owned()],
            vec![format!("evidence {ALICE_ID} has content_hash {ALICE_HASH}")],
        ),
    ];
    let mut wrong = Vec::new();
    for (name, document, expected_keys, needles) in &cases {
        for backend in BACKENDS {
            let (seeded, _) = seed_fresh(backend, document);
            let err = stderr(&seeded);
            if seeded.status.code() != Some(2) || !err.contains("seed-evidence-payload-missing") {
                wrong.push(format!(
                    "{name} {backend}: exit {:?}: {}",
                    seeded.status.code(),
                    err.trim_end()
                ));
                continue;
            }
            let listed = unnamed_keys(&err);
            if &listed != expected_keys {
                wrong.push(format!(
                    "{name} {backend}: listed {listed:?}, expected {expected_keys:?}"
                ));
            }
            for needle in needles {
                if !err.contains(needle.as_str()) {
                    wrong.push(format!(
                        "{name} {backend}: no {needle:?} in {}",
                        err.trim_end()
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The guide's CONFIGURATION sentences, for the relative `--store` path an agent types first:
/// "--store need not exist: `ekr seed` creates it. The file provider creates the directory and
/// any missing parents; the sqlite provider creates the database file, but its directory must
/// already exist".
///
/// Asserts: from a working directory, `--store state.db` (sqlite; its directory is the working
/// directory) and `--store a/b/store` (file) both seed, exit 0, and create what the guide says.
#[test]
fn relative_store_paths_seed_as_the_guide_says() {
    let directory = tempfile::tempdir().unwrap();
    let seed = directory.path().join("seed.yaml");
    std::fs::write(&seed, text(&["example", "ekr-seed/2"])).unwrap();
    let seed = seed.display().to_string();
    let sqlite = at(
        directory.path(),
        "sqlite",
        Path::new("state.db"),
        &["seed", &seed],
    );
    assert_eq!(sqlite.status.code(), Some(0), "{}", stderr(&sqlite));
    assert!(directory.path().join("state.db").is_file());
    let file = at(
        directory.path(),
        "file",
        Path::new("a/b/store"),
        &["seed", &seed],
    );
    assert_eq!(file.status.code(), Some(0), "{}", stderr(&file));
    assert!(directory.path().join("a/b/store").is_dir());
}
