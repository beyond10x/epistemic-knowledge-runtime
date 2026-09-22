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
    let vector = vector("proposed-graph");
    (
        vector["json"].as_str().unwrap().as_bytes().to_vec(),
        vector["payload_hash"].as_str().unwrap().parse().unwrap(),
    )
}

fn vector(name: &str) -> serde_json::Value {
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let corpus: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root.join("tests/fixtures/legacy/vectors.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        corpus["source_commit"],
        "73ab8b0a5aa5c670bacbc4c1abdca02f87b62bf0"
    );
    corpus["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap()
        .clone()
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

#[test]
fn unique_map_refuses_decoded_duplicate_keys_and_preserves_unique_entries() {
    use std::collections::BTreeMap;

    for raw in [
        r#"{"runtime":1,"runtime":2}"#,
        r#"{"runtime":1,"run\u0074ime":2}"#,
    ] {
        let mut decoder = serde_json::Deserializer::from_str(raw);
        let result: Result<BTreeMap<String, u32>, _> = ekr_graph::legacy::unique_map(&mut decoder);
        assert!(result.unwrap_err().to_string().contains("duplicate-key"));
    }
    let mut decoder = serde_json::Deserializer::from_str(r#"{"z":2,"a":1}"#);
    let observed: BTreeMap<String, u32> = ekr_graph::legacy::unique_map(&mut decoder).unwrap();
    assert_eq!(observed, BTreeMap::from([("a".into(), 1), ("z".into(), 2)]));
    decoder.end().unwrap();
    let mut decoder = serde_json::Deserializer::from_str("{}");
    let empty: BTreeMap<String, u32> = ekr_graph::legacy::unique_map(&mut decoder).unwrap();
    assert!(empty.is_empty());
}

#[test]
fn unique_set_refuses_decoded_duplicate_members_and_preserves_unique_entries() {
    use std::collections::BTreeSet;

    for raw in [r#"["runtime","runtime"]"#, r#"["runtime","run\u0074ime"]"#] {
        let mut decoder = serde_json::Deserializer::from_str(raw);
        let result: Result<BTreeSet<String>, _> = ekr_graph::legacy::unique_set(&mut decoder);
        assert!(result.unwrap_err().to_string().contains("duplicate-member"));
    }
    let mut decoder = serde_json::Deserializer::from_str(r#"["z","a"]"#);
    let observed: BTreeSet<String> = ekr_graph::legacy::unique_set(&mut decoder).unwrap();
    assert_eq!(observed, BTreeSet::from(["a".into(), "z".into()]));
    decoder.end().unwrap();
    let mut decoder = serde_json::Deserializer::from_str("[]");
    let empty: BTreeSet<String> = ekr_graph::legacy::unique_set(&mut decoder).unwrap();
    assert!(empty.is_empty());
}

#[test]
fn direct_address_verifiers_distinguish_exact_payloads_from_frozen_values() {
    use ekr_core::canonical::Canonical;
    use ekr_core::ContentHash;
    use ekr_store::legacy::Refusal;

    let captured = vector("scalar-node");
    let bytes = captured["json"].as_str().unwrap().as_bytes();
    let payload_hash: ContentHash = captured["payload_hash"].as_str().unwrap().parse().unwrap();
    let value_hash: ContentHash = captured["value_hash"].as_str().unwrap().parse().unwrap();
    let node: ekr_graph::legacy::Node = ekr_store::legacy::decode_json(bytes).unwrap();

    assert_eq!(
        ekr_store::legacy::verify_payload(bytes, payload_hash),
        Ok(())
    );
    assert_eq!(ekr_store::legacy::verify_value(&node, value_hash), Ok(()));
    assert_eq!(
        ekr_store::legacy::verify_payload(bytes, value_hash),
        Err(Refusal::PayloadAddressMismatch {
            expected: value_hash,
            observed: payload_hash
        })
    );
    assert_eq!(
        ekr_store::legacy::verify_value(&node, payload_hash),
        Err(Refusal::ValueAddressMismatch {
            expected: payload_hash,
            observed: value_hash
        })
    );

    // The identical canonical byte sequence still has two different domain addresses.
    let encoded_payload_hash = ContentHash::of_bytes(&node.canonical_bytes());
    assert_eq!(
        ekr_store::legacy::verify_value(&node, encoded_payload_hash),
        Err(Refusal::ValueAddressMismatch {
            expected: encoded_payload_hash,
            observed: value_hash
        })
    );

    let mut reformatted = bytes.to_vec();
    reformatted.push(b' ');
    assert_eq!(
        ekr_store::legacy::verify_payload(&reformatted, payload_hash),
        Err(Refusal::PayloadAddressMismatch {
            expected: payload_hash,
            observed: ContentHash::of_bytes(&reformatted),
        })
    );
    let same_node: ekr_graph::legacy::Node = ekr_store::legacy::decode_json(&reformatted).unwrap();
    assert_eq!(node, same_node);
    assert_eq!(
        ekr_store::legacy::verify_value(&same_node, value_hash),
        Ok(())
    );
}

#[test]
fn direct_identity_check_refuses_each_misfiled_collection() {
    use ekr_store::legacy::{GraphDocument, Refusal};

    let (bytes, _) = graph_vector();
    let source: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let document: GraphDocument = ekr_store::legacy::decode_json(&bytes).unwrap();
    assert_eq!(document.check_identities(), Ok(()));
    for collection in ["nodes", "edges", "assertions", "evidence"] {
        let mut changed = source.clone();
        changed[collection]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap()["id"] = "00000000-0000-4000-8000-000000000099".into();
        // Decode without verify_bytes so this exercises the direct identity boundary.
        let misfiled: GraphDocument =
            ekr_store::legacy::decode_json(&serde_json::to_vec(&changed).unwrap()).unwrap();
        assert_eq!(
            misfiled.check_identities(),
            Err(Refusal::InvalidData("misfiled-identity".into())),
            "{collection} must compare the map key with the contained identity"
        );
    }
}

#[test]
fn knowledge_bytes_keep_original_map_framing_and_captured_record_order() {
    use std::collections::BTreeMap;

    let (bytes, _) = graph_vector();
    let mut document: ekr_store::legacy::GraphDocument =
        ekr_store::legacy::decode_json(&bytes).unwrap();
    let node_vector = vector("scalar-node");
    let edge_vector = vector("scalar-edge");
    let assertion_vector = vector("assertion-state-0");
    let node: ekr_graph::legacy::Node =
        ekr_store::legacy::decode_json(node_vector["json"].as_str().unwrap().as_bytes()).unwrap();
    let edge: ekr_graph::legacy::Edge =
        ekr_store::legacy::decode_json(edge_vector["json"].as_str().unwrap().as_bytes()).unwrap();
    let assertion: ekr_graph::legacy::Assertion =
        ekr_store::legacy::decode_json(assertion_vector["json"].as_str().unwrap().as_bytes())
            .unwrap();
    document.nodes = BTreeMap::from([(node.id, node)]);
    document.edges = BTreeMap::from([(edge.id, edge)]);
    document.assertions = BTreeMap::from([(assertion.id, assertion)]);

    // Original map tag/count and fixed identity keys, followed by captured 73ab8b0 bytes.
    // No current or frozen encoder manufactures these expected record bytes.
    let expected = format!(
        "0a00000000000000010d0000000000004000800000000000000a{}\
         0a00000000000000010d0000000000004000800000000000000b{}\
         0a00000000000000010d0000000000004000800000000000000d{}",
        node_vector["canonical_hex"].as_str().unwrap(),
        edge_vector["canonical_hex"].as_str().unwrap(),
        assertion_vector["canonical_hex"].as_str().unwrap(),
    );
    let actual: String = document
        .knowledge_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(actual, expected);

    // Root/revision and evidence metadata are separate from the original knowledge sub-root.
    document.evidence.clear();
    document.revision = ekr_core::RevisionNumber::new(27);
    document.root.created_at = ekr_core::Timestamp::from_millis(100);
    let unchanged: String = document
        .knowledge_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(unchanged, expected);

    document.nodes.clear();
    document.edges.clear();
    document.assertions.clear();
    let empty: String = document
        .knowledge_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(
        empty,
        "0a00000000000000000a00000000000000000a0000000000000000"
    );
}
