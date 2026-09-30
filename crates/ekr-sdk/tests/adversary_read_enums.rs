//! Adversary pass 1 on `task:sdk-read-enums-tolerate-new-kinds` (wave reads-04, unit K).
//!
//! * `read::tolerant` retries a refused document without its content key and keeps the retry
//!   only when it reads as `Other`. A known kind whose content is wrong must keep failing — also
//!   a known *unit* kind, which the unit's own cases never exercise, so a retry that kept every
//!   success would stay green there.
//! * An unknown kind with a nested object value, and unknown kinds deep inside lists and records.
//! * Every `Other` writes back as `docs/sdk.md` says and reads back as `Other`.
//! * `docs/sdk.md` now promises that "every closed set of kinds in these values ends in `Other`";
//!   `Ontology` and `Snapshot` also hold `Cardinality`, `Subject` and `Predicate`, closed enums
//!   reused from the document model.
//! * `TransactionState::Other` passed to `transactions` through a real session.
//!
//! Every case that runs `ekr` drives the binary built from this checkout, by path.

#![cfg(unix)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ekr_sdk::binary::EkrBinary;
use ekr_sdk::read::{
    ChangeKind, CodeNameKind, ExplanationLink, GraphChange, MatchField, MatchTier, Ontology,
    OntologyValueType, ReadError, Reader, Snapshot, TransactionState, ViewValue,
};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{Request, Transport};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

// ---- tolerant: what must still fail ------------------------------------------------------------

/// A known unit kind of `OntologyValueType` carrying parameters is not a kind this SDK does not
/// know: it is a known kind whose content is wrong. The retry without `parameters` reads it as
/// that known kind, so only the `is_other` guard stops it being accepted with its parameters
/// silently dropped.
#[test]
fn a_known_unit_value_kind_with_parameters_is_refused_not_read_without_them() {
    for document in [
        json!({"value_kind": "String", "parameters": 5}),
        json!({"value_kind": "Float", "parameters": {"precision": 2}}),
        json!({"value_kind": "List", "parameters": {"value_kind": "Boolean", "parameters": [1]}}),
    ] {
        let read = serde_json::from_value::<OntologyValueType>(document.clone());
        assert!(read.is_err(), "{document} read as {read:?}");
    }
}

/// The mutant the unit's own cases miss: `tolerant` with its `is_other` guard dropped, keeping
/// any retry that reads. Written here over the derived reader (`OntologyValueType::deserialize`,
/// the inherent `remote = "Self"` function). The unit's two refusal inputs are still refused by
/// the mutant, so its suite stays green; the known unit kind with parameters is accepted by it,
/// so [`a_known_unit_value_kind_with_parameters_is_refused_not_read_without_them`] turns red.
#[test]
fn the_unit_cases_do_not_see_a_retry_that_keeps_every_success() {
    fn mutant(document: Value) -> Result<OntologyValueType, serde_json::Error> {
        OntologyValueType::deserialize(document.clone()).or_else(|error| {
            let mut bare = document;
            bare.as_object_mut().unwrap().remove("parameters");
            OntologyValueType::deserialize(bare).map_err(|_| error)
        })
    }
    assert!(mutant(json!({"value_kind": "NodeRef", "parameters": {"allowed_types": 7}})).is_err());
    assert_eq!(
        mutant(json!({"value_kind": "Geo", "parameters": 1})).unwrap(),
        OntologyValueType::Other
    );
    assert_eq!(
        mutant(json!({"value_kind": "String", "parameters": 5})).unwrap(),
        OntologyValueType::String
    );
}

/// `#[serde(remote = "Self")]` makes the derived, intolerant reader a public inherent function
/// named `deserialize`, and an inherent function wins over the trait's in path resolution. So
/// a consumer writing `ViewValue::deserialize(…)` — the usual spelling in a hand-written
/// `Deserialize` — gets the reader that fails on a kind a newer `ekr` adds, while
/// `docs/sdk.md` promises `Other`.
#[test]
fn the_path_a_consumer_writes_reads_an_unknown_kind_as_other() {
    assert_eq!(
        <ViewValue as serde::Deserialize>::deserialize(json!({"kind": "Geo", "value": 1})).unwrap(),
        ViewValue::Other
    );
    let value = ViewValue::deserialize(json!({"kind": "Geo", "value": 1}));
    assert_eq!(value.ok(), Some(ViewValue::Other), "ViewValue::deserialize");
    let declared = OntologyValueType::deserialize(json!({"value_kind": "Geo", "parameters": 1}));
    assert_eq!(
        declared.ok(),
        Some(OntologyValueType::Other),
        "OntologyValueType::deserialize"
    );
}

/// A known kind whose value is wrong fails with the first reading's error, not the retry's.
#[test]
fn a_known_kind_with_a_wrong_value_fails_with_its_own_error() {
    let error = serde_json::from_str::<ViewValue>(r#"{"kind":"Integer","value":"seven"}"#)
        .expect_err("a string is no Integer");
    let message = error.to_string();
    assert!(message.contains("invalid type"), "{message}");
    assert!(!message.contains("missing field"), "{message}");
}

/// A known list or record with one wrong element deep inside fails; it is never `Other`.
#[test]
fn a_wrong_value_deep_inside_a_known_kind_fails_the_whole_value() {
    let deep = r#"{"kind":"Record","value":{"a":{"kind":"List","value":[
        {"kind":"String","value":"fine"},{"kind":"Boolean","value":"yes"}]}}}"#;
    assert!(serde_json::from_str::<ViewValue>(deep).is_err());
}

// ---- tolerant: what must read ------------------------------------------------------------------

#[test]
fn an_unknown_kind_with_a_nested_object_value_reads_as_other_wherever_it_sits() {
    let unknown = json!({"kind": "Geo", "value": {"lat": {"kind": "Integer", "value": "x"},
                                                 "ring": [[1, 2], {"deep": null}]}});
    assert_eq!(
        serde_json::from_value::<ViewValue>(unknown.clone()).unwrap(),
        ViewValue::Other
    );
    let document = json!({"kind": "List", "value": [
        {"kind": "Record", "value": {"at": unknown, "n": {"kind": "Integer", "value": 3}}},
        {"kind": "Money"},
        {"kind": "Money", "value": null},
    ]});
    let read: ViewValue = serde_json::from_str(&document.to_string()).unwrap();
    assert_eq!(
        read,
        ViewValue::List(vec![
            ViewValue::Record(
                [
                    ("at".to_owned(), ViewValue::Other),
                    ("n".to_owned(), ViewValue::Integer(3)),
                ]
                .into_iter()
                .collect()
            ),
            ViewValue::Other,
            ViewValue::Other,
        ])
    );
}

/// Lists nested 60 deep (over 120 JSON levels, under serde_json's limit of 128), read from text
/// as a real read is: the innermost unknown kind is `Other`, and the innermost wrong value fails
/// the read — no stack overflow either way.
#[test]
fn a_value_nested_sixty_lists_deep_reads_and_refuses_as_a_shallow_one() {
    fn nested(depth: usize, inner: &str) -> String {
        let mut text = inner.to_owned();
        for _ in 0..depth {
            text = format!(r#"{{"kind":"List","value":[{text}]}}"#);
        }
        text
    }
    let mut read: ViewValue =
        serde_json::from_str(&nested(60, r#"{"kind":"Geo","value":{"a":[1]}}"#)).unwrap();
    let mut depth = 0;
    while let ViewValue::List(mut items) = read {
        assert_eq!(items.len(), 1);
        read = items.remove(0);
        depth += 1;
    }
    assert_eq!((depth, read), (60, ViewValue::Other));
    assert!(
        serde_json::from_str::<ViewValue>(&nested(60, r#"{"kind":"Integer","value":"x"}"#))
            .is_err()
    );
}

// ---- Other round-trips -------------------------------------------------------------------------

fn round_trip<T>(value: &T, written: &Value)
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    assert_eq!(&serde_json::to_value(value).unwrap(), written, "{value:?}");
    assert_eq!(
        &serde_json::from_value::<T>(written.clone()).unwrap(),
        value
    );
}

#[test]
fn every_other_writes_back_as_documented_and_reads_back_as_other() {
    round_trip(&ChangeKind::Other, &json!("Other"));
    round_trip(&MatchTier::Other, &json!("Other"));
    round_trip(&MatchField::Other, &json!("Other"));
    round_trip(&TransactionState::Other, &json!("Other"));
    round_trip(&CodeNameKind::Other, &json!("Other"));
    round_trip(&ViewValue::Other, &json!({"kind": "Other"}));
    round_trip(&OntologyValueType::Other, &json!({"value_kind": "Other"}));
    round_trip(&ExplanationLink::Other, &json!({"kind": "Other"}));
    round_trip(
        &ViewValue::List(vec![ViewValue::Other]),
        &json!({"kind": "List", "value": [{"kind": "Other"}]}),
    );
}

/// An EvidenceAdded of a statement recording no identity (`views.yaml`: an empty locator) reads
/// and writes back exactly; an unknown kind keeps its locator and hash.
#[test]
fn an_evidence_added_with_an_empty_locator_writes_back_exactly() {
    let hash = "d665088b6d8d615784418d2e9e79245f5aad71d0565a60fd45ed4649cb8c425c";
    let change = json!({"revision": 3, "recorded_at": 1_800_000_000_010_i64,
                        "change": "EvidenceAdded", "id": "00000000-0000-4000-8000-0000000000e1",
                        "locator": "", "content_hash": hash, "evidence": []});
    let read: GraphChange = serde_json::from_value(change.clone()).unwrap();
    assert_eq!(read.change, ChangeKind::EvidenceAdded);
    assert_eq!(read.locator.as_deref(), Some(""));
    assert_eq!(serde_json::to_value(&read).unwrap(), change);

    let mut newer = change;
    newer["change"] = json!("EvidenceRetired");
    let read: GraphChange = serde_json::from_value(newer).unwrap();
    assert_eq!(read.change, ChangeKind::Other);
    assert_eq!(
        read.content_hash.map(|h| h.to_string()).as_deref(),
        Some(hash)
    );
}

// ---- a real ekr --------------------------------------------------------------------------------

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

fn ekr_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
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

fn ekr_text(args: &[&str]) -> String {
    let output = std::process::Command::new(ekr_path())
        .args(args)
        .env_clear()
        .output()
        .unwrap();
    assert!(output.status.success(), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// The example host and seed, seeded, with a session open on the file provider.
fn seeded() -> (tempfile::TempDir, ProcessSession) {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("host.json"),
        ekr_text(&["example", "ekr.cli-host/1"]),
    )
    .unwrap();
    std::fs::write(
        directory.path().join("seed.yaml"),
        ekr_text(&["example", "ekr-seed/2"]),
    )
    .unwrap();
    let store = StoreConfig {
        host: directory.path().join("host.json"),
        store: directory.path().join("store"),
        backend: Backend::File,
    };
    let options = SessionOptions {
        current_dir: Some(directory.path().to_path_buf()),
        ..SessionOptions::default()
    };
    let binary = EkrBinary::open(ekr_path()).expect("the built ekr meets the SDK's minimum");
    let mut session = ProcessSession::start(&binary, store, options).unwrap();
    let reply = session
        .request(&Request::new(["seed", "seed.yaml"]))
        .unwrap();
    assert_eq!(reply.exit, 0, "{}", reply.stderr);
    (directory, session)
}

/// The document `ekr <argv>` answers through `session`.
fn answered(session: &mut ProcessSession, argv: &[&str]) -> Value {
    let reply = session
        .request(&Request::new(argv.iter().copied()))
        .unwrap();
    assert_eq!(reply.exit, 0, "{argv:?}: {}", reply.stderr);
    reply.document.unwrap()
}

/// `docs/sdk.md` documents that `ekr` refuses `TransactionState::Other` for `--state`: the read
/// is a usage refusal, and the session reads on afterwards.
#[test]
fn transaction_state_other_passed_to_transactions_is_a_usage_refusal() {
    let (_directory, mut session) = seeded();
    let mut reader = Reader::new(&mut session);
    let refused = reader.transactions(Some(TransactionState::Other));
    assert!(
        matches!(&refused, Err(ReadError::Usage { verb, .. }) if verb == "transactions"),
        "{refused:?}"
    );
    assert_eq!(reader.head().unwrap().revision, 0);
    assert!(reader.transactions(None).unwrap().0.is_empty());
}

/// "Every closed set of kinds in these values ends in `Other`" (`docs/sdk.md`, Typed reads):
/// the ontology's cardinality is such a set, and a cardinality a newer `ekr` adds fails the
/// whole `ontology` read.
#[test]
fn a_cardinality_a_newer_ekr_adds_does_not_fail_the_ontology_read() {
    let (_directory, mut session) = seeded();
    let mut document = answered(&mut session, &["ontology"]);
    serde_json::from_value::<Ontology>(document.clone()).expect("the ontology as printed reads");
    let property = document["node_types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .flat_map(|node_type| node_type["properties"].as_array_mut().unwrap().iter_mut())
        .next()
        .expect("the example ontology declares a property");
    property["cardinality"] = json!("AtLeastOne");
    let read = serde_json::from_value::<Ontology>(document);
    assert!(
        read.is_ok(),
        "a new cardinality fails the whole read: {read:?}"
    );
}

/// The same promise for a snapshot assertion's `subject`: a subject kind a newer `ekr` adds
/// fails the whole `snapshot` read.
#[test]
fn a_subject_kind_a_newer_ekr_adds_does_not_fail_the_snapshot_read() {
    let (_directory, mut session) = seeded();
    let mut document = answered(&mut session, &["snapshot"]);
    serde_json::from_value::<Snapshot>(document.clone()).expect("the snapshot as printed reads");
    let assertion = document["graph"]["graph"]["assertions"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .expect("the example seed holds an assertion");
    let subject = assertion["subject"].clone();
    assertion["subject"] = json!({"Assertion": subject.as_object().unwrap().values().next()});
    let read = serde_json::from_value::<Snapshot>(document);
    assert!(
        read.is_ok(),
        "a new subject kind fails the whole read: {read:?}"
    );
}
