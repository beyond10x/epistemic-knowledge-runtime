//! Attachment decoding preserves unique records and refuses duplicate decoded map keys.
use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AssertionId, EvidenceId, GraphRootId, RevisionNumber, SchemaVersionId, Timestamp};
use ekr_graph::{AttachedEvidence, CanonicalRef, GraphRoot, Space};
use ekr_store::GraphDocument;

const ASSERTION: &str = "aaaaaaaa-aaaa-7aaa-8aaa-aaaaaaaaaaaa";

fn empty_document() -> GraphDocument {
    GraphDocument {
        root: GraphRoot {
            id: GraphRootId::mint(),
            space: Space::Canonical,
            schema_version_id: SchemaVersionId::mint(),
            parent: None,
            created_at: Timestamp::EPOCH,
        },
        revision: RevisionNumber::new(2),
        nodes: BTreeMap::new(),
        edges: BTreeMap::new(),
        assertions: BTreeMap::new(),
        evidence: BTreeMap::new(),
        attachments: BTreeMap::new(),
    }
}

fn with_attachments(document: &GraphDocument, attachments: &str) -> Vec<u8> {
    let mut wire = serde_json::to_value(document).unwrap();
    wire["graph"]["attachments"] = serde_json::json!({});
    serde_json::to_string(&wire)
        .unwrap()
        .replace(
            "\"attachments\":{}",
            &format!("\"attachments\":{attachments}"),
        )
        .into_bytes()
}

#[test]
fn unique_attachment_records_round_trip_without_changing_the_wire_shape() {
    let mut document = empty_document();
    let records = BTreeSet::from([
        AttachedEvidence {
            evidence: CanonicalRef::new(EvidenceId::mint()),
            revision: RevisionNumber::new(1),
        },
        AttachedEvidence {
            evidence: CanonicalRef::new(EvidenceId::mint()),
            revision: RevisionNumber::new(2),
        },
    ]);
    document
        .attachments
        .insert(ASSERTION.parse().unwrap(), records.clone());
    let bytes = document.to_bytes().unwrap();
    let wire: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        wire["graph"]["attachments"][ASSERTION],
        serde_json::to_value(records).unwrap()
    );
    assert_eq!(GraphDocument::from_bytes(&bytes).unwrap(), document);
}

#[test]
fn omitted_and_empty_attachment_collections_still_decode() {
    let mut document = empty_document();
    let bytes = document.to_bytes().unwrap();
    assert!(
        !serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["graph"]
            .as_object()
            .unwrap()
            .contains_key("attachments")
    );
    assert_eq!(GraphDocument::from_bytes(&bytes).unwrap(), document);
    assert_eq!(
        GraphDocument::from_bytes(&with_attachments(&document, "{}")).unwrap(),
        document
    );
    let bytes = with_attachments(&document, &format!("{{\"{ASSERTION}\":[]}}"));
    document
        .attachments
        .insert(ASSERTION.parse().unwrap(), BTreeSet::new());
    assert_eq!(GraphDocument::from_bytes(&bytes).unwrap(), document);
}

#[test]
fn duplicate_decoded_assertion_keys_are_refused_before_their_second_value() {
    let document = empty_document();
    let escaped = ASSERTION.replacen('a', "\\u0061", 1);
    assert_eq!(
        serde_json::from_str::<AssertionId>(&format!("\"{escaped}\"")).unwrap(),
        ASSERTION.parse().unwrap()
    );
    for key in [ASSERTION, &escaped] {
        let bytes = with_attachments(
            &document,
            &format!("{{\"{ASSERTION}\":[],\"{key}\":false}}"),
        );
        let error = GraphDocument::from_bytes(&bytes).expect_err("duplicate assertion key");
        assert!(
            error.to_string().contains("duplicate decoded map key"),
            "{error}"
        );
    }
}
