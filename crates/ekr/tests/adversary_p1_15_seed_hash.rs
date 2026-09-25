//! Adversary pass 1 on `story:seed-evidence-content-hash` (wave p1-15): `ekr hash` against the
//! kernel's own check, and the guide's ADDING EVIDENCE TO A SEED paragraph read as the contract.

use std::io::Write;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];

/// The printed seed's first evidence payload, "Alice is CEO of Acme.", and its content hash.
const ALICE: &str = "Alice is CEO of Acme.";
const ALICE_HASH: &str = "2f954f8f77731e11a4dca21e0bb6c566f719aae4116838f3095f60649e5429db";

fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
        command.env_remove(var);
    }
    command
}

fn run(args: &[&str]) -> Output {
    ekr().args(args).output().unwrap()
}

fn text(args: &[&str]) -> String {
    let output = run(args);
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

fn byte_list(bytes: &[u8]) -> String {
    let values: Vec<String> = bytes.iter().map(ToString::to_string).collect();
    format!("[{}]", values.join(", "))
}

fn hash_stdin(bytes: &[u8]) -> Value {
    let mut child = ekr()
        .args(["hash", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let owned = bytes.to_vec();
    let writer = std::thread::spawn(move || stdin.write_all(&owned).unwrap());
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert_eq!(output.status.code(), Some(0));
    serde_json::from_slice(&output.stdout).unwrap()
}

/// Seeds `seed` (YAML text) into a fresh store of `backend`; returns the process output.
fn seed(backend: &str, seed: &str) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let host = directory.path().join("host.json");
    std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
    let document = directory.path().join("seed.yaml");
    std::fs::write(&document, seed).unwrap();
    let store = match backend {
        "file" => directory.path().join("store"),
        _ => directory.path().join("state.db"),
    };
    ekr()
        .arg("--host")
        .arg(&host)
        .arg("--store")
        .arg(&store)
        .args(["--backend", backend, "seed"])
        .arg(&document)
        .output()
        .unwrap()
}

/// The guide: "exit 1  a fault: provider, verification, unreadable input, ...". A payload path
/// that does not exist, or names a directory, is unreadable input.
///
/// Asserts: `ekr hash <missing>` and `ekr hash <directory>` exit 1 with nothing on stdout.
#[test]
fn hash_of_unreadable_input_is_a_fault_exit_1() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("no-such-payload");
    for path in [missing.as_path(), directory.path()] {
        let output = ekr().arg("hash").arg(path).output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stdout.is_empty(), "{}", path.display());
    }
}

/// `ekr hash` equals `ContentHash::of_bytes` for bytes that are not UTF-8, and for a payload
/// larger than one read buffer, from a file and from stdin.
///
/// Asserts: content_hash and byte_len agree with `of_bytes` and the byte count on both inputs.
#[test]
fn hash_equals_the_kernel_for_non_utf8_and_large_payloads() {
    let directory = tempfile::tempdir().unwrap();
    let non_utf8: Vec<u8> = vec![0xff, 0xfe, 0x00, b'Z', 0xc3, 0x28, b'\r', b'\n'];
    let large: Vec<u8> = (0..(2 * 1024 * 1024 + 1))
        .map(|i: u32| (i % 251) as u8)
        .collect();
    for bytes in [non_utf8, large] {
        let path = directory.path().join("payload.bin");
        std::fs::write(&path, &bytes).unwrap();
        let expected = ekr_core::ContentHash::of_bytes(&bytes).to_hex();
        let from_file = json(&["hash", &path.display().to_string()]);
        let from_stdin = hash_stdin(&bytes);
        for printed in [from_file, from_stdin] {
            assert_eq!(printed["content_hash"], expected.as_str(), "{printed}");
            assert_eq!(printed["byte_len"], bytes.len(), "{printed}");
        }
    }
}

/// The guide's ADDING EVIDENCE TO A SEED, followed literally for payloads the example does not
/// show: not UTF-8, and empty. The hash comes from `ekr hash`, the entry and the key carry it,
/// and the key's value is the byte list.
///
/// Asserts: on both providers `ekr seed` exits 0.
#[test]
fn the_guide_procedure_seeds_a_non_utf8_and_an_empty_payload() {
    let host: Value = serde_json::from_str(&text(&["example", "ekr.cli-host/1"])).unwrap();
    let operator = host["context"]["operator"].as_str().unwrap().to_owned();
    let directory = tempfile::tempdir().unwrap();
    let mut refused = Vec::new();
    for bytes in [vec![0xffu8, 0xfe, 0x00, b'Z'], Vec::new()] {
        let path = directory.path().join("payload.txt");
        std::fs::write(&path, &bytes).unwrap();
        let hash = json(&["hash", &path.display().to_string()])["content_hash"]
            .as_str()
            .unwrap()
            .to_owned();
        let evidence = json(&["mint", "evidence"])["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let printed = text(&["example", "ekr-seed/2"]);
        let entry = format!(
            "\n    evidence:\n      {evidence}:\n        id: {evidence}\n        source: !HumanStatement\n          identity: Runtime operator\n        content_hash: {hash}\n        extracted_by: {operator}\n        observed_at: 1773273600000\n        confidence: 10000\n"
        );
        let mut document = printed.replace("\n    evidence:\n", &entry);
        document.push_str(&format!("  {hash}: {}\n", byte_list(&bytes)));
        for backend in BACKENDS {
            let output = seed(backend, &document);
            if output.status.code() != Some(0) {
                refused.push(format!(
                    "{backend} {bytes:?}: exit {:?}: {}",
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
    }
    assert!(refused.is_empty(), "{refused:#?}");
}

/// A half-applied correction: the author changes the hash in one of the two places the guide
/// names (the entry's `content_hash`, the `evidence_payloads` key) and not in the other.
///
/// Rewritten by the coordinator after the wave p1-15 correction (`story:seed-evidence-content-hash`):
/// the kernel keeps the code `seed-evidence-payload-missing` for this input, and its reason now
/// names both hashes, which is what the guide says. Asserts: on both providers, for either half,
/// the refusal is exit 2 and stderr names `seed-evidence-payload-missing` and both hashes.
#[test]
fn a_half_applied_hash_correction_is_refused_naming_both_hashes() {
    let other = json(&["hash", "-"])["content_hash"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_ne!(other, ALICE_HASH);
    let printed = text(&["example", "ekr-seed/2"]);
    let entry_line = format!("content_hash: {ALICE_HASH}");
    let key_line = format!("  {ALICE_HASH}: {}", byte_list(ALICE.as_bytes()));
    assert_eq!(printed.matches(&entry_line).count(), 1);
    assert_eq!(printed.matches(&key_line).count(), 1);
    let entry_differs = printed.replace(&entry_line, &format!("content_hash: {other}"));
    let key_differs = printed.replace(
        &key_line,
        &format!("  {other}: {}", byte_list(ALICE.as_bytes())),
    );
    let mut wrong = Vec::new();
    for (half, document) in [("entry", &entry_differs), ("key", &key_differs)] {
        for backend in BACKENDS {
            let output = seed(backend, document);
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            let named = output.status.code() == Some(2)
                && stderr.contains("seed-evidence-payload-missing")
                && stderr.contains(ALICE_HASH)
                && stderr.contains(&other);
            if !named {
                wrong.push(format!(
                    "{half} differs, {backend}: exit {:?}: {}",
                    output.status.code(),
                    stderr.trim_end()
                ));
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
