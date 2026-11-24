//! `story:ekr-explain-evidence-fixes`, review of EKR 0.0.42 finding 19: the typed explanation
//! carries the range an evidence payload was cut to, and reads any other range of it.
//!
//! The store is the example seed with one more assertion, Acme's `legal_name`, citing one evidence
//! record of 81,900 ASCII bytes. `ekr explain --documents` answers 65,536 of them with `offset`,
//! `record_length` and `truncated: true`; [`ExplainedEvidence`] holds the three, and
//! [`Reader::explain_range`] reads the record from any offset, through a session and one-shot, on
//! both providers.

#![cfg(unix)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ekr_core::{AssertionId, ContentHash};
use ekr_sdk::binary::EkrBinary;
use ekr_sdk::read::{ExplainedEvidence, Explanation, ExplanationLink, OneShotReader, Reader};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use serde_json::Value;

const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const LEGAL: &str = "00000000-0000-4000-8000-000000000513";
const LARGE: &str = "00000000-0000-4000-8000-000000000403";
const RECORD_BYTES: usize = 81_900;
const CITED_AT: usize = 40_800;
const CITED: &str = "Acme Widgets Holding was registered in 2019 (A7)";
const DEFAULT_LIMIT: usize = 64 * 1024;

/// The workspace root, found at run time from the directory holding `Cargo.lock`.
fn workspace_root() -> PathBuf {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    manifest
        .ancestors()
        .find(|directory| directory.join("Cargo.lock").is_file())
        .expect("a workspace root above the crate")
        .to_path_buf()
}

/// The `ekr` binary of this checkout: the test step's, or built once per test process.
fn ekr_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        if let Some(path) = std::env::var_os("EKR_SDK_TEST_EKR") {
            return PathBuf::from(path);
        }
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
        let output = std::process::Command::new(cargo)
            .arg("build")
            .arg("--manifest-path")
            .arg(workspace_root().join("Cargo.toml"))
            .args(["--locked", "-p", "ekr", "--bin", "ekr"])
            .arg("--message-format=json-render-diagnostics")
            .stderr(std::process::Stdio::inherit())
            .output()
            .expect("running cargo build -p ekr");
        assert!(output.status.success(), "cargo build -p ekr failed");
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|message| {
                message["reason"] == "compiler-artifact" && message["target"]["name"] == "ekr"
            })
            .find_map(|message| message["executable"].as_str().map(PathBuf::from))
            .expect("cargo named the ekr executable it built")
    })
}

fn binary() -> EkrBinary {
    EkrBinary::open(ekr_path()).expect("the built ekr meets the SDK's minimum")
}

/// Exit 0 and stdout, from the built binary.
fn ekr(args: &[&std::ffi::OsStr]) -> Vec<u8> {
    let output = std::process::Command::new(ekr_path())
        .args(args)
        .env_clear()
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn ekr_text(args: &[&str]) -> String {
    let args: Vec<&std::ffi::OsStr> = args.iter().map(std::ffi::OsStr::new).collect();
    String::from_utf8(ekr(&args)).unwrap()
}

/// ASCII filler with `CITED` once at `CITED_AT`.
fn record() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(RECORD_BYTES);
    let mut line = 0;
    while bytes.len() < RECORD_BYTES {
        bytes.extend_from_slice(format!("Ledger line {line:05}: nothing of note.\n").as_bytes());
        line += 1;
    }
    bytes.truncate(RECORD_BYTES);
    bytes[CITED_AT..CITED_AT + CITED.len()].copy_from_slice(CITED.as_bytes());
    bytes
}

struct World {
    directory: tempfile::TempDir,
    backend: Backend,
    record: Vec<u8>,
}

impl World {
    /// The example seed with Acme's `legal_name` citing the large record, seeded one-shot.
    fn new(backend: Backend) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
            record: record(),
        };
        world.file("host.json", ekr_text(&["example", "ekr.cli-host/1"]).as_bytes());
        world.file("record.txt", &world.record);
        let hash = ContentHash::of_bytes(&world.record);
        let added = format!(
            "      {LEGAL}:
        id: {LEGAL}
        root_id: 00000000-0000-4000-8000-000000000002
        subject: !Node 00000000-0000-4000-8000-000000000303
        predicate: !Property 00000000-0000-4000-8000-000000000801
        object: !Value
          value_kind: String
          value: {CITED}
        evidence:
        - {LARGE}
        proposed_by: {OPERATOR}
        assessment: Proposed
        lifecycle: Active
        valid_time:
          from: 1577836800000
          to: null
        transaction_time:
          recorded_from: 0
          recorded_to: null
    evidence:
      {LARGE}:
        id: {LARGE}
        source: !HumanStatement
          identity: Runtime operator
        content_hash: {hash}
        extracted_by: {OPERATOR}
        observed_at: 1773273600000
        confidence: 10000
"
        );
        let example = ekr_text(&["example", "ekr-seed/2"]);
        let marker = "\n    evidence:\n";
        assert_eq!(example.matches(marker).count(), 1, "{example}");
        world.file(
            "seed.yaml",
            example
                .replacen(marker, &format!("\n{added}"), 1)
                .as_bytes(),
        );
        let seeded = world.one_shot(&[
            "seed",
            &world.path("seed.yaml"),
            "--evidence",
            &world.path("record.txt"),
        ]);
        assert_eq!(seeded["result"]["revision"], 0);
        world
    }

    fn path(&self, name: &str) -> String {
        self.directory.path().join(name).display().to_string()
    }

    fn file(&self, name: &str, contents: &[u8]) {
        std::fs::write(self.directory.path().join(name), contents).unwrap();
    }

    fn store(&self) -> StoreConfig {
        StoreConfig {
            host: self.directory.path().join("host.json"),
            store: match self.backend {
                Backend::Sqlite => self.directory.path().join("state.db"),
                _ => self.directory.path().join("store"),
            },
            backend: self.backend,
        }
    }

    fn options(&self) -> SessionOptions {
        SessionOptions {
            current_dir: Some(self.directory.path().to_path_buf()),
            ..SessionOptions::default()
        }
    }

    /// `ekr <verb>` under this world's store, run directly: its stdout as JSON.
    fn one_shot(&self, verb: &[&str]) -> Value {
        let store = self.store();
        let backend = match self.backend {
            Backend::Sqlite => "sqlite",
            _ => "file",
        };
        let mut args: Vec<&std::ffi::OsStr> = vec![
            "--host".as_ref(),
            store.host.as_os_str(),
            "--store".as_ref(),
            store.store.as_os_str(),
            "--backend".as_ref(),
            backend.as_ref(),
        ];
        args.extend(verb.iter().map(std::ffi::OsStr::new));
        serde_json::from_slice(&ekr(&args)).unwrap()
    }
}

/// The one Evidence link of an explanation.
fn evidence(explanation: &Explanation) -> &ExplainedEvidence {
    let links: Vec<&ExplainedEvidence> = explanation
        .links
        .iter()
        .filter_map(|link| match link {
            ExplanationLink::Evidence(evidence) => Some(evidence),
            _ => None,
        })
        .collect();
    assert_eq!(links.len(), 1, "{explanation:?}");
    assert_eq!(links[0].evidence.id.to_string(), LARGE);
    links[0]
}

fn reads_whole_evidence(backend: Backend) {
    let world = World::new(backend);
    let assertion: AssertionId = LEGAL.parse().unwrap();
    let mut session = ProcessSession::start(&binary(), world.store(), world.options()).unwrap();
    let mut reader = Reader::new(&mut session);
    let mut one_shot = OneShotReader::new(&binary(), world.store(), world.options());

    // The default window: the typed value says it is a range of a longer record, and writes back
    // exactly the document `ekr` printed.
    let documents = reader.explain_documents(assertion).unwrap();
    assert_eq!(
        serde_json::to_value(&documents).unwrap(),
        world.one_shot(&["explain", LEGAL, "--documents"]),
        "{backend:?}"
    );
    let window = evidence(&documents);
    assert!(window.truncated, "{backend:?}");
    assert_eq!(window.offset, Some(8_056), "{backend:?}");
    assert_eq!(window.record_length, Some(RECORD_BYTES as u64), "{backend:?}");
    let text = window.text.as_deref().unwrap();
    assert_eq!(text.len(), DEFAULT_LIMIT, "{backend:?}");
    assert!(text.contains(CITED), "{backend:?}");

    // Every range from 0, through the session, reassembles the record; one-shot reads the same.
    let mut reassembled = Vec::new();
    loop {
        let offset = reassembled.len() as u64;
        let part = reader.explain_range(assertion, offset, None).unwrap();
        assert_eq!(
            part,
            one_shot.explain_range(assertion, offset, None).unwrap(),
            "{backend:?}"
        );
        let link = evidence(&part);
        assert_eq!(link.offset, Some(offset), "{backend:?}");
        assert_eq!(link.record_length, Some(RECORD_BYTES as u64), "{backend:?}");
        assert!(link.truncated, "{backend:?}");
        let bytes = link.text.as_deref().unwrap().as_bytes();
        assert!(!bytes.is_empty(), "{backend:?}: offset {offset}");
        reassembled.extend_from_slice(bytes);
        if reassembled.len() as u64 >= link.record_length.unwrap() {
            break;
        }
    }
    assert!(reassembled == world.record, "{backend:?}: the ranges reassemble the record");

    // A limit bounds the range.
    let first = one_shot.explain_range(assertion, 0, Some(1_000)).unwrap();
    assert_eq!(
        evidence(&first).text.as_deref().map(str::as_bytes),
        Some(&world.record[..1_000]),
        "{backend:?}"
    );
}

#[test]
fn a_typed_explanation_names_its_evidence_range_and_reads_the_next_file() {
    reads_whole_evidence(Backend::File);
}

#[test]
fn a_typed_explanation_names_its_evidence_range_and_reads_the_next_sqlite() {
    reads_whole_evidence(Backend::Sqlite);
}
