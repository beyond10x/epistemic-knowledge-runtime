//! Fixed current-format vectors for the kernel's retained carriers: the six decision records and
//! the complete validation basis (§ 91.4), the host authority anchor and its profile (§ 91.1), the
//! value-domain validation material (§ 89), and the `ekr-seed-envelope/2` a real seed retains
//! together with the Root0 it produces (§ 91.2) on both providers.
//!
//! The frozen original-format vectors are `tests/legacy.rs`. These are the current counterparts:
//! one fixed input each, with its exact retained bytes or content address pinned as a literal.
mod current_fixture;

use std::collections::BTreeSet;

use current_fixture::{anchor, context, id, seed, seeded, SEEDED_AT};
use ekr_core::canonical::Canonical;
use ekr_core::{ContentHash, RevisionNumber, Timestamp};
use ekr_graph::{CanonicalValue, Root};
use ekr_kernel::{
    CommitReceiptV1, GraphOperation, GraphTransaction, NodeDraft, ProposalRecordV1,
    RecordedValidationIssue, RejectionRecordV1, SeedResultV1, StaleRecordV1, ValidationBasisV1,
    ValidationMaterialV1, ValidationReceiptV1, ValidatorName,
};

fn hash(byte: u8) -> ContentHash {
    ContentHash::from_bytes([byte; 32])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Every mismatch at once, with its actual value.
#[derive(Default)]
struct Pins(Vec<String>);
impl Pins {
    fn check(&mut self, name: &str, actual: impl Into<String>, expected: &str) {
        let actual = actual.into();
        if actual != expected {
            self.0.push(format!("{name}: actual {actual}"));
        }
    }
    fn finish(self) {
        assert!(self.0.is_empty(), "\n{}", self.0.join("\n"));
    }
}

fn root() -> Root {
    Root {
        revision: RevisionNumber::new(2),
        parent: Some(hash(0xd0)),
        ontology_root: hash(0xd1),
        knowledge_root: hash(0xd2),
        evidence_root: hash(0xd3),
        agent_root: hash(0xd4),
        transaction: hash(0xd5),
    }
}

fn proposal() -> ProposalRecordV1 {
    ProposalRecordV1 {
        format: ProposalRecordV1::FORMAT.into(),
        event_id: id(0x50),
        submitted_at: Timestamp::from_millis(20),
        submitter: id(0x03),
        document_hash: hash(0xe0),
        document_bytes: b"format: x".to_vec(),
        transaction_id: id(0x51),
        operation_count: 1,
        evidence_hash: hash(0xe1),
        canonical_transaction_hash: Some(hash(0xe2)),
        canonical_operations_hash: None,
    }
}

fn basis() -> ValidationBasisV1 {
    ValidationBasisV1 {
        format: ValidationBasisV1::FORMAT.into(),
        graph_root_id: id(0x02),
        previous_revision_id: id(0x52),
        previous_event_id: id(0x53),
        previous_record_hash: hash(0xe3),
        previous_root: root(),
        previous_root_hash: ContentHash::of(&root()),
        seed_hash: hash(0xe4),
        ontology_root: hash(0xd1),
        authority_root: hash(0xd4),
        validation_profile_hash: hash(0xe5),
    }
}

fn validation() -> ValidationReceiptV1 {
    ValidationReceiptV1 {
        format: ValidationReceiptV1::FORMAT.into(),
        event_id: id(0x54),
        proposed_event_id: id(0x50),
        proposal_record_hash: hash(0xe6),
        transaction_hash: hash(0xe2),
        operations_hash: hash(0xe7),
        evidence_hash: hash(0xe1),
        operation_count: 1,
        basis: basis(),
        validators: BTreeSet::from([id(0x04)]),
        validated_at: Timestamp::from_millis(30),
        validation_hash: hash(0xe8),
    }
}

fn commit() -> CommitReceiptV1 {
    CommitReceiptV1 {
        format: CommitReceiptV1::FORMAT.into(),
        event_id: id(0x55),
        revision_id: id(0x56),
        proposal: proposal(),
        validation: validation(),
        validation_record_hash: hash(0xe9),
        committer: id(0x03),
        committed_at: Timestamp::from_millis(40),
        result: root(),
        result_hash: ContentHash::of(&root()),
    }
}

fn seed_result() -> SeedResultV1 {
    SeedResultV1 {
        format: SeedResultV1::FORMAT.into(),
        event_id: id(0x57),
        revision_id: id(0x58),
        seed_hash: hash(0xe4),
        authority_root: hash(0xd4),
        committed_at: SEEDED_AT,
        result: root(),
        result_hash: ContentHash::of(&root()),
    }
}

fn rejection() -> RejectionRecordV1 {
    RejectionRecordV1 {
        format: RejectionRecordV1::FORMAT.into(),
        event_id: id(0x59),
        proposed_event_id: id(0x50),
        proposal_record_hash: hash(0xe6),
        requested_basis: basis(),
        validator: id(0x04),
        rejected_at: Timestamp::from_millis(31),
        issues: vec![RecordedValidationIssue {
            id: id(0x5a),
            transaction_id: id(0x51),
            validator: ValidatorName::Reference,
            code: "dangling-reference".into(),
            message: "the edge target is not a node".into(),
        }],
    }
}

fn stale() -> StaleRecordV1 {
    StaleRecordV1 {
        format: StaleRecordV1::FORMAT.into(),
        event_id: id(0x5b),
        validation_record_hash: hash(0xe9),
        expected_basis: basis(),
        observed_revision_id: id(0x5c),
        observed_event_id: id(0x5d),
        observed_record_hash: hash(0xea),
        observed_root: root(),
        observed_root_hash: ContentHash::of(&root()),
        stale_at: Timestamp::from_millis(41),
    }
}

/// One record's exact retained bytes, its payload-domain address, and a strict decode of the
/// pinned bytes back to the same value.
macro_rules! record {
    ($pins:ident, $name:literal, $value:expr, $type:ty, $json:expr, $address:expr) => {{
        let value: $type = $value;
        let bytes = value.to_bytes().unwrap();
        $pins.check(
            concat!($name, " bytes"),
            String::from_utf8(bytes.clone()).unwrap(),
            $json,
        );
        $pins.check(
            concat!($name, " address"),
            ContentHash::of_bytes(&bytes).to_hex(),
            $address,
        );
        let json: &str = $json;
        if !json.is_empty() {
            assert_eq!(<$type>::from_bytes(json.as_bytes()).unwrap(), value);
        }
    }};
}

#[test]
fn every_retained_decision_record_has_fixed_bytes_and_address() {
    let mut pins = Pins::default();
    record!(
        pins,
        "proposal",
        proposal(),
        ProposalRecordV1,
        r#"{"format":"ekr.proposal-record/1","event_id":"00000000-0000-4000-8000-000000000050","submitted_at":20,"submitter":"00000000-0000-4000-8000-000000000003","document_hash":"e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0","document_bytes":[102,111,114,109,97,116,58,32,120],"transaction_id":"00000000-0000-4000-8000-000000000051","operation_count":1,"evidence_hash":"e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1","canonical_transaction_hash":"e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2","canonical_operations_hash":null}"#,
        "e968d6ed3da7f291b0a680c2dfd733eff36352a1942ab6f4e69ce281cff03769"
    );
    record!(
        pins,
        "validation",
        validation(),
        ValidationReceiptV1,
        r#"{"format":"ekr.validation-receipt/1","event_id":"00000000-0000-4000-8000-000000000054","proposed_event_id":"00000000-0000-4000-8000-000000000050","proposal_record_hash":"e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6","transaction_hash":"e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2","operations_hash":"e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7","evidence_hash":"e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1","operation_count":1,"basis":{"format":"ekr.validation-basis/1","graph_root_id":"00000000-0000-4000-8000-000000000002","previous_revision_id":"00000000-0000-4000-8000-000000000052","previous_event_id":"00000000-0000-4000-8000-000000000053","previous_record_hash":"e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3","previous_root":{"revision":2,"parent":"d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0","ontology_root":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1","knowledge_root":"d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2","evidence_root":"d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3","agent_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","transaction":"d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5"},"previous_root_hash":"a3645e9e49c98a12e012c597b50c713be25d00e78d215a4f2cebff28f9651218","seed_hash":"e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4","ontology_root":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1","authority_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","validation_profile_hash":"e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5"},"validators":["00000000-0000-4000-8000-000000000004"],"validated_at":30,"validation_hash":"e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8"}"#,
        "e6d96eb95648933adc95f845666b16d62fca982289c01a0298f39d7e65f22f7e"
    );
    record!(
        pins,
        "commit",
        commit(),
        CommitReceiptV1,
        r#"{"format":"ekr.commit-receipt/1","event_id":"00000000-0000-4000-8000-000000000055","revision_id":"00000000-0000-4000-8000-000000000056","proposal":{"format":"ekr.proposal-record/1","event_id":"00000000-0000-4000-8000-000000000050","submitted_at":20,"submitter":"00000000-0000-4000-8000-000000000003","document_hash":"e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0e0","document_bytes":[102,111,114,109,97,116,58,32,120],"transaction_id":"00000000-0000-4000-8000-000000000051","operation_count":1,"evidence_hash":"e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1","canonical_transaction_hash":"e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2","canonical_operations_hash":null},"validation":{"format":"ekr.validation-receipt/1","event_id":"00000000-0000-4000-8000-000000000054","proposed_event_id":"00000000-0000-4000-8000-000000000050","proposal_record_hash":"e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6","transaction_hash":"e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2e2","operations_hash":"e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7e7","evidence_hash":"e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1e1","operation_count":1,"basis":{"format":"ekr.validation-basis/1","graph_root_id":"00000000-0000-4000-8000-000000000002","previous_revision_id":"00000000-0000-4000-8000-000000000052","previous_event_id":"00000000-0000-4000-8000-000000000053","previous_record_hash":"e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3","previous_root":{"revision":2,"parent":"d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0","ontology_root":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1","knowledge_root":"d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2","evidence_root":"d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3","agent_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","transaction":"d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5"},"previous_root_hash":"a3645e9e49c98a12e012c597b50c713be25d00e78d215a4f2cebff28f9651218","seed_hash":"e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4","ontology_root":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1","authority_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","validation_profile_hash":"e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5"},"validators":["00000000-0000-4000-8000-000000000004"],"validated_at":30,"validation_hash":"e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8e8"},"validation_record_hash":"e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9","committer":"00000000-0000-4000-8000-000000000003","committed_at":40,"result":{"revision":2,"parent":"d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0","ontology_root":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1","knowledge_root":"d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2","evidence_root":"d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3","agent_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","transaction":"d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5"},"result_hash":"a3645e9e49c98a12e012c597b50c713be25d00e78d215a4f2cebff28f9651218"}"#,
        "5d717667c25888e6553725ef1e96f6ddba0de65cdeb276397163347203f9f4ed"
    );
    record!(
        pins,
        "seed-result",
        seed_result(),
        SeedResultV1,
        r#"{"format":"ekr.seed-result/1","event_id":"00000000-0000-4000-8000-000000000057","revision_id":"00000000-0000-4000-8000-000000000058","seed_hash":"e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4","authority_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","committed_at":10,"result":{"revision":2,"parent":"d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0","ontology_root":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1","knowledge_root":"d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2","evidence_root":"d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3","agent_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","transaction":"d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5"},"result_hash":"a3645e9e49c98a12e012c597b50c713be25d00e78d215a4f2cebff28f9651218"}"#,
        "62fc540bb21cbe970630aefb7052917ec4f0a06d14f6ef5c06348ec8ed20db31"
    );
    record!(
        pins,
        "rejection",
        rejection(),
        RejectionRecordV1,
        r#"{"format":"ekr.rejection-record/1","event_id":"00000000-0000-4000-8000-000000000059","proposed_event_id":"00000000-0000-4000-8000-000000000050","proposal_record_hash":"e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6e6","requested_basis":{"format":"ekr.validation-basis/1","graph_root_id":"00000000-0000-4000-8000-000000000002","previous_revision_id":"00000000-0000-4000-8000-000000000052","previous_event_id":"00000000-0000-4000-8000-000000000053","previous_record_hash":"e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3","previous_root":{"revision":2,"parent":"d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0","ontology_root":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1","knowledge_root":"d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2","evidence_root":"d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3","agent_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","transaction":"d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5"},"previous_root_hash":"a3645e9e49c98a12e012c597b50c713be25d00e78d215a4f2cebff28f9651218","seed_hash":"e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4","ontology_root":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1","authority_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","validation_profile_hash":"e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5"},"validator":"00000000-0000-4000-8000-000000000004","rejected_at":31,"issues":[{"id":"00000000-0000-4000-8000-00000000005a","transaction_id":"00000000-0000-4000-8000-000000000051","validator":"Reference","code":"dangling-reference","message":"the edge target is not a node"}]}"#,
        "f65e403fa3db0282d134176172b8a4a2af589d07cf8f10ea5d563b1dd5a532c5"
    );
    record!(
        pins,
        "stale",
        stale(),
        StaleRecordV1,
        r#"{"format":"ekr.stale-record/1","event_id":"00000000-0000-4000-8000-00000000005b","validation_record_hash":"e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9e9","expected_basis":{"format":"ekr.validation-basis/1","graph_root_id":"00000000-0000-4000-8000-000000000002","previous_revision_id":"00000000-0000-4000-8000-000000000052","previous_event_id":"00000000-0000-4000-8000-000000000053","previous_record_hash":"e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3","previous_root":{"revision":2,"parent":"d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0","ontology_root":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1","knowledge_root":"d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2","evidence_root":"d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3","agent_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","transaction":"d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5"},"previous_root_hash":"a3645e9e49c98a12e012c597b50c713be25d00e78d215a4f2cebff28f9651218","seed_hash":"e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4","ontology_root":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1","authority_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","validation_profile_hash":"e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5"},"observed_revision_id":"00000000-0000-4000-8000-00000000005c","observed_event_id":"00000000-0000-4000-8000-00000000005d","observed_record_hash":"eaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaeaea","observed_root":{"revision":2,"parent":"d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0","ontology_root":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1","knowledge_root":"d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2","evidence_root":"d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3","agent_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","transaction":"d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5"},"observed_root_hash":"a3645e9e49c98a12e012c597b50c713be25d00e78d215a4f2cebff28f9651218","stale_at":41}"#,
        "aeda623ae43d87c808f2f5252ed1f609d864b7c3aadd9d366a989babbcd3a539"
    );
    pins.finish();
}

#[test]
fn the_complete_validation_basis_has_fixed_record_and_canonical_forms() {
    let mut pins = Pins::default();
    record!(
        pins,
        "basis",
        basis(),
        ValidationBasisV1,
        r#"{"format":"ekr.validation-basis/1","graph_root_id":"00000000-0000-4000-8000-000000000002","previous_revision_id":"00000000-0000-4000-8000-000000000052","previous_event_id":"00000000-0000-4000-8000-000000000053","previous_record_hash":"e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3","previous_root":{"revision":2,"parent":"d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0","ontology_root":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1","knowledge_root":"d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2","evidence_root":"d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3","agent_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","transaction":"d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5"},"previous_root_hash":"a3645e9e49c98a12e012c597b50c713be25d00e78d215a4f2cebff28f9651218","seed_hash":"e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4","ontology_root":"d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1","authority_root":"d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4","validation_profile_hash":"e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5"}"#,
        "63c07bc04277b813817cfbb29c88cc9c8efb6decadbc60a8a5bb72ec6eccf0ff"
    );
    pins.check("basis canonical", hex(&basis().canonical_bytes()), "060000000000000016656b722e76616c69646174696f6e2d62617369732f310d000000000000400080000000000000020d000000000000400080000000000000520d000000000000400080000000000000530ee3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e3e304000000000000000000000000000000020c0ed0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d0d00ed1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d10ed2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d2d20ed3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d3d30ed4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d40ed5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d5d50ea3645e9e49c98a12e012c597b50c713be25d00e78d215a4f2cebff28f96512180ee4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e4e40ed1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d1d10ed4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d4d40ee5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5e5");
    pins.check(
        "basis value address",
        ContentHash::of(&basis()).to_hex(),
        "db4a8512536548ce30a4ceeb9bf0aeb54781e55f607732aae18a2fcfdbb046ff",
    );
    pins.finish();
}

fn canonical_transaction() -> GraphTransaction<CanonicalValue> {
    GraphTransaction {
        id: id(0x51),
        proposer: id(0x03),
        operations: vec![GraphOperation::CreateNode(NodeDraft {
            id: id(0x60),
            root_id: id(0x02),
            type_id: id(0x05),
            canonical_name: "third".into(),
            properties: [(id(0x06), vec![CanonicalValue::String("beta".into())])]
                .into_iter()
                .collect(),
        })],
        evidence: BTreeSet::from([id(0x13)]),
    }
}

#[test]
fn validation_material_has_a_fixed_value_address() {
    let tx = canonical_transaction();
    let validators = BTreeSet::from([id(0x04)]);
    let mut pins = Pins::default();
    pins.check(
        "transaction",
        ContentHash::of(&tx).to_hex(),
        "502e70734ace7c78c4e32b79bf387997703f1c75dba971c266db7bc132a5dd12",
    );
    pins.check(
        "validation material",
        ContentHash::of(&ValidationMaterialV1 {
            transaction: &tx,
            basis: &basis(),
            validators: &validators,
        })
        .to_hex(),
        "7887754cd4666173dc724d18a3efde8f30a7b608a0ed9c980a73f114c966d562",
    );
    pins.finish();
}

#[test]
fn the_host_anchor_and_its_profile_have_fixed_value_addresses() {
    let anchor = anchor();
    let mut pins = Pins::default();
    pins.check(
        "authority state",
        ContentHash::of(&anchor).to_hex(),
        "765656ac90343aa873f2a76b11c93f4e55d6684c2fe00bbaaacced3a9d92327c",
    );
    pins.check(
        "validation profile",
        ContentHash::of(&anchor.validation_profile).to_hex(),
        "737bca2f945739ce127141686114487183f3793fb9c83caabf50ac8599851645",
    );
    pins.check(
        "profile bytes",
        hex(&anchor.validation_profile.canonical_bytes()),
        "06000000000000001b656b722e70312d76616c69646174696f6e2d70726f66696c652f31060000000000000016656b722e70312d64657465726d696e69737469632f310800000000000000070f000000000f000000010f000000020f000000030f000000040f000000050f000000060d0000000000004000800000000000000406000000000000001e64697374696e63742d61757468656e746963617465642d6163746f722f3106000000000000001e72657461696e65642d61646d69737369626c652d65766964656e63652f3106000000000000000e656b722e70312d6170706c792f31",
    );
    pins.finish();
}

/// A real seed on both providers: the retained envelope and Root0 are identical and pinned.
#[test]
fn a_real_seed_retains_a_fixed_envelope_and_produces_a_fixed_root_on_both_providers() {
    let mut pins = Pins::default();
    for file in [false, true] {
        let (_directory, runtime, result) = seeded(seed(), context(), anchor(), file, SEEDED_AT);
        let provider = if file { "file" } else { "sqlite" };
        let envelope = runtime.content(&result.seed_hash).unwrap().unwrap();
        assert_eq!(ContentHash::of_bytes(&envelope), result.seed_hash);
        let text = String::from_utf8(envelope).unwrap();
        assert!(
            text.starts_with(r#"{"format":"ekr-seed-envelope/2","input":{"format":"ekr-seed/2","#),
            "{provider}: {text}"
        );
        let order: Vec<usize> = [
            r#""input":"#,
            r#""context":{"operator""#,
            r#""authority":{"format":"ekr.authority-state/1""#,
            r#""committed_at":10}"#,
        ]
        .iter()
        .map(|key| {
            text.find(key)
                .unwrap_or_else(|| panic!("{provider}: {key}"))
        })
        .collect();
        assert!(
            order.windows(2).all(|w| w[0] < w[1]),
            "{provider}: {order:?}"
        );
        assert!(text.ends_with(r#""committed_at":10}"#), "{provider}");
        let root = result.result;
        pins.check(
            &format!("{provider} envelope"),
            result.seed_hash.to_hex(),
            "d9fc8c0fe513570feebbfd97837e79ee8e7636b657ee7b6974a9c5e892d51dde",
        );
        pins.check(
            &format!("{provider} ontology_root"),
            root.ontology_root.to_hex(),
            "46eddc0a9a78fc47272ca431bade36af25a84265dbd66f736257385208b663f4",
        );
        pins.check(
            &format!("{provider} knowledge_root"),
            root.knowledge_root.to_hex(),
            "4b2eb53ed4e3d1eb56af0a7c0d9f40c2ae28564c8567c1a318911101a3abe2cc",
        );
        pins.check(
            &format!("{provider} evidence_root"),
            root.evidence_root.to_hex(),
            "ddb727b80fe83cd0d433ac3412f75f1d4615e846eebb956dbc6717571a6256e4",
        );
        pins.check(
            &format!("{provider} agent_root"),
            root.agent_root.to_hex(),
            "765656ac90343aa873f2a76b11c93f4e55d6684c2fe00bbaaacced3a9d92327c",
        );
        pins.check(
            &format!("{provider} root"),
            ContentHash::of(&root).to_hex(),
            "8c58ac2ab74064c6002234a830133cb28ff6f5aac7a94429a822bff32698cd9f",
        );
        assert_eq!(root.transaction, result.seed_hash);
        assert_eq!(root.parent, None);
        assert_eq!(root.revision, RevisionNumber::SEED);
        assert_eq!(result.committed_at, SEEDED_AT);
        assert_eq!(result.authority_root, root.agent_root);
        assert_eq!(result.result_hash, ContentHash::of(&root));
    }
    pins.finish();
}
