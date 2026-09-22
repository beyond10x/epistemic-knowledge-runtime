//! Independent probes of supplied original bytes; no provider or current codec is used.
use ekr_core::canonical::{Canonical, Encoder};
use ekr_core::{ContentHash, RevisionNumber};
use ekr_graph::legacy as graph;
use ekr_kernel::legacy as kernel;
use ekr_store::legacy::{decode_json, GraphDocument, Refusal};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};
use std::path::PathBuf;

fn corpus() -> Value {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    serde_json::from_slice(
        &std::fs::read(root.join("../ekr-store/tests/fixtures/legacy/vectors.json")).unwrap(),
    )
    .unwrap()
}

fn vector(name: &str) -> Value {
    corpus()["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == name)
        .unwrap()
        .clone()
}

fn value(name: &str) -> Value {
    serde_json::from_str(vector(name)["json"].as_str().unwrap()).unwrap()
}

#[test]
fn adversary_nested_original_fields_refuse_unknown_semantics() {
    let mut outcomes = Vec::new();
    let mut operation = value("operation-6");
    let definition = operation["DefineNodeType"].as_object_mut().unwrap();
    definition["lifecycle"]
        .as_object_mut()
        .unwrap()
        .insert("future".into(), json!(true));
    outcomes.push((
        "lifecycle",
        decode_json::<kernel::GraphOperation>(&serde_json::to_vec(&operation).unwrap()).is_err(),
    ));

    let mut operation = value("operation-6");
    operation["DefineNodeType"]["operations"]["close"]["arguments"]["reason"]["future"] =
        json!(true);
    outcomes.push((
        "operation argument type",
        decode_json::<kernel::GraphOperation>(&serde_json::to_vec(&operation).unwrap()).is_err(),
    ));

    let mut operation = value("operation-4");
    operation["AddAssertion"]["valid_time"]["future"] = json!(true);
    outcomes.push((
        "assertion time",
        decode_json::<kernel::GraphOperation>(&serde_json::to_vec(&operation).unwrap()).is_err(),
    ));

    let mut evidence = value("human-statement");
    evidence["source"]["HumanStatement"]["future"] = json!(true);
    outcomes.push((
        "evidence source",
        decode_json::<graph::Evidence>(&serde_json::to_vec(&evidence).unwrap()).is_err(),
    ));

    assert!(
        outcomes.iter().all(|(_, refused)| *refused),
        "nested data was lost: {outcomes:?}"
    );
}

fn original_serialization<T: DeserializeOwned + Serialize>(record: &Value) {
    let bytes = record["json"].as_str().unwrap().as_bytes();
    let decoded: T = decode_json(bytes).unwrap();
    assert_eq!(
        serde_json::to_vec(&decoded).unwrap(),
        bytes,
        "{}",
        record["name"]
    );
}

#[test]
fn adversary_frozen_serializers_preserve_all_original_capture_bytes() {
    let capture = corpus();
    let records = capture["vectors"].as_array().unwrap();
    assert_eq!(records.len(), 52);
    for record in records {
        match record["kind"].as_str().unwrap() {
            "value" => original_serialization::<graph::Value>(record),
            "node" => original_serialization::<graph::Node>(record),
            "edge" => original_serialization::<graph::Edge>(record),
            "evidence" => original_serialization::<graph::Evidence>(record),
            "assertion" => original_serialization::<graph::Assertion>(record),
            "operation" => original_serialization::<kernel::GraphOperation>(record),
            "transaction" => original_serialization::<kernel::GraphTransaction>(record),
            "event" => original_serialization::<graph::RevisionEvent>(record),
            "graph-document" => original_serialization::<GraphDocument>(record),
            "seed" => original_serialization::<kernel::SeedDocument>(record),
            "seed-envelope" => original_serialization::<kernel::SeedEnvelope>(record),
            "revision-root" => original_serialization::<graph::Root>(record),
            other => panic!("unreviewed fixture kind {other}"),
        }
    }
}

struct ValidationPair<'a>(&'a kernel::GraphTransaction, RevisionNumber);
impl Canonical for ValidationPair<'_> {
    fn encode(&self, encoder: &mut Encoder) {
        self.0.encode(encoder);
        self.1.encode(encoder);
    }
}

#[test]
fn adversary_validation_receipts_refuse_identical_bytes_in_wrong_hash_domain_and_order() {
    let capture = corpus();
    let record = vector("all-operations");
    let bytes = record["json"].as_str().unwrap().as_bytes();
    let payload = record["payload_hash"].as_str().unwrap().parse().unwrap();
    let against: RevisionNumber =
        serde_json::from_value(capture["validation"]["against"].clone()).unwrap();
    let expected: ContentHash = capture["validation"]["validation_hash"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let transaction =
        kernel::GraphTransaction::verify_bytes(bytes, payload, against, expected).unwrap();
    let pair = ValidationPair(&transaction, against);
    assert_eq!(
        pair.canonical_bytes(),
        transaction.validation_bytes(against)
    );
    let wrong_domain = ContentHash::of(&pair);
    assert_ne!(wrong_domain, expected);
    assert!(matches!(
        kernel::GraphTransaction::verify_bytes(bytes, payload, against, wrong_domain),
        Err(Refusal::PayloadAddressMismatch { expected: refused, observed }) if refused == wrong_domain && observed == expected
    ));
    let mut reordered = transaction.clone();
    reordered.operations.rotate_left(1);
    let mut duplicated = transaction.clone();
    duplicated
        .operations
        .push(transaction.operations[4].clone());
    for changed in [reordered, duplicated] {
        let changed_bytes = serde_json::to_vec(&changed).unwrap();
        let address = ContentHash::of_bytes(&changed_bytes);
        assert!(
            kernel::GraphTransaction::verify_bytes(&changed_bytes, address, against, expected)
                .is_err()
        );
        kernel::GraphTransaction::verify_bytes(
            &changed_bytes,
            address,
            against,
            changed.validation_address(against),
        )
        .unwrap();
    }
}

fn envelope(value: &Value) -> kernel::SeedEnvelope {
    let bytes = serde_json::to_vec(value).unwrap();
    kernel::SeedEnvelope::verify_bytes(&bytes, ContentHash::of_bytes(&bytes)).unwrap()
}

#[test]
fn adversary_seed_attribution_preserves_input_and_refuses_mismatched_original_contexts() {
    let original = value("seed-envelope");
    let retained = envelope(&original);
    let before = retained.input.graph.clone();
    let attributed = retained.attributed_document().unwrap();
    assert_eq!(retained.input.graph, before);
    assert_ne!(attributed.knowledge_address(), before.knowledge_address());
    assert_eq!(attributed.evidence_address(), before.evidence_address());
    assert_eq!(serde_json::to_value(retained).unwrap(), original);

    let other = json!("00000000-0000-4000-8000-000000000099");
    let mut variants = Vec::new();
    for (name, pointer, replacement) in [
        (
            "same execution identities",
            "/context/validator",
            original["context"]["operator"].clone(),
        ),
        ("nonseed revision", "/input/graph/revision", json!(1)),
        ("graph parent", "/input/graph/root/parent", other.clone()),
        (
            "transient graph",
            "/input/graph/root/space",
            json!("Transient"),
        ),
        ("later ontology", "/input/ontology/version/number", json!(1)),
        (
            "ontology parent",
            "/input/ontology/version/parent",
            other.clone(),
        ),
        (
            "schema mismatch",
            "/input/ontology/version/id",
            other.clone(),
        ),
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        variants.push((name, changed));
    }
    let mut changed = original.clone();
    changed["input"]["graph"]["assertions"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap()["proposed_by"] = other.clone();
    variants.push(("assertion proposer", changed));
    let mut changed = original.clone();
    changed["input"]["graph"]["evidence"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap()["extracted_by"] = other;
    variants.push(("evidence attribution", changed));
    let mut changed = original;
    changed["input"]["graph"]["evidence"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap()["source"] = json!({"Url":"https://example.invalid/runtime"});
    variants.push(("unsupported bootstrap source", changed));
    let admitted: Vec<_> = variants
        .into_iter()
        .filter_map(|(name, changed)| {
            envelope(&changed)
                .attributed_document()
                .is_ok()
                .then_some(name)
        })
        .collect();
    assert!(
        admitted.is_empty(),
        "mismatched bootstrap context accepted: {admitted:?}"
    );
}

#[test]
fn adversary_nonproposed_seed_states_never_receive_reconstructed_acceptance() {
    let original = value("seed-envelope");
    for state in 1..=6 {
        let assertion = value(&format!("assertion-state-{state}"));
        let mut changed = original.clone();
        changed["input"]["graph"]["assertions"]
            .as_object_mut()
            .unwrap()
            .values_mut()
            .next()
            .unwrap()["validation"] = assertion["validation"].clone();
        let retained = envelope(&changed);
        assert!(
            matches!(retained.attributed_document(), Err(Refusal::InvalidData(message)) if message == "unsupported-original-bootstrap-attribution"),
            "nonproposed state {state} was reattributed"
        );
    }
}

#[test]
fn adversary_noncanonical_identity_spelling_and_escaped_duplicate_keys_refuse() {
    let mut document = value("proposed-graph");
    let evidence = document["evidence"].as_object_mut().unwrap();
    let (id, record) = evidence
        .iter()
        .next()
        .map(|(id, record)| (id.clone(), record.clone()))
        .unwrap();
    let alias = id.to_uppercase();
    assert_ne!(alias, id, "fixture needs a letter in its UUID");
    evidence.insert(alias, record);
    let bytes = serde_json::to_vec(&document).unwrap();
    let result = GraphDocument::verify_bytes(&bytes, ContentHash::of_bytes(&bytes));
    assert!(
        matches!(result, Err(Refusal::InvalidData(_))),
        "noncanonical identity: {result:?}"
    );
    let bytes = br#"{"value_kind":"Record","value":{"event":{"value_kind":"Integer","value":1},"\u0065vent":{"value_kind":"Integer","value":2}}}"#;
    assert!(matches!(
        decode_json::<graph::Value>(bytes),
        Err(Refusal::DuplicateKey(_))
    ));
}
