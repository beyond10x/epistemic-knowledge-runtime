//! Public schema-evidence reads refuse an incomplete immutable history.
#[path = "support/upgrade_fixture.rs"]
#[allow(dead_code)]
mod fixture;
use ekr_core::{RevisionNumber, Timestamp, TransactionId};
use ekr_kernel::{human_review, CommitCommandResult, Runtime, ValidationCommandResult};
use ekr_sdk::document::{NodeType, TransactionBuilder};

#[test]
fn schema_evidence_projection_requires_both_immutable_schema_and_transaction() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let store = if sqlite {
            Runtime::sqlite(
                &path,
                fixture::TENANT,
                fixture::context(),
                fixture::anchor(),
            )
        } else {
            Runtime::file(
                &path,
                fixture::TENANT,
                fixture::context(),
                fixture::anchor(),
            )
        }
        .unwrap();
        let seeded = store
            .seed(fixture::seed(false), || Timestamp::EPOCH)
            .unwrap();
        let human = fixture::Human::new(seeded.seed_hash);
        let store = store.with_review_authority(human.binding.clone()).unwrap();
        let preview = store.preview_upgrade(&human.policy).unwrap();
        let proof = human_review::proof_from_document(&human.proof(&preview)).unwrap();
        store
            .apply_upgrade(&preview, &human.policy, &proof, fixture::STATEMENT, || {
                Timestamp::from_millis(1)
            })
            .unwrap();
        let transaction = TransactionId::mint();
        let support = fixture::id(500);
        let document = TransactionBuilder::new(fixture::context().operator)
            .with_id(transaction)
            .with_schema_version(ekr_core::SchemaVersionId::mint())
            .with_schema_evidence([support])
            .push(NodeType::new("ReviewedSchemaType").into())
            .build()
            .unwrap();
        store
            .propose(
                document.to_yaml().unwrap().as_bytes(),
                fixture::context().operator,
                || Timestamp::from_millis(2),
            )
            .unwrap();
        assert!(matches!(
            store
                .validate(transaction, store.read(None).unwrap().root.revision, || {
                    Timestamp::from_millis(3)
                })
                .unwrap(),
            ValidationCommandResult::Validated(_)
        ));
        assert!(matches!(
            store
                .commit(transaction, fixture::context().operator, || {
                    Timestamp::from_millis(4)
                })
                .unwrap(),
            CommitCommandResult::Committed(_)
        ));
        let revision = store.read(None).unwrap().root.revision;
        let history = store.schema_history(revision).unwrap();
        let projected = ekr_views::schema_evidence(&history).unwrap();
        assert_eq!(projected.len(), 1);
        assert_eq!(projected[0].revision.0.as_u64(), Some(revision.get()));
        assert_eq!(projected[0].transaction_id.0, transaction.to_string());
        assert_eq!(projected[0].evidence.len(), 1);
        assert_eq!(projected[0].evidence[0].0, support.to_string());
        for omit_schema in [false, true] {
            let mut missing = history.clone();
            if omit_schema {
                missing.schemas.remove(&revision);
            } else {
                missing.transactions.remove(&revision);
            }
            assert!(ekr_views::schema_evidence(&missing).is_err());
        }
        let historical = store.schema_history(RevisionNumber::SEED).unwrap();
        assert!(ekr_views::schema_evidence(&historical).unwrap().is_empty());
    }
}
