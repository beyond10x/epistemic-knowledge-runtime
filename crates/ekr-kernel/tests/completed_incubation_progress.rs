//! A mapping completes one exact retained source item, independently of import-time receipts.
#![allow(unused_imports, dead_code)]
include!("support/schema_proposal_fixture.rs");
include!("support/knowledge_learning_fixtures.rs");

#[test]
fn completed_items_leave_discovery_and_attention_without_hiding_other_items_or_versions() {
    use ekr_core::Canonical;
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let (store, human) = learning_runtime(&path, sqlite, learning_seed());
        let observation = learning_observation("Maple", b"Maple health is amber.");
        store
            .import_observation(&observation, Timestamp::from_millis(2))
            .unwrap();
        let reference = serde_json::json!({"node_type":"Project","aliases":["Maple"]});
        let schema = serde_json::json!({"node_types":[{"name":"Project","parents":[],"abstract_type":false,
            "properties":[{"name":"health","value":{"value_kind":"String"},"required":false,"cardinality":"One"}]}],"edge_types":[]});
        let fact = serde_json::json!({"kind":"Property","value":{"subject":reference,"property":"health",
            "value":{"kind":"String","canonical_bytes":ekr_core::bytes::encode(&ekr_graph::CanonicalValue::String("amber".into()).canonical_bytes())}}});
        let mut source = learning_interpretation(&observation, schema, vec![reference], fact);
        source.document.facts.push(source.document.facts[0].clone());
        source.payload = ekr_core::bytes::encode(&serde_json::to_vec(&source.document).unwrap());
        let first = store
            .import_interpretation(&source, Timestamp::from_millis(2))
            .unwrap();
        let original = store.interpretation(&first.version).unwrap();
        source.document.version.version = 2.into();
        source.document.facts.truncate(1);
        source.payload = ekr_core::bytes::encode(&serde_json::to_vec(&source.document).unwrap());
        let second = store
            .import_interpretation(&source, Timestamp::from_millis(2))
            .unwrap();
        let other = store.interpretation(&second.version).unwrap();
        let completed = original
            .blockers
            .iter()
            .find(|b| b.item == "facts[0]")
            .unwrap()
            .blocker_id
            .0
            .clone();
        let expected: BTreeSet<_> = original
            .blockers
            .iter()
            .filter(|b| b.item != "facts[0]")
            .chain(other.blockers.iter())
            .map(|b| b.blocker_id.0.clone())
            .collect();
        assert_eq!(expected.len(), 2);
        let proposal = learning_proposal(
            &store,
            &[*first.version.clone()],
            vec![serde_json::json!({
                "kind":"AddOptionalProperty","value":{"owner_type":"Project","property":{"name":"health","value":{"value_kind":"String"},"required":false,"cardinality":"One"}}
            })],
            vec![
                serde_json::json!({"source":first.version,"source_item":"facts[0]","source_type":"Project","target_type":"Project","target_member":"health",
            "value":{"kind":"CopyField","value":{"declaration":"Project","field":"health"}}}),
            ],
            vec![],
            "Integrate only the first selected item of the first immutable version.",
        );
        let (shown, approval) = approve_learning(
            &store,
            &human,
            &proposal,
            b"Approve only the exact first source item.",
        );
        let before: BTreeSet<_> = store
            .discover_schema_gaps()
            .unwrap()
            .groups
            .into_iter()
            .flat_map(|g| g.blockers)
            .map(|b| b.0)
            .collect();
        assert!(before.contains(&completed));
        store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approval.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(5),
            )
            .unwrap();
        let after = store.schema_proposal(&shown.proposal.proposal_id).unwrap();
        let w::EssPresence::Present(application) = after.application else {
            panic!("application");
        };
        assert!(application.publications.iter().any(|p|matches!(&p.guard.step.item,w::EssPresence::Present(item) if item.source == first.version && item.item == "facts[0]")));
        drop(store);
        let mut reopened = runtime(&path, sqlite)
            .with_review_authority(human.binding)
            .unwrap();
        reopened.set_full_replay(true);
        assert_eq!(reopened.read(None).unwrap().graph.assertions.len(), 1);
        let events = reopened.published_events().unwrap();
        let gaps: BTreeSet<_> = reopened
            .discover_schema_gaps()
            .unwrap()
            .groups
            .into_iter()
            .flat_map(|g| g.blockers)
            .map(|b| b.0)
            .collect();
        let questions: BTreeSet<_> = reopened
            .attention()
            .unwrap()
            .into_iter()
            .filter_map(|item| match item.subject.blocker_id {
                w::EssPresence::Present(id) => Some(id.0),
                w::EssPresence::Absent => None,
            })
            .collect();
        assert_eq!(
            reopened.interpretation(&first.version).unwrap().blockers,
            original.blockers,
            "immutable findings were overwritten"
        );
        assert_eq!(
            (&gaps, &questions),
            (&expected, &expected),
            "{sqlite}: completed item {completed} still visible or unselected items hidden"
        );
        assert_eq!(
            reopened.published_events().unwrap(),
            events,
            "reading progress wrote events"
        );
    }
}
