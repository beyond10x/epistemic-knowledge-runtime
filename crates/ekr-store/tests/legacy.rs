//! Original bytes must not lose semantic information during verification.

#[test]
fn duplicate_keys_are_named_refusals_even_inside_user_records() {
    let bytes = br#"{"Record":{"format":1,"format":2}}"#;
    let result = ekr_store::legacy::decode_json::<serde_json::Value>(bytes);
    assert!(
        result.is_err(),
        "verification silently discarded a duplicate key"
    );
    assert!(result.unwrap_err().to_string().contains("duplicate-key"));
}

fn graph_vector() -> (Vec<u8>, ekr_core::ContentHash) {
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let corpus: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("tests/fixtures/legacy/vectors.json")).unwrap(),
    )
    .unwrap();
    let vector = corpus["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["kind"] == "graph-document")
        .unwrap();
    (
        vector["json"].as_str().unwrap().as_bytes().to_vec(),
        vector["payload_hash"].as_str().unwrap().parse().unwrap(),
    )
}

#[test]
fn original_document_fixture_verifies_exact_payload_bytes() {
    let (bytes, hash) = graph_vector();
    let document = ekr_store::legacy::GraphDocument::verify_bytes(&bytes, hash).unwrap();
    assert_eq!(document.nodes.len(), 1);
    assert_eq!(document.edges.len(), 1);
    assert_eq!(document.assertions.len(), 1);
    assert_eq!(serde_json::to_vec(&document).unwrap(), bytes);
    let mut changed = bytes;
    changed.push(b' ');
    assert!(matches!(
        ekr_store::legacy::GraphDocument::verify_bytes(&changed, hash),
        Err(ekr_store::legacy::Refusal::PayloadAddressMismatch { .. })
    ));
}

#[test]
fn original_documents_refuse_unknown_fields_and_misfiled_identities() {
    let (bytes, _) = graph_vector();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["new_state"] = true.into();
    let changed = serde_json::to_vec(&value).unwrap();
    assert!(ekr_store::legacy::GraphDocument::verify_bytes(
        &changed,
        ekr_core::ContentHash::of_bytes(&changed)
    )
    .is_err());
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    value["nodes"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap()["id"] = "00000000-0000-4000-8000-000000000099".into();
    let changed = serde_json::to_vec(&value).unwrap();
    assert!(
        matches!(ekr_store::legacy::GraphDocument::verify_bytes(&changed, ekr_core::ContentHash::of_bytes(&changed)), Err(ekr_store::legacy::Refusal::InvalidData(message)) if message == "misfiled-identity")
    );
}

#[test]
fn all_user_record_keys_are_preserved_when_unique() {
    let bytes = br#"{"event":"Seeded","format":"owned by user","operation":{"nested":1}}"#;
    let parsed: serde_json::Value = ekr_store::legacy::decode_json(bytes).unwrap();
    assert_eq!(parsed["event"], "Seeded");
    assert_eq!(parsed["format"], "owned by user");
    assert_eq!(parsed["operation"]["nested"], 1);
}
