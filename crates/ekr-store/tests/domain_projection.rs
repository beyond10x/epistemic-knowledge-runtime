//! What this crate says about `systems/ekr/domains/store.yaml`, checked against the document.
//!
//! `ekr-ontology` and `ekr-graph` each have a case of this shape, and it exists because
//! `ess specify validate` checks the document against itself and the rest of the suite checks the
//! crate against itself, so nothing reads the two together.
//!
//! `ekr-store` gets one for a specific reason. Correction round 1 of wave p1-05 gave
//! [`StorageClass`] a **retention ordering** — which class wins when the same bytes are stored
//! twice — and an ordering is a hand-written table over a hand-written list of variants. A sixth
//! class added to the domain and not to `StorageClass::ALL` would be a class with no rank, silently
//! outside the ordering that decides what a collector may delete. The list is the defect; a missing
//! entry is only its symptom, so the list is checked against the domain here rather than reviewed.
//!
//! `ekr-store` declares no YAML parser, so the document is read as text, and the scan is held to
//! its own catch: a parse that stops finding declarations fails loudly rather than passing
//! vacuously.

use ekr_store::{ObjectStore, RevisionLog, StorageClass};

/// `systems/ekr/domains/store.yaml`, as text.
fn domain_text() -> String {
    let path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("../../systems/ekr/domains/store.yaml");
    std::fs::read_to_string(path).expect("the ESS domain is beside the crates")
}

/// The variants one `kind: enum` type of the domain declares, in document order.
fn variants_of(type_name: &str) -> Vec<String> {
    let text = domain_text();
    let mut lines = text
        .lines()
        .skip_while(|line| line.trim_start() != format!("- name: {type_name}"));
    assert!(
        lines.next().is_some(),
        "the domain declares no type named {type_name}"
    );

    let mut variants = Vec::new();
    let mut inside = false;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("- name: ") {
            break;
        }
        if trimmed == "variants:" {
            inside = true;
            continue;
        }
        if inside {
            if let Some(variant) = trimmed.strip_prefix("- ") {
                variants.push(variant.to_owned());
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                break;
            }
        }
    }
    assert!(
        !variants.is_empty(),
        "the variant scan is broken, not the domain: {type_name}"
    );
    variants
}

/// `StorageClass::ALL` is exactly `ekr.store.StorageClass`, in the order the domain declares it.
///
/// This is the case that keeps the retention ordering honest. Every other case about retention
/// walks `ALL`, so a class the domain declares and `ALL` omits would be a class no case touches,
/// with no rank and no place in [`StorageClass::strongest`].
#[test]
fn the_storage_classes_are_the_ones_the_domain_declares() {
    let declared = variants_of("ekr.store.StorageClass");
    let held: Vec<String> = StorageClass::ALL
        .iter()
        .map(|class| class.name().to_owned())
        .collect();

    assert_eq!(
        held, declared,
        "the crate's storage classes and the domain's disagree"
    );
    assert_eq!(
        StorageClass::ALL.len(),
        declared.len(),
        "and there are as many of them"
    );
}

/// Every class has its own rank: the ordering is total and no two classes tie.
///
/// A tie would make `strongest` pick by argument order, which is the order-dependence the whole
/// rule exists to remove.
#[test]
fn every_storage_class_has_its_own_retention_rank() {
    let mut ranks: Vec<u8> = StorageClass::ALL
        .iter()
        .map(StorageClass::retention_rank)
        .collect();
    ranks.sort_unstable();
    ranks.dedup();

    assert_eq!(
        ranks.len(),
        StorageClass::ALL.len(),
        "two classes share a retention rank, so `strongest` would answer by argument order"
    );
}

/// The ranks are design § 37's ladder, most durable first.
///
/// Transcribed rather than derived: the ordering is a reading of what § 37 says each class is for,
/// and a case that derived it from the code would assert nothing. `Canonical` is "durable,
/// revisioned, strongly governed"; `Cache` is "reproducible, and freely deletable"; `Ephemeral` is
/// short-lived working state and is the weakest of the five.
#[test]
fn the_retention_ladder_is_the_one_section_thirty_seven_describes() {
    let mut by_strength: Vec<StorageClass> = StorageClass::ALL.to_vec();
    by_strength.sort_by_key(|class| std::cmp::Reverse(class.retention_rank()));

    assert_eq!(
        by_strength,
        vec![
            StorageClass::Canonical,
            StorageClass::Provenance,
            StorageClass::Incubating,
            StorageClass::Cache,
            StorageClass::Ephemeral,
        ]
    );
}

/// The **derived** `Ord` is not the retention order, and nothing may read it as one.
///
/// `StorageClass` derives `Ord` so that `StoredObject` is sortable, and the derived order is
/// declaration position — which runs the opposite way to durability. `Canonical < Ephemeral` there,
/// so `max()` over two classes picks the one that may be deleted first. This case pins the trap so
/// that a later reader reaching for `max` finds a case that says why not.
#[test]
fn the_derived_ordering_is_not_the_retention_ordering() {
    assert!(
        StorageClass::Canonical < StorageClass::Ephemeral,
        "the derived order is declaration position"
    );
    assert!(
        StorageClass::Canonical.retention_rank() > StorageClass::Ephemeral.retention_rank(),
        "and retention runs the other way"
    );
    assert_eq!(
        StorageClass::ALL
            .iter()
            .copied()
            .max()
            .expect("five classes"),
        StorageClass::Ephemeral,
        "so the derived max is the weakest class, which is why `strongest` exists"
    );
}

/// `strongest` is commutative, idempotent, and never weaker than either side — over all
/// twenty-five pairs rather than a chosen few.
#[test]
fn the_strongest_of_two_classes_does_not_depend_on_which_arrived_first() {
    for left in StorageClass::ALL {
        assert_eq!(left.strongest(left), left, "idempotent: {left:?}");
        for right in StorageClass::ALL {
            let folded = left.strongest(right);
            assert_eq!(
                folded,
                right.strongest(left),
                "commutative, or the answer depends on write order: {left:?} and {right:?}"
            );
            assert!(
                folded.retention_rank() >= left.retention_rank()
                    && folded.retention_rank() >= right.retention_rank(),
                "never weaker than either requirement: {left:?} and {right:?}"
            );
            assert!(
                folded == left || folded == right,
                "and it is one of the two, not a third class: {left:?} and {right:?}"
            );
        }
    }
}

// Every event this crate writes, against the fields the domain declares for it.
//
// Correction round 2 found `ekr.store.ObjectRetentionRaised` written with one field where the
// domain declares three. Fixing that one event would have left the class open — this file already
// read that same document and checked only `StorageClass` — so the check below is over **every**
// event name the crate can write, derived from its source rather than listed here, and over the
// body each one actually puts in a log, read back out of the provider's journal.
//
// It found its own second instance immediately: `ekr.store.ObjectStored` was written by the same
// code path and declared nowhere. That declaration is now in `store.yaml` too.

use std::collections::{BTreeMap, BTreeSet};

/// `crates/ekr-store/src/eventlog.rs`, as text — the one module that names an event.
fn eventlog_source() -> String {
    let path = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("src");
    // `preparation.rs` writes `ekr.store.PublicationPrepared` (ADR 0009); every other event name
    // lives in `eventlog.rs`.
    ["eventlog.rs", "preparation.rs"]
        .iter()
        .map(|file| std::fs::read_to_string(path.join(file)).expect("the module is in this crate"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every event name the crate can write, from the constants that hold them.
///
/// Derived rather than listed, which is the whole point: a seventh event added to the crate arrives
/// in this set without anyone remembering to add it, and then has to be declared or turn the case
/// below red.
fn event_names_the_crate_writes() -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for line in eventlog_source().lines() {
        let trimmed = line.trim();
        // A constant holding a name, or a literal passed straight to `NewEvent::new`.
        let names_an_event = (trimmed.starts_with("const ") && trimmed.contains(": &str"))
            || (trimmed.starts_with('"') && trimmed.ends_with("\",") && !trimmed.contains(' '));
        if !names_an_event {
            continue;
        }
        let Some((_, rest)) = trimmed.split_once('"') else {
            continue;
        };
        let Some((literal, _)) = rest.split_once('"') else {
            continue;
        };
        // A stream *type* is also a qualified name and is not an event; events are the constants
        // whose literal names a type in `PascalCase` after the domain.
        if let Some(last) = literal.rsplit('.').next() {
            if literal.starts_with("ekr.store.")
                && last.starts_with(|c: char| c.is_ascii_uppercase())
            {
                names.insert(literal.to_owned());
            }
        }
    }
    assert!(
        !names.is_empty(),
        "the constant scan is broken, not the crate: it found no event names at all"
    );
    names
}

/// Every event the domain declares, with its field names in document order.
fn declared_events() -> BTreeMap<String, Vec<String>> {
    let text = domain_text();
    let mut declared: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut current: Option<String> = None;
    let mut in_events = false;
    for line in text.lines() {
        if line.starts_with("events:") {
            in_events = true;
            continue;
        }
        if !in_events {
            continue;
        }
        // A top-level key ends the section.
        if !line.starts_with(' ') && !line.trim().is_empty() && !line.starts_with('#') {
            break;
        }
        let trimmed = line.trim();
        if let Some(name) = trimmed.strip_prefix("- name: ekr.") {
            current = Some(format!("ekr.{name}"));
            declared.entry(format!("ekr.{name}")).or_default();
        } else if let Some(field) = trimmed.strip_prefix("- name: ") {
            if let Some(event) = &current {
                declared
                    .get_mut(event)
                    .expect("the event was entered when its name was read")
                    .push(field.to_owned());
            }
        }
    }
    assert!(
        !declared.is_empty(),
        "the event scan is broken, not the domain: it found no declarations at all"
    );
    declared
}

/// The crate writes exactly the events the domain declares — no more, and no fewer.
///
/// Both directions. An event the crate writes and the domain does not declare is bytes no reader
/// has a contract for; an event the domain declares and nothing writes is a contract describing a
/// log that does not exist.
#[test]
fn every_event_the_crate_writes_is_declared_by_the_domain() {
    let written = event_names_the_crate_writes();
    let declared: BTreeSet<String> = declared_events().into_keys().collect();

    let undeclared: Vec<&String> = written.difference(&declared).collect();
    assert!(
        undeclared.is_empty(),
        "the crate writes events the store domain does not declare: {undeclared:?}"
    );
    let unwritten: Vec<&String> = declared.difference(&written).collect();
    assert!(
        unwritten.is_empty(),
        "the store domain declares events nothing in the crate writes: {unwritten:?}"
    );
}

/// Every event's **stored body** carries exactly the fields the domain declares for it.
///
/// Read out of the provider's own journal rather than off the crate's types: the record types are
/// private, a case that serialised one would be asking the code what the code does, and the bytes
/// on disk are what a later reader actually gets. This is the check that would have caught the
/// retention raise being written with one field where three are declared.
///
/// One scenario produces both events — bytes stored under a weak class, then stored again under a
/// strong one — so no event is checked by being absent.
#[test]
fn every_event_the_crate_writes_carries_the_fields_the_domain_declares() {
    let directory = tempfile::TempDir::new().expect("a temporary directory");
    let root = directory.path().join("journal");
    {
        let store =
            ekr_store::FileStore::file(&root, "ekr", ontology()).expect("the file provider opens");
        let bytes = b"a payload stored weakly and then needed durably";
        store
            .put(StorageClass::Cache, bytes, ekr_core::Timestamp::EPOCH)
            .expect("the first write lands");
        store
            .put(StorageClass::Canonical, bytes, ekr_core::Timestamp::EPOCH)
            .expect("the raise lands");
    }
    {
        // An elected, unpublished bootstrap preparation (design § 94): the one event written
        // outside the revision and object streams.
        let store = ekr_store::FileStore::file(&root, "ekr", ontology())
            .expect("the file provider reopens")
            .under(AdmitsNothingYet);
        let record = b"a seed record staged for publication";
        let decision = ekr_store::Publication {
            event: ekr_graph::RevisionEvent {
                format: ekr_graph::RevisionEvent::FORMAT.to_owned(),
                event_id: ekr_core::EventId::mint(),
                record_hash: ekr_core::ContentHash::of_bytes(record),
                payload: ekr_graph::RevisionPayload::Seeded {
                    revision_id: ekr_core::RevisionId::mint(),
                    seed_hash: ekr_core::ContentHash::of_bytes(record),
                },
            },
            objects: [(
                ekr_core::ContentHash::of_bytes(record),
                ekr_store::PublicationObject {
                    storage_class: StorageClass::Canonical,
                    stored_at: ekr_core::Timestamp::EPOCH,
                    bytes: record.to_vec(),
                },
            )]
            .into_iter()
            .collect(),
            expected_version: 0,
        };
        let key = ekr_store::PublicationCommandKey {
            kind: ekr_store::PublicationCommandKind::Bootstrap,
            transaction_id: None,
            predecessor_event_id: None,
            predecessor_record_hash: None,
        };
        store
            .prepare(
                &key,
                ekr_core::ContentHash::of_bytes(b"bootstrap command input"),
                &decision,
                None,
            )
            .expect("the preparation is elected");
    }

    let declared = declared_events();
    let mut checked = 0usize;
    for (event, fields) in &declared {
        let body = journal_body(&root, event)
            .unwrap_or_else(|| panic!("the store wrote no {event} into its log at all"));
        let carried: BTreeSet<String> = body.keys().cloned().collect();
        let want: BTreeSet<String> = fields.iter().cloned().collect();

        assert_eq!(
            carried, want,
            "the stored body of {event} and the domain's declaration of it disagree"
        );
        checked += 1;
    }
    assert_eq!(
        checked,
        declared.len(),
        "every declared event is reached by this scenario"
    );
    assert!(checked >= 2, "the scenario produces both events, not one");
}

/// A stand-in authority over a lineage with no revisions: it names no extra objects and admits no
/// revision. Not kernel validation; `crates/ekr-kernel/tests/seed.rs` holds that.
struct AdmitsNothingYet;

impl ekr_store::CommitAuthority for AdmitsNothingYet {
    fn required_objects(
        &self,
        _history: &ekr_store::RetainedHistory,
    ) -> Result<BTreeSet<ekr_core::ContentHash>, ekr_store::StoreError> {
        Ok(BTreeSet::new())
    }
    fn replay(
        &self,
        _history: &ekr_store::RetainedHistory,
        _ontology: Option<&ekr_ontology::Ontology>,
        _revision: Option<ekr_core::RevisionNumber>,
    ) -> Result<Option<ekr_store::AdmittedRevision>, ekr_store::StoreError> {
        Ok(None)
    }
}

/// An ontology with no types, for the store this file opens.
fn ontology() -> ekr_ontology::Ontology {
    ekr_ontology::Ontology::load(ekr_ontology::OntologyDocument {
        version: ekr_ontology::SchemaVersion::seed(
            ekr_core::SchemaVersionId::mint(),
            ekr_core::Timestamp::EPOCH,
        ),
        node_types: Vec::new(),
        edge_types: Vec::new(),
    })
    .expect("a document with no declarations coheres")
}

/// The `data` object of the first event named `name` anywhere the provider wrote under `root`.
fn journal_body(
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
