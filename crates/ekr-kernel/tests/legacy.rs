//! Fixed original-format vectors, captured before any current production codec changed.
use std::path::PathBuf;

use ekr_core::canonical::Canonical;
use ekr_core::{ContentHash, RevisionNumber};
use ekr_graph::legacy as graph;
use ekr_kernel::legacy as kernel;
use ekr_store::legacy::{decode_json, verify_payload, verify_value, Refusal};
use serde::de::DeserializeOwned;
use serde::Deserialize;

#[derive(Deserialize)]
struct Corpus {
    source_commit: String,
    vectors: Vec<Vector>,
    validation: Validation,
}
#[derive(Deserialize)]
struct Vector {
    kind: String,
    name: String,
    json: String,
    canonical_hex: Option<String>,
    value_hash: Option<ContentHash>,
    payload_hash: ContentHash,
    knowledge_root: Option<ContentHash>,
    evidence_root: Option<ContentHash>,
}
#[derive(Deserialize)]
struct Validation {
    against: RevisionNumber,
    canonical_hex: String,
    validation_hash: ContentHash,
}
fn corpus() -> Corpus {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    serde_json::from_slice(
        &std::fs::read(root.join("../ekr-store/tests/fixtures/legacy/vectors.json")).unwrap(),
    )
    .unwrap()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn frozen<T: DeserializeOwned + Canonical>(vector: &Vector) -> T {
    verify_payload(vector.json.as_bytes(), vector.payload_hash).unwrap();
    let value: T = decode_json(vector.json.as_bytes()).unwrap();
    assert_eq!(
        hex(&value.canonical_bytes()),
        *vector.canonical_hex.as_ref().unwrap(),
        "{} bytes",
        vector.name
    );
    verify_value(&value, vector.value_hash.unwrap()).unwrap();
    value
}
fn selected(kind: &str) -> Vec<Vector> {
    corpus()
        .vectors
        .into_iter()
        .filter(|v| v.kind == kind)
        .collect()
}

#[test]
fn original_value_scalar_node_and_edge_bytes_are_fixed() {
    let corpus = corpus();
    assert_eq!(corpus.source_commit, graph::SOURCE_COMMIT);
    let mut count = 0;
    for vector in &corpus.vectors {
        match vector.kind.as_str() {
            "value" => {
                frozen::<graph::Value>(vector);
                count += 1;
            }
            "node" => {
                frozen::<graph::Node>(vector);
                count += 1;
            }
            "edge" => {
                frozen::<graph::Edge>(vector);
                count += 1;
            }
            _ => {}
        }
    }
    assert_eq!(count, 14);
}

#[test]
fn every_original_combined_assertion_state_and_evidence_source_is_fixed() {
    let assertions = selected("assertion");
    assert_eq!(assertions.len(), 10);
    for vector in &assertions {
        frozen::<graph::Assertion>(vector);
    }
    let evidence = selected("evidence");
    assert_eq!(evidence.len(), 6);
    for vector in &evidence {
        frozen::<graph::Evidence>(vector);
    }
}

#[test]
fn all_six_original_event_discriminants_and_fields_are_fixed() {
    let events = selected("event");
    assert_eq!(events.len(), 6);
    for (index, vector) in events.iter().enumerate() {
        let event = frozen::<graph::RevisionEvent>(vector);
        assert_eq!(event.variant_index(), index as u32);
        assert!(vector.json.contains("\"event\":"));
    }
}

#[test]
fn original_revision_root_fields_are_fixed() {
    let roots = selected("revision-root");
    assert_eq!(roots.len(), 1);
    frozen::<graph::Root>(&roots[0]);
}

#[test]
fn all_original_operations_include_transitive_ontology_and_assertion_bytes() {
    let operations = selected("operation");
    assert_eq!(operations.len(), 11);
    for vector in &operations {
        frozen::<kernel::GraphOperation>(vector);
    }
}

#[test]
fn original_transaction_validation_uses_payload_domain_and_ordered_operations() {
    let corpus = corpus();
    let vector = corpus
        .vectors
        .iter()
        .find(|v| v.kind == "transaction")
        .unwrap();
    let mut transaction = frozen::<kernel::GraphTransaction>(vector);
    let claim = &corpus.validation;
    assert_eq!(
        hex(&transaction.validation_bytes(claim.against)),
        claim.canonical_hex
    );
    assert_eq!(
        transaction.validation_address(claim.against),
        claim.validation_hash
    );
    kernel::GraphTransaction::verify_bytes(
        vector.json.as_bytes(),
        vector.payload_hash,
        claim.against,
        claim.validation_hash,
    )
    .unwrap();
    assert_ne!(
        transaction.validation_address(RevisionNumber::new(8)),
        claim.validation_hash
    );
    transaction.operations.reverse();
    assert_ne!(
        transaction.validation_address(claim.against),
        claim.validation_hash
    );
}

#[test]
fn seed_envelope_reproduces_actual_original_bootstrap_attribution() {
    let corpus = corpus();
    let input_vector = corpus.vectors.iter().find(|v| v.kind == "seed").unwrap();
    let seed =
        kernel::SeedDocument::verify_bytes(input_vector.json.as_bytes(), input_vector.payload_hash)
            .unwrap();
    assert!(seed
        .graph
        .assertions
        .values()
        .all(|a| a.validation == graph::ValidationState::Proposed));
    let vector = corpus
        .vectors
        .iter()
        .find(|v| v.kind == "seed-envelope")
        .unwrap();
    let envelope =
        kernel::SeedEnvelope::verify_bytes(vector.json.as_bytes(), vector.payload_hash).unwrap();
    assert_eq!(envelope.input, seed);
    let attributed = envelope.attributed_document().unwrap();
    assert_eq!(
        attributed.knowledge_address(),
        vector.knowledge_root.unwrap()
    );
    assert_eq!(attributed.evidence_address(), vector.evidence_root.unwrap());
    let accepted = corpus
        .vectors
        .iter()
        .find(|v| v.name == "bootstrap-accepted")
        .unwrap();
    assert_eq!(
        attributed.assertions.values().next().unwrap(),
        &frozen::<graph::Assertion>(accepted)
    );
    assert_ne!(
        seed.graph.knowledge_address(),
        attributed.knowledge_address()
    );
}

fn envelope_json() -> serde_json::Value {
    let vector = selected("seed-envelope").pop().unwrap();
    serde_json::from_str(&vector.json).unwrap()
}
fn verify_envelope(value: &serde_json::Value) -> Result<kernel::SeedEnvelope, Refusal> {
    let bytes = serde_json::to_vec(value).unwrap();
    kernel::SeedEnvelope::verify_bytes(&bytes, ContentHash::of_bytes(&bytes))
}

#[test]
fn missing_and_corrupt_original_evidence_are_named_refusals() {
    let mut value = envelope_json();
    value["input"]["evidence_payloads"] = serde_json::json!({});
    assert!(matches!(
        verify_envelope(&value),
        Err(Refusal::MissingEvidence { .. })
    ));
    let mut value = envelope_json();
    let payload = value["input"]["evidence_payloads"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap();
    *payload = serde_json::json!([0]);
    assert!(matches!(
        verify_envelope(&value),
        Err(Refusal::PayloadAddressMismatch { .. })
    ));
}

#[test]
fn original_formats_unknown_fields_and_duplicate_semantic_identities_refuse() {
    let mut value = envelope_json();
    value["format"] = "ekr-seed-envelope/2".into();
    assert!(matches!(
        verify_envelope(&value),
        Err(Refusal::UnsupportedFormat(_))
    ));
    let mut value = envelope_json();
    value["input"]["format"] = "ekr-seed/2".into();
    assert!(matches!(
        verify_envelope(&value),
        Err(Refusal::UnsupportedFormat(_))
    ));
    let mut value = envelope_json();
    value["context"]["new_authority"] = true.into();
    assert!(matches!(
        verify_envelope(&value),
        Err(Refusal::InvalidData(_))
    ));
    let mut value = envelope_json();
    let duplicate = value["input"]["ontology"]["node_types"][0].clone();
    value["input"]["ontology"]["node_types"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    assert!(matches!(
        verify_envelope(&value),
        Err(Refusal::DuplicateMember(_))
    ));
}

#[test]
fn frozen_contracts_refuse_unknown_fields_and_duplicate_set_members() {
    let repeated_arguments = br#"{"Invoke":{"node":"00000000-0000-4000-8000-000000000010","operation":"inspect","arguments":{"reason":{"value_kind":"String","value":"first"},"reason":{"value_kind":"String","value":"second"}}}}"#;
    assert!(serde_json::from_slice::<kernel::GraphOperation>(repeated_arguments).is_err());
    for vector in selected("event") {
        let mut value: serde_json::Value = serde_json::from_str(&vector.json).unwrap();
        value["new_effect"] = true.into();
        assert!(decode_json::<graph::RevisionEvent>(&serde_json::to_vec(&value).unwrap()).is_err());
    }
    let vector = selected("transaction").pop().unwrap();
    let mut value: serde_json::Value = serde_json::from_str(&vector.json).unwrap();
    let repeated = value["evidence"][0].clone();
    value["evidence"].as_array_mut().unwrap().push(repeated);
    assert!(matches!(
        decode_json::<kernel::GraphTransaction>(&serde_json::to_vec(&value).unwrap()),
        Err(Refusal::DuplicateMember(_))
    ));
    let mut value: serde_json::Value = serde_json::from_str(&vector.json).unwrap();
    value["operations"][0]["CreateNode"]["new_effect"] = true.into();
    assert!(decode_json::<kernel::GraphTransaction>(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn user_record_discriminator_keys_survive_and_floats_do_not_gain_an_encoding() {
    let vector = selected("value").pop().unwrap();
    let graph::Value::Record(fields) = frozen::<graph::Value>(&vector) else {
        panic!("fixture shape")
    };
    assert!(
        fields.contains_key("event")
            && fields.contains_key("format")
            && fields.contains_key("operation")
    );
    assert!(decode_json::<graph::Value>(br#"{"value_kind":"Float","value":1.5}"#).is_err());
    assert!(decode_json::<graph::Value>(br#"{"value_kind":"Record","value":{"event":{"value_kind":"String","value":"kept"},"event":{"value_kind":"String","value":"lost"}}}"#).is_err());
}

#[test]
fn withdrawn_legacy_assertions_never_acquire_invented_acceptance_history() {
    let vector = selected("assertion")
        .into_iter()
        .find(|v| v.name == "assertion-state-6")
        .unwrap();
    let assertion = frozen::<graph::Assertion>(&vector);
    assert!(matches!(
        assertion.validation,
        graph::ValidationState::Retracted { .. }
    ));
    assert!(!assertion.validation.is_accepted());
    let mut value = envelope_json();
    value["input"]["graph"]["assertions"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap()["validation"] = serde_json::to_value(assertion.validation).unwrap();
    assert!(verify_envelope(&value)
        .unwrap()
        .attributed_document()
        .is_err());
}

#[test]
fn historical_claims_require_the_supplied_bytes_and_revision() {
    let corpus = corpus();
    let vector = corpus
        .vectors
        .iter()
        .find(|v| v.kind == "transaction")
        .unwrap();
    let mut corrupt = vector.json.as_bytes().to_vec();
    corrupt.push(b' ');
    assert!(matches!(
        kernel::GraphTransaction::verify_bytes(
            &corrupt,
            vector.payload_hash,
            corpus.validation.against,
            corpus.validation.validation_hash
        ),
        Err(Refusal::PayloadAddressMismatch { .. })
    ));
    assert!(kernel::GraphTransaction::verify_bytes(
        vector.json.as_bytes(),
        vector.payload_hash,
        RevisionNumber::new(8),
        corpus.validation.validation_hash
    )
    .is_err());
    let transaction: kernel::GraphTransaction = decode_json(vector.json.as_bytes()).unwrap();
    assert!(matches!(
        verify_value(&transaction, vector.payload_hash),
        Err(Refusal::ValueAddressMismatch { .. })
    ));
}
