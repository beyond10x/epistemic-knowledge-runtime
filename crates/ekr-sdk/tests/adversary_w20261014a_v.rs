//! Adversary case for `story:schema-transaction-cites-evidence`, SDK side.
//!
//! `docs/cli.md` § `ekr ontology` now documents `schema_version_evidence` in the document the
//! verb prints, and `ekr_sdk::read::Reader::ontology` is the SDK's typed read of that document.
//! A consumer of the SDK reads the schema through it, so the typed value must carry the citation:
//! read the documented document and write it back, and nothing may be lost.

use ekr_sdk::read::Ontology;
use serde_json::{json, Value};

#[test]
fn the_sdk_typed_ontology_read_keeps_the_evidence_a_version_cites() {
    let printed = json!({
        "revision": 2,
        "schema_version": "00000000-0000-4000-8000-000000000005",
        "schema_version_number": 1,
        "schema_version_parent": "00000000-0000-4000-8000-000000000001",
        "schema_version_evidence": ["00000000-0000-4000-8000-000000000411"],
        "node_types": [],
        "edge_types": []
    });
    let read: Ontology = serde_json::from_value(printed.clone()).unwrap();
    let written: Value = serde_json::to_value(&read).unwrap();
    assert_eq!(
        written, printed,
        "the SDK's typed `ekr ontology` read drops what the verb prints"
    );
}
