//! `ekr seed --evidence <file>`: evidence payloads read from files instead of pasted into the seed
//! as byte lists, checked by the kernel exactly as a pasted payload is.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];

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

/// The printed example seed with its `evidence_payloads` section replaced by `{}`, and one file per
/// payload it held, in document order.
fn example_without_payloads(directory: &Path) -> (PathBuf, Vec<PathBuf>) {
    let example = text(&["example", "ekr-seed/2"]);
    let lines: Vec<&str> = example.lines().collect();
    let start = lines
        .iter()
        .position(|line| *line == "evidence_payloads:")
        .unwrap();
    let mut files = Vec::new();
    for line in &lines[start + 1..] {
        let Some((_, list)) = line.trim().split_once(": [") else {
            continue;
        };
        let bytes: Vec<u8> = list
            .trim_end_matches(']')
            .split(',')
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().parse().unwrap())
            .collect();
        let file = directory.join(format!("payload-{}.txt", files.len()));
        std::fs::write(&file, bytes).unwrap();
        files.push(file);
    }
    assert_eq!(files.len(), 2, "the example seed carries two payloads");
    let mut document = lines[..start].join("\n");
    document.push_str("\nevidence_payloads: {}\n");
    let path = directory.join("seed.yaml");
    std::fs::write(&path, document).unwrap();
    (path, files)
}

struct Store {
    _directory: tempfile::TempDir,
    host: PathBuf,
    store: PathBuf,
    backend: &'static str,
}

impl Store {
    fn new(backend: &'static str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let host = directory.path().join("host.json");
        std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
        let store = match backend {
            "file" => directory.path().join("store"),
            _ => directory.path().join("state.db"),
        };
        Self {
            _directory: directory,
            host,
            store,
            backend,
        }
    }

    fn run(&self, args: &[&str], extra: &[PathBuf]) -> Output {
        let mut command = ekr();
        command
            .arg("--host")
            .arg(&self.host)
            .arg("--store")
            .arg(&self.store);
        command.args(["--backend", self.backend]).args(args);
        for file in extra {
            command.arg("--evidence").arg(file);
        }
        command.output().unwrap()
    }

    fn evidence_root(&self) -> String {
        let output = self.run(&["head"], &[]);
        let head: Value = serde_json::from_slice(&output.stdout).unwrap();
        head["root"]["evidence_root"].as_str().unwrap().to_owned()
    }
}

/// Asserts: a seed with `evidence_payloads: {}` plus `--evidence` for each payload file seeds
/// revision 0 on both providers, with the same evidence root as the example seed with its bytes
/// pasted; the same stripped seed without the files is refused as seed-evidence-payload-missing.
#[test]
fn evidence_files_seed_the_same_evidence_as_pasted_payloads() {
    for backend in BACKENDS {
        let scratch = tempfile::tempdir().unwrap();
        let (stripped, files) = example_without_payloads(scratch.path());
        let pasted = scratch.path().join("pasted.yaml");
        std::fs::write(&pasted, text(&["example", "ekr-seed/2"])).unwrap();

        let from_files = Store::new(backend);
        let output = from_files.run(&["seed", stripped.to_str().unwrap()], &files);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(result["result"]["revision"], 0, "{backend}: {result}");

        let from_document = Store::new(backend);
        let output = from_document.run(&["seed", pasted.to_str().unwrap()], &[]);
        assert_eq!(output.status.code(), Some(0), "{backend}");
        assert_eq!(
            from_files.evidence_root(),
            from_document.evidence_root(),
            "{backend}"
        );

        let without = Store::new(backend);
        let output = without.run(&["seed", stripped.to_str().unwrap()], &[]);
        assert_eq!(output.status.code(), Some(2), "{backend}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("seed-evidence-payload-missing"),
            "{backend}: {stderr}"
        );
    }
}

/// Asserts: a payload given both in the document and as a file seeds once, with the pasted seed's
/// evidence root; an `--evidence` path that does not exist exits 1 and creates no store.
#[test]
fn a_repeated_payload_lands_once_and_an_unreadable_file_is_a_fault() {
    for backend in BACKENDS {
        let scratch = tempfile::tempdir().unwrap();
        let (_, files) = example_without_payloads(scratch.path());
        let pasted = scratch.path().join("pasted.yaml");
        std::fs::write(&pasted, text(&["example", "ekr-seed/2"])).unwrap();

        let both = Store::new(backend);
        let output = both.run(
            &["seed", pasted.to_str().unwrap()],
            &[files[0].clone(), files[0].clone()],
        );
        assert_eq!(
            output.status.code(),
            Some(0),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let reference = Store::new(backend);
        assert_eq!(
            reference
                .run(&["seed", pasted.to_str().unwrap()], &[])
                .status
                .code(),
            Some(0)
        );
        assert_eq!(both.evidence_root(), reference.evidence_root(), "{backend}");

        let missing = Store::new(backend);
        let output = missing.run(
            &["seed", pasted.to_str().unwrap()],
            &[scratch.path().join("no-such-file")],
        );
        assert_eq!(output.status.code(), Some(1), "{backend}");
        assert!(output.stdout.is_empty(), "{backend}");
        assert!(
            !missing.store.exists(),
            "{backend}: a refused seed created {}",
            missing.store.display()
        );
    }
}
