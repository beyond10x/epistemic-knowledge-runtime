//! Executable contract for `ekr.transaction-document/2` (design § 97): the envelope and grammar of
//! `/1` under a larger frozen profile, 10,000 operations in 8 MiB, while `/1` keeps its own.
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{NodeType, PropertyDefinition, Value, ValueType};
use serde::Serialize;

const ID: &str = "00000000-0000-7000-8000-000000000001";
const V1: &str = "ekr.transaction-document/1";
const V2: &str = "ekr.transaction-document/2";

/// `count` operations in one flow list, under `format`.
fn repeated(format: &str, count: usize) -> String {
    format!(
        "format: {format}\ntransaction: {{id: {ID}, proposer: {ID}, operations: [{}], evidence: []}}",
        vec![format!("!DeleteEdge {ID}"); count].join(",")
    )
}

fn refusal_of(input: &[u8]) -> (DocumentLimit, DocumentFormat, String) {
    match TransactionDocument::parse(input) {
        Err(error @ DocumentError::Limit(limit, format)) => (limit, format, error.to_string()),
        other => panic!("expected a limit refusal, got {other:?}"),
    }
}

#[test]
fn the_two_profiles_are_frozen_and_the_second_contains_the_first() {
    assert_eq!(DocumentFormat::V1.limits(), DOCUMENT_V1_LIMITS);
    assert_eq!(DocumentFormat::V2.limits(), ekr_kernel::DOCUMENT_V2_LIMITS);
    assert_eq!(DOCUMENT_V2_LIMITS.operations, 10_000);
    assert_eq!(DOCUMENT_V2_LIMITS.input_bytes, 8 * 1024 * 1024);
    assert_eq!(DOCUMENT_V1_LIMITS.operations, 256);
    assert_eq!(DOCUMENT_V1_LIMITS.input_bytes, 262_144);
    let (one, two) = (DOCUMENT_V1_LIMITS, DOCUMENT_V2_LIMITS);
    for (first, second) in [
        (one.input_bytes, two.input_bytes),
        (one.depth, two.depth),
        (one.nodes, two.nodes),
        (one.mapping_entries, two.mapping_entries),
        (one.sequence_elements, two.sequence_elements),
        (one.string_bytes, two.string_bytes),
        (one.key_bytes, two.key_bytes),
        (one.total_string_bytes, two.total_string_bytes),
        (one.operations, two.operations),
        (one.evidence, two.evidence),
    ] {
        assert!(first <= second);
    }
    assert!(two.operations <= two.sequence_elements);
    assert_eq!(DocumentFormat::CURRENT, DocumentFormat::V2);
    assert_eq!(DocumentFormat::from_name(V1), Some(DocumentFormat::V1));
    assert_eq!(DocumentFormat::from_name(V2), Some(DocumentFormat::V2));
    assert_eq!(
        DocumentFormat::from_name("ekr.transaction-document/3"),
        None
    );
}

#[test]
fn a_v1_document_over_256_operations_is_still_refused_with_its_own_bound() {
    let at = TransactionDocument::parse(repeated(V1, 256).as_bytes()).unwrap();
    assert_eq!(at.format(), DocumentFormat::V1);
    let (limit, format, refusal) = refusal_of(repeated(V1, 257).as_bytes());
    assert_eq!(
        (limit, format),
        (DocumentLimit::Operations, DocumentFormat::V1)
    );
    assert!(
        refusal.starts_with("transaction document limit: operations (at most 256 operations"),
        "{refusal}"
    );
    assert!(
        refusal.contains(V2),
        "the /1 refusal names the larger format: {refusal}"
    );
}

#[test]
fn a_v2_document_holds_10000_operations_and_refuses_the_10001st_naming_the_bound() {
    let parsed = TransactionDocument::parse(repeated(V2, 10_000).as_bytes()).unwrap();
    assert_eq!(parsed.format(), DocumentFormat::V2);
    assert_eq!(parsed.transaction().operations.len(), 10_000);
    let (limit, format, refusal) = refusal_of(repeated(V2, 10_001).as_bytes());
    assert_eq!(
        (limit, format),
        (DocumentLimit::Operations, DocumentFormat::V2)
    );
    assert!(
        refusal.starts_with("transaction document limit: operations (at most 10000 operations"),
        "{refusal}"
    );
    let (limit, _, _) = refusal_of(repeated(V2, 0).as_bytes());
    assert_eq!(limit, DocumentLimit::EmptyOperations);
}

#[test]
fn the_byte_cap_is_the_declared_versions_and_the_refusal_names_it() {
    let pad = |format: &str, total: usize| {
        let mut text = repeated(format, 1);
        text.push_str("\n#");
        text.extend(std::iter::repeat_n('x', total - text.len()));
        text
    };
    // /2: exactly 8 MiB is read, one byte more is refused naming 8388608.
    let exact = pad(V2, DOCUMENT_V2_LIMITS.input_bytes);
    assert_eq!(
        TransactionDocument::parse(exact.as_bytes())
            .unwrap()
            .format(),
        DocumentFormat::V2
    );
    assert_eq!(
        TransactionDocument::read(exact.as_bytes()).unwrap().bytes(),
        exact.as_bytes()
    );
    let over = pad(V2, DOCUMENT_V2_LIMITS.input_bytes + 1);
    let (limit, format, refusal) = refusal_of(over.as_bytes());
    assert_eq!(
        (limit, format),
        (DocumentLimit::InputBytes, DocumentFormat::V2)
    );
    assert!(
        refusal.ends_with("input_bytes (at most 8388608 bytes)"),
        "{refusal}"
    );
    let mut reader = over.as_bytes();
    assert!(matches!(
        TransactionDocument::read(&mut reader),
        Err(DocumentError::Limit(
            DocumentLimit::InputBytes,
            DocumentFormat::V2
        ))
    ));
    assert!(reader.is_empty() || reader.len() < over.len() - DOCUMENT_V2_LIMITS.input_bytes);
    // /1 keeps 262144, whichever door the bytes come through.
    let v1 = pad(V1, DOCUMENT_V1_LIMITS.input_bytes + 1);
    let (limit, format, refusal) = refusal_of(v1.as_bytes());
    assert_eq!(
        (limit, format),
        (DocumentLimit::InputBytes, DocumentFormat::V1)
    );
    assert!(
        refusal.ends_with("input_bytes (at most 262144 bytes)"),
        "{refusal}"
    );
    assert!(matches!(
        TransactionDocument::read(v1.as_bytes()),
        Err(DocumentError::Limit(
            DocumentLimit::InputBytes,
            DocumentFormat::V1
        ))
    ));
    TransactionDocument::parse(pad(V1, DOCUMENT_V1_LIMITS.input_bytes).as_bytes()).unwrap();
    // A /2 document between the caps is read through the bounded reader too.
    let middle = pad(V2, DOCUMENT_V1_LIMITS.input_bytes * 4);
    assert_eq!(
        TransactionDocument::read(middle.as_bytes())
            .unwrap()
            .format(),
        DocumentFormat::V2
    );
    // A host's stricter upload cap still wins, and says so.
    assert!(matches!(
        TransactionDocument::read_with_upload_limit(middle.as_bytes(), 300_000),
        Err(DocumentError::Limit(
            DocumentLimit::UploadBytes,
            DocumentFormat::V2
        ))
    ));
}

#[test]
fn a_format_not_on_its_own_line_selects_its_profile_under_the_v1_byte_cap() {
    // One flow mapping: no top-level `format:` line. Within the /1 byte cap the declared version
    // still governs: /2 admits 300 operations, /1 refuses them with its own bound.
    let flow = |format: &str, count: usize| {
        format!(
            "{{format: {format}, transaction: {{id: {ID}, proposer: {ID}, operations: [{}], evidence: []}}}}",
            vec![format!("!DeleteEdge {ID}"); count].join(",")
        )
    };
    let two = TransactionDocument::parse(flow(V2, 300).as_bytes()).unwrap();
    assert_eq!(two.format(), DocumentFormat::V2);
    let (limit, format, _) = refusal_of(flow(V1, 300).as_bytes());
    assert_eq!(
        (limit, format),
        (DocumentLimit::Operations, DocumentFormat::V1)
    );
    // A quoted key and a trailing comment on the line are still that line.
    let quoted = repeated(V2, 300).replacen("format: ", "\"format\": ", 1);
    assert_eq!(
        TransactionDocument::parse(quoted.as_bytes())
            .unwrap()
            .format(),
        DocumentFormat::V2
    );
    let commented = repeated(V2, 300).replacen(V2, &format!("'{V2}' # the larger profile"), 1);
    assert_eq!(
        TransactionDocument::parse(commented.as_bytes())
            .unwrap()
            .format(),
        DocumentFormat::V2
    );
    // Over the /1 byte cap, a version the reader cannot see before parsing is held to /1.
    let mut big = flow(V2, 1);
    big.push_str("\n#");
    big.extend(std::iter::repeat_n('x', DOCUMENT_V1_LIMITS.input_bytes));
    let (limit, format, _) = refusal_of(big.as_bytes());
    assert_eq!(
        (limit, format),
        (DocumentLimit::InputBytes, DocumentFormat::V1)
    );
    // An unknown version is refused as one, and a format line naming one version cannot carry
    // a document that declares another past that version's limits.
    assert!(matches!(
        TransactionDocument::parse(repeated("ekr.transaction-document/3", 1).as_bytes()),
        Err(DocumentError::UnsupportedFormat(_))
    ));
}

// A /2 document of 10,000 operations validates and commits on both providers. -----------------

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}
fn anchor() -> AuthorityStateV1 {
    let c = context();
    AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [(c.operator, "operator"), (c.validator, "validator")]
            .into_iter()
            .map(|(id, name)| {
                (
                    id,
                    Agent {
                        id,
                        name: name.into(),
                        capabilities: BTreeSet::new(),
                    },
                )
            })
            .collect(),
        validation_profile: ValidationProfileV1::deterministic(c.validator),
    }
}
fn open(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, "v2", context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), "v2", context(), anchor())
    }
    .unwrap()
}
fn seed() -> SeedDocument {
    let mut seed = SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let type_id = "00000000-0000-4000-8000-000000000005".parse().unwrap();
    let label = "00000000-0000-4000-8000-000000000006".parse().unwrap();
    let mut declared = NodeType::new(type_id, "Subject");
    declared.properties.insert(
        label,
        PropertyDefinition::new(label, "label", ValueType::String),
    );
    seed.ontology.node_types.push(declared);
    let bytes = b"synthetic human evidence".to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let evidence = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::from_basis_points(10000).unwrap(),
    };
    seed.graph.evidence.insert(evidence.id, evidence);
    seed.evidence_payloads.insert(hash, bytes);
    seed
}

/// `operations` operations under `/2`: half new nodes, half an evidenced assertion on each.
fn document(seed: &SeedDocument, operations: usize) -> (TransactionId, Vec<u8>) {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let ty = seed
        .ontology
        .node_types
        .iter()
        .find(|ty| ty.name == "Subject")
        .unwrap();
    let label = *ty.properties.keys().next().unwrap();
    let evidence = *seed.graph.evidence.keys().next().unwrap();
    let mut list = Vec::with_capacity(operations);
    for n in 0..operations / 2 {
        let id = NodeId::mint();
        list.push(GraphOperation::CreateNode(NodeDraft {
            id,
            root_id: seed.graph.root.id,
            type_id: ty.id,
            canonical_name: format!("subject {n}"),
            properties: BTreeMap::new(),
        }));
        list.push(GraphOperation::AddAssertion(Box::new(Assertion {
            id: AssertionId::mint(),
            root_id: seed.graph.root.id,
            subject: Subject::Node(id),
            predicate: Predicate::Property(label),
            object: Object::Value(Value::String(format!("label {n}"))),
            evidence: BTreeSet::from([evidence]),
            proposed_by: context().operator,
            assessment: Assessment::Proposed,
            lifecycle: AssertionLifecycle::Active,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        })));
    }
    let tx = GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: list,
        evidence: BTreeSet::from([evidence]),
        schema_version: None,
    };
    let bytes = serde_yaml_ng::to_string(&Wire {
        format: V2,
        transaction: &tx,
    })
    .unwrap()
    .into_bytes();
    (tx.id, bytes)
}

#[test]
fn a_v2_document_of_10000_operations_validates_and_commits_on_both_providers() {
    let seed = seed();
    let (tx, bytes) = document(&seed, 10_000);
    assert!(bytes.len() > DOCUMENT_V1_LIMITS.input_bytes);
    assert!(bytes.len() <= DOCUMENT_V2_LIMITS.input_bytes);
    for file in [true, false] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        open(path, file)
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
        open(path, file)
            .propose(&bytes, context().operator, || Timestamp::from_millis(100))
            .unwrap();
        let validated = open(path, file)
            .validate(tx, RevisionNumber::new(0), || Timestamp::from_millis(101))
            .unwrap();
        assert!(
            matches!(validated, ValidationCommandResult::Validated(_)),
            "file={file}: {validated:?}"
        );
        let committed = open(path, file)
            .commit(tx, context().operator, || Timestamp::from_millis(102))
            .unwrap();
        let CommitCommandResult::Committed(receipt) = committed else {
            panic!("file={file}: {committed:?}");
        };
        assert_eq!(receipt.result.revision, RevisionNumber::new(1));
        let reopened = open(path, file);
        assert_eq!(reopened.head().unwrap(), Some(receipt.result));
        let snapshot = reopened.snapshot().unwrap();
        assert_eq!(snapshot.nodes.len(), seed.graph.nodes.len() + 5_000);
        assert_eq!(
            snapshot.assertions.len(),
            seed.graph.assertions.len() + 5_000
        );
    }
}
