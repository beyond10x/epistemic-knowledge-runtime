//! The original five commit-boundary cases, migrated to actual retained command records.
//! No transient in-memory attestation substitutes for kernel replay after reopening.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::*;
use ekr_store::{CommitAuthority, GraphDocument, RevisionLog, SqliteStore, StorageClass};
use std::collections::{BTreeMap, BTreeSet};

struct World {
    seed: SeedDocument,
    context: BootstrapContext,
    anchor: AuthorityStateV1,
    ty: TypeId,
}
fn world() -> World {
    let schema = SchemaVersionId::mint();
    let ty = TypeId::mint();
    let context = BootstrapContext {
        operator: AgentId::mint(),
        validator: AgentId::mint(),
    };
    let anchor = AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [
            (context.operator, "operator"),
            (context.validator, "validator"),
        ]
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
        validation_profile: ValidationProfileV1::deterministic(context.validator),
    };
    let seed = SeedDocument {
        format: "ekr-seed/2".into(),
        ontology: OntologyDocument {
            version: SchemaVersion::seed(schema, Timestamp::EPOCH),
            node_types: vec![NodeType::new(ty, "Thing")],
            edge_types: vec![],
        },
        graph: GraphDocument {
            root: GraphRoot {
                id: GraphRootId::mint(),
                space: Space::Canonical,
                schema_version_id: schema,
                parent: None,
                created_at: Timestamp::EPOCH,
            },
            revision: RevisionNumber::SEED,
            nodes: BTreeMap::new(),
            edges: BTreeMap::new(),
            assertions: BTreeMap::new(),
            evidence: BTreeMap::new(),
        },
        evidence_payloads: BTreeMap::new(),
    };
    World {
        seed,
        context,
        anchor,
        ty,
    }
}
fn open(path: &std::path::Path, w: &World) -> (Commit<SqliteStore>, KernelAuthority) {
    let mut authority = None;
    let kernel = Commit::over_with_authority(w.context, w.anchor.clone(), |held| {
        authority = Some(held.clone());
        Ok(SqliteStore::sqlite(path, "ekr", None)?.under(held))
    })
    .unwrap();
    (kernel, authority.unwrap())
}
fn proposal(w: &World) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: w.context.operator,
        operations: vec![GraphOperation::CreateNode(NodeDraft {
            id: NodeId::mint(),
            root_id: w.seed.graph.root.id,
            type_id: w.ty,
            canonical_name: "a-thing".into(),
            properties: BTreeMap::new(),
        })],
        evidence: BTreeSet::new(),
    }
}
fn validated(
    kernel: &Commit<SqliteStore>,
    w: &World,
    tx: &GraphTransaction,
) -> ValidationReceiptV1 {
    #[derive(serde::Serialize)]
    struct Document<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let bytes = serde_yaml_ng::to_string(&Document {
        format: "ekr.transaction-document/1",
        transaction: tx,
    })
    .unwrap();
    kernel
        .propose(bytes.as_bytes(), w.context.operator, || {
            Timestamp::from_millis(10)
        })
        .unwrap();
    let ValidationCommandResult::Validated(receipt) = kernel
        .validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(20))
        .unwrap()
    else {
        panic!("fixture proposal refused")
    };
    receipt
}
#[test]
fn a_validated_transaction_commits_and_the_lineage_advances() {
    let dir = tempfile::tempdir().unwrap();
    let w = world();
    let (kernel, _) = open(&dir.path().join("state.db"), &w);
    kernel.seed(w.seed.clone(), || Timestamp::EPOCH).unwrap();
    let tx = proposal(&w);
    validated(&kernel, &w, &tx);
    let CommitCommandResult::Committed(receipt) = kernel
        .commit(tx.id, w.context.operator, || Timestamp::from_millis(30))
        .unwrap()
    else {
        panic!("stale")
    };
    assert_eq!(receipt.result.revision, RevisionNumber::new(1));
    assert_eq!(kernel.head().unwrap(), Some(receipt.result));
    let GraphOperation::CreateNode(created) = &tx.operations[0] else {
        unreachable!()
    };
    assert!(kernel.snapshot().unwrap().nodes.contains_key(&created.id));
    drop(kernel);
    let (reopened, _) = open(&dir.path().join("state.db"), &w);
    assert_eq!(reopened.head().unwrap(), Some(receipt.result));
}
#[test]
fn the_same_lineage_written_by_hand_does_not_advance_anything() {
    let dir = tempfile::tempdir().unwrap();
    let w = world();
    let path = dir.path().join("state.db");
    let (kernel, authority) = open(&path, &w);
    kernel.seed(w.seed.clone(), || Timestamp::EPOCH).unwrap();
    let tx = proposal(&w);
    validated(&kernel, &w, &tx);
    kernel
        .commit(tx.id, w.context.operator, || Timestamp::from_millis(30))
        .unwrap();
    let raw = SqliteStore::sqlite(&path, "ekr", None)
        .unwrap()
        .under(authority.clone());
    let mut history = raw.history().unwrap();
    // A valid event payload and receipt, with the validation occurrence removed, cannot authorize
    // a commit. Re-numbering provider coordinates does not make the missing decision real.
    history.occurrences.remove(2);
    for (index, item) in history.occurrences.iter_mut().enumerate() {
        item.version = index as u64 + 1;
    }
    assert!(
        authority.replay(&history, None, None).is_err(),
        "a bare committed fact must not create authority or fall back to the seed"
    );
    let no_authority = SqliteStore::sqlite(&path, "ekr", None).unwrap();
    assert!(matches!(
        no_authority.head(),
        Err(PersistenceError::NoSeedAuthority)
    ));
}
#[test]
fn a_transaction_validated_against_a_revision_the_lineage_moved_past_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let w = world();
    let (kernel, _) = open(&dir.path().join("state.db"), &w);
    kernel.seed(w.seed.clone(), || Timestamp::EPOCH).unwrap();
    let first = proposal(&w);
    let second = proposal(&w);
    validated(&kernel, &w, &first);
    validated(&kernel, &w, &second);
    kernel
        .commit(second.id, w.context.operator, || Timestamp::from_millis(30))
        .unwrap();
    let CommitCommandResult::Stale(stale) = kernel
        .commit(first.id, w.context.operator, || Timestamp::from_millis(30))
        .unwrap()
    else {
        panic!("stale transaction committed")
    };
    assert_eq!(
        stale.expected_basis.previous_root.revision,
        RevisionNumber::SEED
    );
    assert_eq!(stale.observed_root.revision, RevisionNumber::new(1));
    assert_eq!(
        kernel.head().unwrap().unwrap().revision,
        RevisionNumber::new(1)
    );
    assert_eq!(
        kernel.transactions().unwrap()[&first.id].state(),
        TransactionState::Stale
    );
}
#[test]
fn a_commit_into_an_unseeded_lineage_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let w = world();
    let (kernel, _) = open(&dir.path().join("state.db"), &w);
    assert!(matches!(
        kernel.commit(proposal(&w).id, w.context.operator, || panic!(
            "unseeded clock"
        )),
        Err(CommitError::NotSeeded)
    ));
}
#[test]
fn the_kernels_authority_stands_behind_exactly_what_it_validated() {
    let dir = tempfile::tempdir().unwrap();
    let w = world();
    let path = dir.path().join("state.db");
    let (kernel, authority) = open(&path, &w);
    kernel.seed(w.seed.clone(), || Timestamp::EPOCH).unwrap();
    let tx = proposal(&w);
    let original = validated(&kernel, &w, &tx);
    let raw = SqliteStore::sqlite(&path, "ekr", None)
        .unwrap()
        .under(authority.clone());
    let history = raw.history().unwrap();
    assert!(authority.replay(&history, None, None).is_ok());
    for fault in 0..8 {
        let mut receipt = original.clone();
        match fault {
            0 => receipt.proposed_event_id = EventId::mint(),
            1 => receipt.basis.previous_root.revision = RevisionNumber::new(9),
            2 => receipt.validation_hash = ContentHash::of_bytes(b"forged"),
            3 => receipt.basis.previous_record_hash = ContentHash::of_bytes(b"forged"),
            4 => receipt.validators = BTreeSet::from([w.context.operator]),
            5 => receipt.validated_at = Timestamp::from_millis(9),
            6 => receipt.basis.validation_profile_hash = ContentHash::of_bytes(b"forged"),
            7 => receipt.proposal_record_hash = ContentHash::of_bytes(b"forged"),
            _ => unreachable!(),
        };
        let bytes = receipt.to_bytes().unwrap();
        let hash = ContentHash::of_bytes(&bytes);
        let mut forged = history.clone();
        forged.occurrences.last_mut().unwrap().event.record_hash = hash;
        forged.objects.insert(
            hash,
            ekr_store::RetainedObject {
                metadata: ekr_store::StoredObject {
                    content_hash: hash,
                    storage_class: StorageClass::Canonical,
                    byte_len: bytes.len() as u64,
                    stored_at: Timestamp::from_millis(20),
                },
                bytes,
            },
        );
        assert!(
            authority.replay(&forged, None, None).is_err(),
            "altered validated field {fault} authorized replay"
        );
    }
    kernel
        .commit(tx.id, w.context.operator, || Timestamp::from_millis(30))
        .unwrap();
    assert_eq!(
        kernel.head().unwrap().unwrap().revision,
        RevisionNumber::new(1)
    );
}
