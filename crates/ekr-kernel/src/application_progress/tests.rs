//! Receipt-prefix regression over actual provider transactions and cold retained replay.
// The shared fixture also provides helpers for other review tests, unused by this unit.
#[allow(unused_imports, dead_code)]
mod fixture {
    use crate as ekr_kernel;
    include!("../../tests/support/schema_proposal_fixture.rs");

    fn source(store: &Runtime) -> w::EkrIntegrateInterpretationImport {
        use ekr_core::Canonical;
        let read = store.read(None).unwrap();
        let evidence = read.graph.evidence.values().next().unwrap();
        let payload = read.content(&evidence.content_hash).unwrap();
        let reference = serde_json::json!({"node_type":"Project","aliases":["Maple"]});
        let document = serde_json::json!({
            "version":{"interpretation_id":NodeId::mint(),"version":1},
            "root_id":ekr_core::GraphRootId::mint(),"observations":[],
            "local_schema":{"node_types":[{"name":"Project","parents":[],"abstract_type":false,
                "properties":[{"name":"health","value":{"value_kind":"String"},"required":false,"cardinality":"One"}]}],"edge_types":[]},
            "entities":[reference],
            "facts":[{"kind":"Property","value":{"subject":reference,"property":"health",
                "value":{"kind":"String","canonical_bytes":ekr_core::bytes::encode(&ekr_graph::CanonicalValue::String("amber".into()).canonical_bytes())},
                "evidence":[evidence.id]}}],
            "evidence":[{"evidence":{"id":evidence.id,"source":{"kind":"HumanStatement"},
                "content_hash":evidence.content_hash,"observed_at":"1970-01-01T00:00:00Z",
                "extracted_by":evidence.extracted_by,"confidence_bp":10000},"payload":ekr_core::bytes::encode(payload)}]
        });
        serde_json::from_value(serde_json::json!({
            "payload":ekr_core::bytes::encode(&serde_json::to_vec_pretty(&document).unwrap()),
            "document":document
        }))
        .unwrap()
    }

    fn captured(
        path: &std::path::Path,
        sqlite: bool,
        human: &Human,
    ) -> (crate::KernelAuthority, ekr_store::RetainedHistory) {
        let mut captured = None;
        if sqlite {
            Commit::over_with_review_authority(
                context(),
                anchor(),
                human.binding.clone(),
                |authority| {
                    let store = SqliteStore::sqlite(path, "upgrade-fixture", None)?
                        .under(authority.clone());
                    captured = Some((authority, store.history()?));
                    Ok(store)
                },
            )
            .unwrap();
        } else {
            Commit::over_with_review_authority(
                context(),
                anchor(),
                human.binding.clone(),
                |authority| {
                    let store =
                        FileStore::file(path, "upgrade-fixture", None)?.under(authority.clone());
                    captured = Some((authority, store.history()?));
                    Ok(store)
                },
            )
            .unwrap();
        }
        captured.unwrap()
    }

    #[test]
    fn older_receipt_reports_only_its_mapping_prefix_after_later_commits() {
        for sqlite in [false, true] {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path().join("store");
            let store = runtime(&path, sqlite);
            let mut initial = seed();
            initial
                .graph
                .nodes
                .values_mut()
                .next()
                .unwrap()
                .aliases
                .push("Maple".into());
            let seeded = store.seed(initial, || Timestamp::EPOCH).unwrap();
            let human = schema_human(seeded.seed_hash);
            let store = store.with_review_authority(human.binding.clone()).unwrap();
            let preview = store.preview_upgrade(&human.policy).unwrap();
            store
                .apply_upgrade(
                    &preview,
                    &human.policy,
                    &human.proof(&preview),
                    b"reviewed contradictions and pending validations",
                    || Timestamp::from_millis(1),
                )
                .unwrap();
            let imported = store
                .import_interpretation(&source(&store), Timestamp::from_millis(2))
                .unwrap();
            let mut proposal = proposal_input(&store);
            proposal.proposal.evidence.clear();
            proposal.proposal.sources = vec![Box::new(
                serde_json::from_value(serde_json::json!({
                    "version":imported.version,"items":["facts[0]"]
                }))
                .unwrap(),
            )];
            // Two distinct qualified mapping items share one source fact. Separate targets
            // avoid a contradiction and make the second mapping an independent later commit.
            proposal.proposal.additions.clear();
            for target in ["reported_health", "recorded_health"] {
                proposal.proposal.additions.push(Box::new(serde_json::from_value(serde_json::json!({
                    "kind":"AddOptionalProperty","value":{"owner_type":"Project","property":{
                        "name":target,"value":{"value_kind":"String"},"required":false,"cardinality":"One"}}
                })).unwrap()));
                proposal.proposal.mappings.push(Box::new(serde_json::from_value(serde_json::json!({
                    "source":imported.version,"source_item":"facts[0]","source_type":"Project",
                    "target_type":"Project","target_member":target,
                    "value":{"kind":"CopyField","value":{"declaration":"Project","field":"health"}}
                })).unwrap()));
            }
            proposal.payload =
                ekr_core::bytes::encode(&serde_json::to_vec(&proposal.proposal).unwrap());
            let shown = store
                .submit_schema_proposal(&proposal, Timestamp::from_millis(3))
                .unwrap();
            assert!(shown.preview.iter().all(|item| item.blockers.is_empty()));
            let approved = store
                .approve_schema_proposal(
                    &signed_review(
                        &human,
                        &shown,
                        true,
                        ekr_core::EventId::mint(),
                        b"Approve both independent health mappings.",
                    ),
                    Timestamp::from_millis(4),
                )
                .unwrap();
            let completed = store
                .apply_schema_proposal(
                    &shown.proposal.proposal_id,
                    &approved.review_id,
                    &shown.proposal_digest,
                    Timestamp::from_millis(5),
                )
                .unwrap();
            assert_eq!(*completed.progress, w::EkrIntegrateApplicationProgress::V0);
            assert_eq!(completed.items.len(), 2);
            drop(store);

            // The state and all processing identities come from cold replay of real retained
            // canonical commits, not a constructed ReplayState or invented publication.
            let (authority, history) = captured(&path, sqlite, &human);
            let state = authority.reconstruct_in_full(&history).unwrap().unwrap();
            let election = &history.applications.elections()[0];
            let mut commits: Vec<_> = history
                .applications
                .processing_receipts()
                .iter()
                .map(|receipt| {
                    let w::EssPresence::Present(tx) = &receipt.transaction_id else {
                        panic!("missing transaction")
                    };
                    let tx: TransactionId = tx.0.parse().unwrap();
                    state.transactions[&tx]
                        .committed
                        .as_ref()
                        .unwrap()
                        .result
                        .revision
                })
                .collect();
            commits.sort();
            assert_eq!(commits.len(), 2);
            assert!(commits[0] < commits[1]);
            assert_eq!(commits[1], state.head().root.revision);

            // Reconstruct the exact older receipt projection, including its already retained
            // processing receipt ID, at the first mapping commit's verified revision.
            let older =
                crate::application_progress::snapshot(&history, &state, election, commits[0])
                    .unwrap()
                    .unwrap();
            assert_eq!(older.processing_receipts.len(), 1);
            assert_eq!(older.remaining_items.len(), 1);
            assert_eq!(*older.progress, w::EkrIntegrateApplicationProgress::V2);
            crate::application_progress::verify_receipt(&history, &state, election, &older)
                .unwrap();
            let report =
                crate::application_progress::report(&history, &state, election, &older, false)
                    .unwrap();
            assert_eq!(report.progress, older.progress);
            assert_eq!(report.remaining_items, older.remaining_items);
            assert_eq!(
                report.receipt_id,
                w::EssPresence::Present(older.receipt_id.clone())
            );
            let remaining = &older.remaining_items[0];
            let pending = report
                .items
                .iter()
                .find(|item| item.mapping_digest == remaining.mapping_digest)
                .unwrap();
            assert_eq!(pending.source, remaining.source);
            assert_eq!(pending.item, remaining.item);
            assert_eq!(
                *pending.disposition,
                w::EkrIntegrateProcessingDisposition::V2
            );
            assert_eq!(pending.transaction_id, w::EssPresence::Absent);
            assert!(pending.assertions.is_empty());
            assert_eq!(
                report
                    .items
                    .iter()
                    .filter(|item| matches!(
                        *item.disposition,
                        w::EkrIntegrateProcessingDisposition::V1
                    ))
                    .count(),
                1
            );

            let latest = history
                .applications
                .receipts()
                .iter()
                .find(|receipt| matches!(*receipt.progress, w::EkrIntegrateApplicationProgress::V0))
                .unwrap();
            let current =
                crate::application_progress::report(&history, &state, election, latest, true)
                    .unwrap();
            assert!(current.already_complete);
            assert!(current.remaining_items.is_empty());
            assert!(current.items.iter().all(|item| matches!(
                *item.disposition,
                w::EkrIntegrateProcessingDisposition::V1
            ) && matches!(
                item.transaction_id,
                w::EssPresence::Present(_)
            ) && item.assertions.len() == 1));
            // Refusing inconsistent receipt fields is a separate control on the new report
            // validation call; merely limiting item reads to an arbitrary revision is insufficient.
            let mut altered = older;
            altered.remaining_items.clear();
            assert!(crate::application_progress::report(
                &history, &state, election, &altered, false
            )
            .is_err());
        }
    }
}
