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
//! `ekr-store` declares no YAML parser, so the document is parsed by the subset parser at the end
//! of this file, which fails on any line it does not consume, and each scan is held to its own
//! catch: a parse that stops finding declarations fails loudly rather than passing vacuously.

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
    let document = parse_yaml(&domain_text());
    let declared = document
        .get("types")
        .map_or(&[][..], Yaml::items)
        .iter()
        .find(|declared| declared.get("name").and_then(Yaml::as_str) == Some(type_name))
        .unwrap_or_else(|| panic!("the domain declares no type named {type_name}"));
    let variants: Vec<String> = declared
        .get("variants")
        .map_or(&[][..], Yaml::items)
        .iter()
        .map(|variant| variant.as_str().expect("a variant is a name").to_owned())
        .collect();
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
    let document = parse_yaml(&domain_text());
    let mut declared: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for event in document.get("events").map_or(&[][..], Yaml::items) {
        let name = event
            .get("name")
            .and_then(Yaml::as_str)
            .unwrap_or_else(|| panic!("an event has no name: {event:?}"));
        let fields = event
            .get("fields")
            .map_or(&[][..], Yaml::items)
            .iter()
            .map(|field| {
                field
                    .get("name")
                    .and_then(Yaml::as_str)
                    .unwrap_or_else(|| panic!("a field of {name} has no name: {field:?}"))
                    .to_owned()
            })
            .collect();
        assert!(
            declared.insert(name.to_owned(), fields).is_none(),
            "{name} is declared twice"
        );
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

// Every `types:` and `entities:` declaration of the domain, against the Rust type that carries it.
//
// `ekr.store.Snapshot` and `ekr.store.SnapshotId` were declared here and implemented nowhere for
// three waves, and it took an implementor's report to say so: the cases above hold events and
// `StorageClass`, and nothing held the rest. This is `crates/ekr-graph/tests/domain_projection.rs`'s
// PROJECTIONS applied to this domain, member for member, so a declaration nothing implements — or
// a field one side has and the other does not — is red rather than invisible.
//
// Adversary pass 1 of wave p1-14 found the first version of this blind twice over: it asked only
// that a type of the carrier's name exist, and it found a declaration only when `name:` was the
// first key of its mapping. Both were reading the document by line shape. It is now parsed, by the
// subset parser at the end of this file, and a key is looked up wherever it sits in its mapping.

/// How the crate carries one declaration.
enum Carrier {
    /// One Rust type holds the declaration member for member: a struct's fields (with an entity's
    /// identity field), or an enum's variants.
    Whole(&'static str),
    /// The declaration's variants are spread over several Rust types. Each part names the variants
    /// it holds, and whether they are *all* of that type's variants.
    Split(&'static [(&'static str, &'static [&'static str], bool)]),
    /// A one-variant enumeration carried as a string constant, `(type, constant)`.
    Constant(&'static str, &'static str),
}

/// Each declaration under `types:` or `entities:`, and how the crate carries it.
///
/// Transcribed, not derived: a binding read from the thing it binds asserts nothing. Both halves of
/// each comparison are derived, so a declaration added without a row, a member added on one side,
/// or a carrier renamed is red.
const BINDINGS: &[(&str, Carrier)] = &[
    ("ekr.store.StorageClass", Carrier::Whole("StorageClass")),
    (
        "ekr.store.PublicationCommandKind",
        Carrier::Whole("PublicationCommandKind"),
    ),
    (
        "ekr.store.PublicationCommandKey",
        Carrier::Whole("PublicationCommandKey"),
    ),
    (
        "ekr.store.PublicationObject",
        Carrier::Whole("PublicationObject"),
    ),
    ("ekr.store.Publication", Carrier::Whole("Publication")),
    (
        "ekr.store.NativeExpectedKind",
        Carrier::Whole("NativeExpectedKind"),
    ),
    ("ekr.store.NativeExpected", Carrier::Whole("NativeExpected")),
    ("ekr.store.NativeStreamId", Carrier::Whole("NativeStreamId")),
    ("ekr.store.NativeNewEvent", Carrier::Whole("NativeNewEvent")),
    (
        "ekr.store.NativeStreamAppend",
        Carrier::Whole("NativeStreamAppend"),
    ),
    ("ekr.store.NativeClaim", Carrier::Whole("NativeClaim")),
    (
        "ekr.store.NativeCommandMeta",
        Carrier::Whole("NativeCommandMeta"),
    ),
    (
        "ekr.store.NativeBlobWrite",
        Carrier::Whole("NativeBlobWrite"),
    ),
    (
        "ekr.store.NativePublicationRequest",
        Carrier::Whole("NativePublicationRequest"),
    ),
    (
        "ekr.store.PublicationPreparationFormatV1",
        Carrier::Constant("PublicationPreparationV1", "FORMAT"),
    ),
    (
        "ekr.store.PublicationPreparationV1",
        Carrier::Whole("PublicationPreparationV1"),
    ),
    (
        "ekr.store.PublicationResolution",
        Carrier::Split(&[
            ("Appended", &["Written", "AlreadyRecorded"], true),
            ("StoreError", &["Conflict", "UnknownCommit"], false),
        ]),
    ),
    ("ekr.store.StoredObject", Carrier::Whole("StoredObject")),
];

/// Every declaration under the top-level `types:` and `entities:` keys of `document`, as
/// `(name, members)`: an enumeration's variants, or a struct's or entity's field names with an
/// entity's identity field among them.
///
/// Takes the parsed document rather than reading it so the case can be shown red against a copy of
/// the document, without editing `systems/`.
fn declared_types_and_entities(document: &Yaml) -> BTreeMap<String, BTreeSet<String>> {
    let mut found = BTreeMap::new();
    for section in ["types", "entities"] {
        for declaration in document.get(section).map_or(&[][..], Yaml::items) {
            let name = declaration
                .get("name")
                .and_then(Yaml::as_str)
                .unwrap_or_else(|| panic!("a {section} entry has no name: {declaration:?}"));
            let mut members: BTreeSet<String> = declaration
                .get("variants")
                .map_or(&[][..], Yaml::items)
                .iter()
                .map(|variant| variant.as_str().expect("a variant is a name").to_owned())
                .collect();
            for field in declaration.get("fields").map_or(&[][..], Yaml::items) {
                members.insert(
                    field
                        .get("name")
                        .and_then(Yaml::as_str)
                        .unwrap_or_else(|| panic!("a field of {name} has no name: {field:?}"))
                        .to_owned(),
                );
            }
            if let Some(identity) = declaration.get("identity") {
                members.insert(
                    identity
                        .get("name")
                        .and_then(Yaml::as_str)
                        .unwrap_or_else(|| panic!("the identity of {name} has no name"))
                        .to_owned(),
                );
            }
            assert!(
                found.insert(name.to_owned(), members).is_none(),
                "{name} is declared twice"
            );
        }
    }
    assert!(
        found.len() >= 10,
        "the declaration scan is broken, not the domain: {found:?}"
    );
    found
}

/// Every current module of this crate's `src/`, as raw text beside the same text with comments and
/// literals blanked (see [`code_only`]), excluding the frozen legacy codec.
fn crate_sources() -> Vec<(String, String)> {
    let directory = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the runtime manifest directory"),
    )
    .join("src");
    let mut paths: Vec<std::path::PathBuf> = std::fs::read_dir(directory)
        .expect("the crate has a src/")
        .map(|entry| entry.expect("a directory entry").path())
        .filter(|path| {
            path.extension().is_some_and(|e| e == "rs")
                && path.file_name().is_some_and(|file| file != "legacy.rs")
        })
        .collect();
    paths.sort();
    assert!(paths.len() >= 5, "the module scan is broken: {paths:?}");
    paths
        .into_iter()
        .map(|path| {
            let raw = std::fs::read_to_string(&path).expect("a source file");
            let code = code_only(&raw);
            (raw, code)
        })
        .collect()
}

/// `text` with every comment and the contents of every string and character literal replaced by
/// spaces of the same byte length, newlines kept, so a brace or comma inside one is not read as
/// structure and a byte offset into the result is one into `text`.
fn code_only(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let blank = |out: &mut String, c: char| {
        if c == '\n' {
            out.push('\n');
        } else {
            out.extend(std::iter::repeat_n(' ', c.len_utf8()));
        }
    };
    let mut at = 0;
    while at < chars.len() {
        let c = chars[at];
        let next = chars.get(at + 1).copied();
        if c == '/' && next == Some('/') {
            while at < chars.len() && chars[at] != '\n' {
                blank(&mut out, chars[at]);
                at += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 0usize;
            while at < chars.len() {
                if chars[at] == '/' && chars.get(at + 1) == Some(&'*') {
                    depth += 1;
                    blank(&mut out, '/');
                    blank(&mut out, '*');
                    at += 2;
                } else if chars[at] == '*' && chars.get(at + 1) == Some(&'/') {
                    depth -= 1;
                    blank(&mut out, '*');
                    blank(&mut out, '/');
                    at += 2;
                    if depth == 0 {
                        break;
                    }
                } else {
                    blank(&mut out, chars[at]);
                    at += 1;
                }
            }
        } else if c == 'r'
            && (next == Some('"') || next == Some('#'))
            && !chars
                .get(at.wrapping_sub(1))
                .is_some_and(|p| p.is_alphanumeric() || *p == '_')
        {
            // A raw string: `r"…"` or `r#"…"#`, closed by a quote and as many hashes.
            let mut hashes = 0;
            let mut probe = at + 1;
            while chars.get(probe) == Some(&'#') {
                hashes += 1;
                probe += 1;
            }
            if chars.get(probe) != Some(&'"') {
                out.push(c);
                at += 1;
                continue;
            }
            for &k in &chars[at..=probe] {
                out.push(k);
            }
            at = probe + 1;
            while at < chars.len() {
                if chars[at] == '"' && (1..=hashes).all(|h| chars.get(at + h) == Some(&'#')) {
                    out.push('"');
                    out.extend(std::iter::repeat_n('#', hashes));
                    at += 1 + hashes;
                    break;
                }
                blank(&mut out, chars[at]);
                at += 1;
            }
        } else if c == '"' {
            out.push('"');
            at += 1;
            while at < chars.len() && chars[at] != '"' {
                if chars[at] == '\\' {
                    blank(&mut out, '\\');
                    at += 1;
                }
                if at < chars.len() {
                    blank(&mut out, chars[at]);
                    at += 1;
                }
            }
            if at < chars.len() {
                out.push('"');
                at += 1;
            }
        } else if c == '\'' {
            // A character literal is `'x'` or `'\…'`; anything else is a lifetime.
            let close = if next == Some('\\') {
                (at + 2..chars.len().min(at + 12)).find(|&k| chars[k] == '\'')
            } else if chars.get(at + 2) == Some(&'\'') {
                Some(at + 2)
            } else {
                None
            };
            if let Some(close) = close {
                out.push('\'');
                for &k in &chars[at + 1..close] {
                    blank(&mut out, k);
                }
                out.push('\'');
                at = close + 1;
            } else {
                out.push(c);
                at += 1;
            }
        } else {
            out.push(c);
            at += 1;
        }
    }
    assert_eq!(out.len(), text.len(), "blanking preserves byte offsets");
    out
}

/// The byte range of the braced block that follows the first line of `code` for which `head`
/// holds, from its `{` through its matching `}`.
fn block_after(code: &str, head: impl Fn(&str) -> bool) -> Option<std::ops::Range<usize>> {
    let mut offset = 0;
    for line in code.split_inclusive('\n') {
        if head(line.trim()) {
            let open = offset + code[offset..].find('{')?;
            let mut depth = 0usize;
            for (at, byte) in code.bytes().enumerate().skip(open) {
                match byte {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(open..at + 1);
                        }
                    }
                    _ => {}
                }
            }
            panic!("an item's braces do not balance");
        }
        offset += line.len();
    }
    None
}

/// Whether `line` opens `pub struct name` or `pub enum name`, and not a longer name.
fn opens_type(line: &str, name: &str) -> bool {
    ["pub struct ", "pub enum "].iter().any(|keyword| {
        line.strip_prefix(keyword)
            .and_then(|rest| rest.strip_prefix(name))
            .is_some_and(|rest| !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
    })
}

/// The member names of `pub struct name` (its fields) or `pub enum name` (its variants), read from
/// the item's own braces with comments, literals and attributes set aside.
///
/// Members are the body's top-level comma-separated entries, where "top level" counts `()`, `[]`,
/// `{}` and `<>` (a `->` is not a bracket), so a generic `BTreeMap<K, V>` is one entry.
fn rust_members(name: &str) -> Option<BTreeSet<String>> {
    for (_, code) in crate_sources() {
        let Some(block) = block_after(&code, |line| opens_type(line, name)) else {
            continue;
        };
        let body = &code[block.start + 1..block.end - 1];
        let mut entries = vec![String::new()];
        let mut depth = 0usize;
        let mut previous = ' ';
        for c in body.chars() {
            match c {
                '(' | '[' | '{' | '<' => depth += 1,
                ')' | ']' | '}' => depth -= 1,
                '>' if previous != '-' => depth -= 1,
                ',' if depth == 0 => {
                    entries.push(String::new());
                    previous = c;
                    continue;
                }
                _ => {}
            }
            entries.last_mut().expect("one entry is open").push(c);
            previous = c;
        }
        let is_struct = code[..block.start]
            .rsplit('\n')
            .next()
            .is_some_and(|line| line.trim_start().starts_with("pub struct"));
        let mut members = BTreeSet::new();
        for entry in entries {
            let mut entry = entry.trim();
            // Attributes before the member: `#[…]`, balanced.
            while let Some(rest) = entry.strip_prefix("#[") {
                let mut depth = 1usize;
                let end = rest
                    .char_indices()
                    .find(|&(_, c)| {
                        match c {
                            '[' => depth += 1,
                            ']' => depth -= 1,
                            _ => {}
                        }
                        depth == 0
                    })
                    .map(|(at, _)| at)
                    .expect("an attribute closes");
                entry = rest[end + 1..].trim_start();
            }
            let entry = entry
                .strip_prefix("pub(crate) ")
                .or_else(|| entry.strip_prefix("pub "))
                .unwrap_or(entry);
            let ident: String = entry
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if ident.is_empty() {
                assert!(
                    entry.is_empty(),
                    "{name} has a member this scan cannot name: {entry:?}"
                );
                continue;
            }
            if is_struct {
                assert!(
                    entry[ident.len()..].trim_start().starts_with(':'),
                    "{name} has a field this scan cannot read: {entry:?}"
                );
            }
            members.insert(ident);
        }
        return Some(members);
    }
    None
}

/// The value of `pub const constant: &'static str = "…";` inside `impl owner { … }`.
fn string_constant(owner: &str, constant: &str) -> Option<String> {
    for (raw, code) in crate_sources() {
        let Some(block) = block_after(&code, |line| line == format!("impl {owner} {{")) else {
            continue;
        };
        let head = format!("pub const {constant}: &'static str = \"");
        let body = &raw[block];
        let at = body.find(&head)? + head.len();
        return Some(body[at..at + body[at..].find('"')?].to_owned());
    }
    None
}

/// Every `types:` and `entities:` declaration of `store.yaml` is carried member for member by the
/// crate, and every binding names a declaration that still exists.
#[test]
fn every_type_and_entity_the_domain_declares_names_a_rust_carrier() {
    let declared = declared_types_and_entities(&parse_yaml(&domain_text()));
    let bound: BTreeSet<String> = BINDINGS
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();
    let names: BTreeSet<String> = declared.keys().cloned().collect();

    let unbound: Vec<&String> = names.difference(&bound).collect();
    assert!(
        unbound.is_empty(),
        "the store domain declares types no Rust type carries: {unbound:?}"
    );
    let stale: Vec<&String> = bound.difference(&names).collect();
    assert!(
        stale.is_empty(),
        "bindings name declarations the store domain no longer has: {stale:?}"
    );

    for (declaration, carrier) in BINDINGS {
        let domain = &declared[*declaration];
        match carrier {
            Carrier::Whole(rust_type) => {
                let rust = rust_members(rust_type).unwrap_or_else(|| {
                    panic!(
                        "{declaration} is bound to {rust_type}, which this crate does not declare"
                    )
                });
                assert_eq!(
                    domain, &rust,
                    "{declaration} (left) and {rust_type} (right) disagree member for member"
                );
            }
            Carrier::Split(parts) => {
                let mut covered = BTreeSet::new();
                for (rust_type, held, whole) in *parts {
                    let rust = rust_members(rust_type).unwrap_or_else(|| {
                        panic!("{declaration} is bound to {rust_type}, which this crate does not declare")
                    });
                    let held: BTreeSet<String> = held.iter().map(|m| (*m).to_owned()).collect();
                    assert!(
                        held.is_subset(&rust),
                        "{declaration} is carried in part by {rust_type}, which lacks {:?}",
                        held.difference(&rust).collect::<Vec<_>>()
                    );
                    if *whole {
                        assert_eq!(
                            held, rust,
                            "{rust_type} carries only {declaration}'s variants"
                        );
                    }
                    covered.extend(held);
                }
                assert_eq!(
                    domain, &covered,
                    "{declaration} (left) and its carriers (right) disagree member for member"
                );
            }
            Carrier::Constant(owner, constant) => {
                let value = string_constant(owner, constant).unwrap_or_else(|| {
                    panic!("{declaration} is bound to {owner}::{constant}, which this crate does not declare")
                });
                assert_eq!(
                    domain,
                    &BTreeSet::from([value]),
                    "{declaration} and {owner}::{constant} disagree"
                );
            }
        }
    }
}

/// A node of the YAML subset the ESS domains are written in.
///
/// `ekr-store` has no YAML parser among its dependencies, and reading the document by the shape of
/// its lines is what adversary pass 1 found blind: `- kind: struct` followed by `name: …` is the
/// same mapping as the other order, and a line scan sees only one of them. This reads block
/// mappings, block sequences, flow sequences of plain scalars and folded or literal block
/// scalars. Anything else — a line it does not consume — fails the parse rather than being
/// skipped. A flow mapping (`{generated: true}`) is kept as one scalar.
#[derive(Clone, Debug, PartialEq)]
enum Yaml {
    Scalar(String),
    Seq(Vec<Yaml>),
    Map(Vec<(String, Yaml)>),
}

impl Yaml {
    /// The value under `key`, wherever it sits in this mapping.
    fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Map(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::Scalar(text) => Some(text),
            _ => None,
        }
    }

    /// A sequence's items, or none for anything else.
    fn items(&self) -> &[Self] {
        match self {
            Self::Seq(items) => items,
            _ => &[],
        }
    }
}

/// `text` parsed as the YAML subset [`Yaml`] describes.
fn parse_yaml(text: &str) -> Yaml {
    let mut lines: Vec<(usize, String)> = text
        .lines()
        .filter_map(|line| {
            let content = match line.find(" #") {
                _ if line.trim_start().starts_with('#') => "",
                Some(at) => &line[..at],
                None => line,
            }
            .trim_end();
            let body = content.trim_start();
            (!body.is_empty()).then(|| (content.len() - body.len(), body.to_owned()))
        })
        .collect();
    let mut at = 0;
    let document = parse_node(&mut lines, &mut at);
    assert_eq!(
        at,
        lines.len(),
        "the YAML subset parser stopped at {:?}",
        lines.get(at)
    );
    document
}

fn is_item(body: &str) -> bool {
    body == "-" || body.starts_with("- ")
}

/// `(key, value)` when `body` is a `key: value` or `key:` line.
fn key_of(body: &str) -> Option<(&str, &str)> {
    if body.starts_with(['[', '"', '\'', '{']) {
        return None;
    }
    let (key, value) = body.split_once(':')?;
    (value.is_empty() || value.starts_with(' ')).then(|| (key.trim(), value.trim()))
}

fn scalar(value: &str) -> Yaml {
    let unquote = |v: &str| v.trim().trim_matches(['"', '\'']).to_owned();
    match value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) {
        Some(inner) => Yaml::Seq(
            inner
                .split(',')
                .filter(|v| !v.trim().is_empty())
                .map(|v| Yaml::Scalar(unquote(v)))
                .collect(),
        ),
        None => Yaml::Scalar(unquote(value)),
    }
}

/// The node starting at `lines[*at]`, at that line's indent.
fn parse_node(lines: &mut [(usize, String)], at: &mut usize) -> Yaml {
    let (indent, body) = lines[*at].clone();
    if is_item(&body) {
        let mut items = Vec::new();
        while *at < lines.len() && lines[*at].0 == indent && is_item(&lines[*at].1) {
            let rest = lines[*at].1[1..].trim_start().to_owned();
            if rest.is_empty() {
                *at += 1;
                items.push(if *at < lines.len() && lines[*at].0 > indent {
                    parse_node(lines, at)
                } else {
                    Yaml::Scalar(String::new())
                });
            } else {
                // The item's content, re-read as a node at the column it starts in.
                let column = indent + (lines[*at].1.len() - rest.len());
                lines[*at] = (column, rest);
                items.push(parse_node(lines, at));
            }
        }
        return Yaml::Seq(items);
    }
    if key_of(&body).is_none() {
        *at += 1;
        return scalar(&body);
    }
    let mut entries = Vec::new();
    while *at < lines.len() && lines[*at].0 == indent && !is_item(&lines[*at].1) {
        let Some((key, value)) = key_of(&lines[*at].1).map(|(k, v)| (k.to_owned(), v.to_owned()))
        else {
            break;
        };
        *at += 1;
        let deeper = |lines: &[(usize, String)], at: usize| {
            at < lines.len()
                && (lines[at].0 > indent || (lines[at].0 == indent && is_item(&lines[at].1)))
        };
        let node = if value.is_empty() {
            if deeper(lines, *at) {
                parse_node(lines, at)
            } else {
                Yaml::Scalar(String::new())
            }
        } else if matches!(value.as_str(), ">" | ">-" | ">+" | "|" | "|-" | "|+") {
            let mut parts = Vec::new();
            while *at < lines.len() && lines[*at].0 > indent {
                parts.push(lines[*at].1.clone());
                *at += 1;
            }
            Yaml::Scalar(parts.join(" "))
        } else {
            scalar(&value)
        };
        entries.push((key, node));
    }
    Yaml::Map(entries)
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
