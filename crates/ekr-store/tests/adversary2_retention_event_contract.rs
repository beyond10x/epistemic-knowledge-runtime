//! `ekr.store.ObjectRetentionRaised` is a **declared** event, and the bytes are the contract.
//!
//! Correction round 1 of wave p1-05 made a raise an append-only event, so the class survives a
//! reopen. The wave declared that event in `systems/ekr/domains/store.yaml` with three fields —
//! `content_hash`, `from` and `to` — and a comment saying why the last two are both carried:
//! "`from` and `to` are both carried so a reader can see the ladder was climbed and not
//! descended."
//!
//! `tests/domain_projection.rs` reads that document and checks `StorageClass` against it. It
//! checks nothing about the event, and nothing else in the suite does either, so the name and the
//! body the crate writes into stored bytes are held by no case at all. An event name and its
//! fields in an append-only log are a compatibility surface the day anything reads a log it did
//! not write, which is the same argument `domain_projection.rs` makes for the class list.
//!
//! The declaration is transcribed here rather than derived, for the reason that file gives for
//! transcribing the ladder: a case that read the shape out of the code would assert nothing.

use ekr_core::{SchemaVersionId, Timestamp};
use ekr_ontology::{Ontology, OntologyDocument, SchemaVersion};
use ekr_store::{FileStore, ObjectStore, StorageClass};
use tempfile::TempDir;

/// An ontology with no types. Built here rather than through `tests/fixture`, because this binary
/// needs no seed graph and an unused helper in an integration test is dead code `-D warnings`
/// refuses.
fn ontology() -> Ontology {
    Ontology::load(OntologyDocument {
        version: SchemaVersion::seed(SchemaVersionId::mint(), Timestamp::EPOCH),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("a document with no declarations coheres")
}

/// The event's name, as `systems/ekr/domains/store.yaml` declares it.
const DECLARED_NAME: &str = "ekr.store.ObjectRetentionRaised";

/// Its fields, as that document declares them.
const DECLARED_FIELDS: [&str; 3] = ["content_hash", "from", "to"];

/// `systems/ekr/domains/store.yaml`, as text — the same path `domain_projection.rs` reads.
fn domain_text() -> String {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../systems/ekr/domains/store.yaml"
    );
    std::fs::read_to_string(path).expect("the ESS domain is beside the crates")
}

/// The domain this tree validates declares the event this crate writes.
///
/// `ess specify validate --path systems/ekr` runs against the tree the gate runs in, and
/// `domain_projection.rs` reads the document from that same tree. If the declaration is not there,
/// the gate is green because the document and the code were never in one place to disagree.
#[test]
fn the_store_domain_declares_the_retention_raise_event_this_crate_writes() {
    let text = domain_text();

    assert!(
        text.contains("\nevents:"),
        "the store domain in this tree has no `events:` section, so the event the crate writes \
         into stored bytes is declared nowhere the gate reads"
    );
    assert!(
        text.contains(DECLARED_NAME),
        "the store domain in this tree does not declare {DECLARED_NAME}"
    );
    for field in DECLARED_FIELDS {
        assert!(
            text.contains(&format!("- name: {field}")),
            "the store domain declares {DECLARED_NAME} without the field `{field}`"
        );
    }
}

/// The body the store actually writes carries the fields the domain declares.
///
/// Measured from the provider's own journal rather than from the crate's types: these are the
/// bytes a later reader gets, and `RetentionRaised` is private, so the only honest way to ask what
/// was written is to read what was written.
#[test]
fn the_retention_raise_in_stored_bytes_carries_the_fields_the_domain_declares() {
    let directory = TempDir::new().expect("a temporary directory");
    let root = directory.path().join("revisions");
    let store = FileStore::file(&root, "ekr", ontology()).expect("the file provider opens");

    let bytes = b"a payload cached before canonical state depended on it";
    store
        .put(StorageClass::Cache, bytes, Timestamp::EPOCH)
        .expect("the cache write lands");
    store
        .put(StorageClass::Canonical, bytes, Timestamp::EPOCH)
        .expect("the canonical write raises the class");

    let written = written_event_body(&root, DECLARED_NAME)
        .unwrap_or_else(|| panic!("the store wrote no {DECLARED_NAME} into its log at all"));
    let carried: Vec<&String> = written.keys().collect();

    for field in DECLARED_FIELDS {
        assert!(
            written.contains_key(field),
            "the domain declares {DECLARED_NAME} with `{field}` and the stored body carries \
             {carried:?}"
        );
    }
}

/// The `data` object of the first event named `name` in whatever the provider wrote under `root`.
fn written_event_body(
    root: &std::path::Path,
    name: &str,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let mut files = Vec::new();
    collect_files(root, &mut files);
    for path in files {
        let Ok(raw) = std::fs::read(&path) else {
            continue;
        };
        for line in String::from_utf8_lossy(&raw).lines() {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if let Some(body) = find_named(&value, name) {
                return Some(body);
            }
        }
    }
    None
}

/// The `data` object of any object in `value` whose `name` field is `name`, at any depth.
fn find_named(
    value: &serde_json::Value,
    name: &str,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    match value {
        serde_json::Value::Object(fields) => {
            if fields.get("name").and_then(serde_json::Value::as_str) == Some(name) {
                return fields
                    .get("data")
                    .and_then(serde_json::Value::as_object)
                    .cloned();
            }
            fields.values().find_map(|nested| find_named(nested, name))
        }
        serde_json::Value::Array(items) => items.iter().find_map(|item| find_named(item, name)),
        _ => None,
    }
}

/// Every regular file under `directory`, recursively.
fn collect_files(directory: &std::path::Path, into: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, into);
        } else {
            into.push(path);
        }
    }
}
