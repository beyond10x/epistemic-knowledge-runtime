// Synthetic domain fixtures for complete knowledge-learning journeys.
fn learning_seed() -> SeedDocument {
    let mut initial =
        SeedDocument::from_yaml(include_str!("../fixtures/seed-minimal-v2.yaml")).unwrap();
    let project = NodeType::new(TypeId::mint(), "Project");
    for name in ["Maple", "Birch"] {
        let mut node = Node::new(NodeId::mint(), initial.graph.root.id, project.id, name);
        node.aliases.push(name.into());
        initial.graph.nodes.insert(node.id, node);
    }
    initial.ontology.node_types.push(project);
    initial
}

fn learning_runtime(
    path: &std::path::Path,
    sqlite: bool,
    initial: SeedDocument,
) -> (Runtime, Human) {
    let store = runtime(path, sqlite);
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
    (store, human)
}

fn learning_observation(source_id: &str, payload: &[u8]) -> w::EkrObserveObservationImport {
    let hash = ContentHash::of_bytes(payload);
    let key = ekr_core::ObservationIdempotencyKey {
        source: "manual-learning-demo".into(),
        source_native_id: Some(source_id.into()),
        content_hash: hash,
    };
    serde_json::from_value(serde_json::json!({
        "observation":{"observation_id":key.observation_id(),"source":key.source,"source_native_id":key.source_native_id,
            "content_hash":hash,"captured_at":"1970-01-01T00:00:00.002Z","kind":"FeedItem"},
        "key":{"source":key.source,"source_native_id":key.source_native_id,"content_hash":hash},
        "payload":ekr_core::bytes::encode(payload)
    })).unwrap()
}

fn learning_interpretation(
    observation: &w::EkrObserveObservationImport,
    local_schema: serde_json::Value,
    entities: Vec<serde_json::Value>,
    mut fact: serde_json::Value,
) -> w::EkrIntegrateInterpretationImport {
    let evidence = EvidenceId::mint();
    fact["value"]["evidence"] = serde_json::json!([evidence]);
    let document = serde_json::json!({
        "version":{"interpretation_id":NodeId::mint(),"version":1},
        "root_id":ekr_core::GraphRootId::mint(),
        "observations":[observation.observation.observation_id],
        "local_schema":local_schema,"entities":entities,"facts":[fact],
        "evidence":[{"evidence":{"id":evidence,
            "source":{"kind":"Observation","observation":observation.observation.observation_id},
            "content_hash":observation.observation.content_hash,
            "observed_at":"1970-01-01T00:00:00.002Z","extracted_by":context().operator,"confidence_bp":7500},
            "payload":observation.payload}]
    });
    serde_json::from_value(serde_json::json!({
        "payload":ekr_core::bytes::encode(&serde_json::to_vec_pretty(&document).unwrap()),"document":document
    })).unwrap()
}

fn learning_proposal(
    store: &Runtime,
    sources: &[w::EkrIntegrateInterpretationVersion],
    additions: Vec<serde_json::Value>,
    mappings: Vec<serde_json::Value>,
    corrections: Vec<serde_json::Value>,
    explanation: &str,
) -> w::EkrIntegrateSchemaProposalImport {
    let proposal = serde_json::json!({
        "proposal_id":NodeId::mint(),"base_schema":store.read(None).unwrap().graph.ontology.version().id,
        "sources":sources.iter().map(|version|serde_json::json!({"version":version,"items":["facts[0]"]})).collect::<Vec<_>>(),
        "observations":[],"evidence":[],"additions":additions,"mappings":mappings,
        "corrections":corrections,"explanation":explanation
    });
    serde_json::from_value(serde_json::json!({
        "payload":ekr_core::bytes::encode(&serde_json::to_vec_pretty(&proposal).unwrap()),"proposal":proposal
    })).unwrap()
}

fn approve_learning(
    store: &Runtime,
    human: &Human,
    proposal: &w::EkrIntegrateSchemaProposalImport,
    statement: &[u8],
) -> (
    w::EkrIntegrateSchemaProposalRead,
    w::EkrIntegrateProposalReviewSnapshot,
) {
    let root = store.read(None).unwrap().root;
    let shown = store
        .submit_schema_proposal(proposal, Timestamp::from_millis(3))
        .unwrap();
    assert_eq!(shown.preview.len(), proposal.proposal.mappings.len());
    assert!(
        shown.preview.iter().all(|item| item.blockers.is_empty()),
        "{:?}",
        shown.preview
    );
    assert_eq!(
        store.read(None).unwrap().root,
        root,
        "submission changed canonical knowledge"
    );
    let approved = store
        .approve_schema_proposal(
            &signed_review(human, &shown, true, ekr_core::EventId::mint(), statement),
            Timestamp::from_millis(4),
        )
        .unwrap();
    assert_eq!(
        store.read(None).unwrap().root,
        root,
        "approval changed canonical knowledge"
    );
    (shown, approved)
}

fn assert_mapped_source(
    read: &ekr_kernel::VerifiedRead,
    assertion: AssertionId,
    version: &w::EkrIntegrateInterpretationVersion,
    interpretation: &w::EkrIntegrateInterpretationImport,
    observation: &w::EkrObserveObservationImport,
) {
    let explained = read.explain(assertion).unwrap();
    assert!(explained.links.iter().any(|link| matches!(link,
        ekr_kernel::ExplanationLink::Mapping(mapping)
            if mapping.source_document_digest == version.document_digest
                && mapping.mapping.source.as_ref() == version
                && mapping.mapping.source_item == "facts[0]")));
    assert!(explained.links.iter().any(|link| matches!(link,
        ekr_kernel::ExplanationLink::Derivation(derivation)
            if derivation.assertion_id.0 == assertion.to_string()
                && derivation.observation_id == w::EssPresence::Present(observation.observation.observation_id.clone()))));
    assert!(explained.links.iter().any(|link| matches!(link,
        ekr_kernel::ExplanationLink::Evidence(evidence)
            if matches!(evidence.source, EvidenceSource::Observation(id) if id.to_string() == observation.observation.observation_id.0)
                && evidence.content_hash.to_string() == observation.observation.content_hash.0)));
    let source_hash: ContentHash = version.document_digest.0.parse().unwrap();
    let payload_hash: ContentHash = observation.observation.content_hash.0.parse().unwrap();
    assert_eq!(
        read.content(&source_hash).unwrap(),
        ekr_core::bytes::decode(&interpretation.payload).unwrap()
    );
    assert_eq!(
        read.content(&payload_hash).unwrap(),
        ekr_core::bytes::decode(&observation.payload).unwrap()
    );
}

fn assert_repeat_does_not_write(
    store: &Runtime,
    shown: &w::EkrIntegrateSchemaProposalRead,
    approval: &w::EkrIntegrateProposalReviewSnapshot,
    original: &w::EkrIntegrateApplicationReport,
) {
    let events = store.published_events().unwrap();
    let root = store.read(None).unwrap().root;
    let retained = store.schema_proposal(&shown.proposal.proposal_id).unwrap();
    let repeated = store
        .apply_schema_proposal(
            &shown.proposal.proposal_id,
            &approval.review_id,
            &shown.proposal_digest,
            Timestamp::from_millis(6),
        )
        .unwrap();
    assert!(repeated.already_complete);
    assert_eq!(repeated.application_id, original.application_id);
    assert_eq!(repeated.schema_transaction, original.schema_transaction);
    assert_eq!(repeated.receipt_id, original.receipt_id);
    assert_eq!(*repeated.progress, w::EkrIntegrateApplicationProgress::V0);
    assert_eq!(repeated.items, original.items);
    assert_eq!(
        store.published_events().unwrap(),
        events,
        "repeat wrote retained or canonical events"
    );
    assert_eq!(store.read(None).unwrap().root, root);
    assert_eq!(
        store.schema_proposal(&shown.proposal.proposal_id).unwrap(),
        retained
    );
}
