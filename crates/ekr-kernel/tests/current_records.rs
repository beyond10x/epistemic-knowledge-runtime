//! Current record codecs remain strict; successful decoding alone never grants authority.
use ekr_core::{
    AgentId, ContentHash, EventId, GraphRootId, RevisionId, RevisionNumber, Timestamp,
    TransactionId,
};
use ekr_graph::{RevisionEvent, RevisionPayload, Root};
use ekr_kernel::{
    CommitReceiptV1, ProposalRecordV1, RejectionRecordV1, SeedResultV1, StaleRecordV1,
    ValidationBasisV1, ValidationReceiptV1,
};
use serde_json::{json, Value};
fn records() -> [Value; 7] {
    let event = EventId::mint();
    let agent = AgentId::mint();
    let hash = ContentHash::of_bytes(b"record codec fixture");
    let root = Root {
        revision: RevisionNumber::SEED,
        parent: None,
        ontology_root: hash,
        knowledge_root: hash,
        evidence_root: hash,
        agent_root: hash,
        transaction: hash,
    };
    let proposal = json!({"format":ProposalRecordV1::FORMAT, "event_id":event,
        "submitted_at":0,"submitter":agent,"document_hash":hash,"document_bytes":[1,2,3],
        "transaction_id":TransactionId::mint(),"operation_count":1,"evidence_hash":hash,
        "canonical_transaction_hash":null,"canonical_operations_hash":null});
    let basis = json!({"format":ValidationBasisV1::FORMAT,"graph_root_id":GraphRootId::mint(),
        "previous_revision_id":RevisionId::mint(),"previous_event_id":event,"previous_record_hash":hash,
        "previous_root":root,"previous_root_hash":hash,"seed_hash":hash,"ontology_root":hash,
        "authority_root":hash,"validation_profile_hash":hash});
    let validation = json!({"format":ValidationReceiptV1::FORMAT,"event_id":event,
        "proposed_event_id":event,"proposal_record_hash":hash,"transaction_hash":hash,
        "operations_hash":hash,"evidence_hash":hash,"operation_count":1,"basis":basis,
        "validators":[agent],"validated_at":0,"validation_hash":hash});
    let commit = json!({"format":CommitReceiptV1::FORMAT,"event_id":event,"revision_id":RevisionId::mint(),
        "proposal":proposal,"validation":validation,"validation_record_hash":hash,
        "committer":agent,"committed_at":0,"result":root,"result_hash":hash});
    let seed = json!({"format":SeedResultV1::FORMAT,"event_id":event,"revision_id":RevisionId::mint(),
        "seed_hash":hash,"authority_root":hash,"committed_at":0,"result":root,"result_hash":hash});
    let rejection = json!({"format":RejectionRecordV1::FORMAT,"event_id":event,"proposed_event_id":event,
        "proposal_record_hash":hash,"requested_basis":basis,"validator":agent,"rejected_at":0,"issues":[]});
    let stale = json!({"format":StaleRecordV1::FORMAT,"event_id":event,"validation_record_hash":hash,
        "expected_basis":basis,"observed_revision_id":RevisionId::mint(),"observed_event_id":event,
        "observed_record_hash":hash,"observed_root":root,"observed_root_hash":hash,"stale_at":0});
    [proposal, basis, validation, commit, seed, rejection, stale]
}
macro_rules! codec {
    ($name:ident, $type:ty, $index:expr) => {
        #[test]
        fn $name() {
            let value = records()[$index].clone();
            let record = <$type>::from_bytes(&serde_json::to_vec(&value).unwrap()).unwrap();
            let bytes = record.to_bytes().unwrap();
            assert_eq!(<$type>::from_bytes(&bytes).unwrap(), record);
            let mut unknown = value.clone();
            unknown["ignored_semantics"] = true.into();
            assert!(<$type>::from_bytes(&serde_json::to_vec(&unknown).unwrap()).is_err());
            let mut wrong = value.clone();
            wrong["format"] = "unsupported/999".into();
            assert!(<$type>::from_bytes(&serde_json::to_vec(&wrong).unwrap()).is_err());
            let mut missing = value;
            missing.as_object_mut().unwrap().remove("format");
            assert!(<$type>::from_bytes(&serde_json::to_vec(&missing).unwrap()).is_err());
            let duplicate = format!(
                "{{\"format\":{},{}",
                serde_json::to_string(<$type>::FORMAT).unwrap(),
                &String::from_utf8(bytes).unwrap()[1..]
            );
            assert!(<$type>::from_bytes(duplicate.as_bytes()).is_err());
        }
    };
}
codec!(proposal_record_is_strict, ProposalRecordV1, 0);
codec!(complete_validation_basis_is_strict, ValidationBasisV1, 1);
codec!(validation_receipt_is_strict, ValidationReceiptV1, 2);
codec!(commit_receipt_is_strict, CommitReceiptV1, 3);
codec!(seed_result_is_strict, SeedResultV1, 4);
codec!(rejection_record_is_strict, RejectionRecordV1, 5);
codec!(stale_record_is_strict, StaleRecordV1, 6);

#[test]
fn nested_record_versions_and_validator_duplicates_refuse() {
    let mut values = records();
    values[3]["validation"]["basis"]["format"] = "ekr.validation-basis/999".into();
    assert!(CommitReceiptV1::from_bytes(&serde_json::to_vec(&values[3]).unwrap()).is_err());
    let validator = values[2]["validators"][0].clone();
    values[2]["validators"] = json!([validator, validator]);
    assert!(ValidationReceiptV1::from_bytes(&serde_json::to_vec(&values[2]).unwrap()).is_err());
}

#[test]
fn event_occurrence_and_record_addresses_contribute_independently() {
    let identity = EventId::mint();
    assert_eq!(identity.to_string().parse::<EventId>().unwrap(), identity);
    assert_eq!(
        EventId::from_uuid(identity.to_uuid()).as_u128(),
        identity.as_u128()
    );
    let event = RevisionEvent {
        format: RevisionEvent::FORMAT.into(),
        event_id: identity,
        record_hash: ContentHash::of_bytes(b"one"),
        payload: RevisionPayload::TransactionProposed {
            transaction_id: TransactionId::mint(),
            proposer: AgentId::mint(),
            operations_hash: None,
        },
    };
    assert_eq!(event.variant_index(), 1);
    assert_eq!(event.name(), "ekr.kernel.TransactionProposed");
    let original = ContentHash::of(&event);
    let mut changed = event.clone();
    changed.event_id = EventId::mint();
    assert_ne!(original, ContentHash::of(&changed));
    changed = event.clone();
    changed.record_hash = ContentHash::of_bytes(b"two");
    assert_ne!(original, ContentHash::of(&changed));
    changed = event;
    changed.format = "ekr.revision-event/999".into();
    assert_ne!(original, ContentHash::of(&changed));
    assert_eq!(Timestamp::EPOCH.millis(), 0);
}
