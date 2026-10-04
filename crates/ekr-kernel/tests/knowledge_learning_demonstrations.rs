//! Complete operator journeys through real retention, signed review and canonical application.
// Shared review fixture also declares helpers used only by its other integration tests.
#![allow(unused_imports, dead_code)]
include!("support/schema_proposal_fixture.rs");
include!("support/knowledge_learning_fixtures.rs");

#[test]
fn recurring_parked_project_health_becomes_reviewed_vocabulary_and_integrated_knowledge() {
    use ekr_core::Canonical;
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let (store, human) = learning_runtime(&path, sqlite, learning_seed());
        let initial = store.read(None).unwrap();
        assert!(initial.graph.assertions.is_empty());
        let mut observations = vec![];
        let mut documents = vec![];
        let mut versions = vec![];
        for (project, value) in [("Maple", "amber"), ("Birch", "green")] {
            let observation = learning_observation(
                project,
                format!("{project} project health is {value}.").as_bytes(),
            );
            store
                .import_observation(&observation, Timestamp::from_millis(2))
                .unwrap();
            let reference = serde_json::json!({"node_type":"Project","aliases":[project]});
            let local_schema = serde_json::json!({"node_types":[{"name":"Project","parents":[],"abstract_type":false,
                "properties":[{"name":"health","value":{"value_kind":"String"},"required":false,"cardinality":"One"}]}],"edge_types":[]});
            let fact = serde_json::json!({"kind":"Property","value":{"subject":reference,"property":"health",
                "value":{"kind":"String","canonical_bytes":ekr_core::bytes::encode(&ekr_graph::CanonicalValue::String(value.into()).canonical_bytes())}}});
            let document =
                learning_interpretation(&observation, local_schema, vec![reference], fact);
            let imported = store
                .import_interpretation(&document, Timestamp::from_millis(2))
                .unwrap();
            let retained = store.interpretation(&imported.version).unwrap();
            assert!(retained
                .blockers
                .iter()
                .any(|blocker| blocker.item == "facts[0]"
                    && matches!(*blocker.kind, w::EkrIntegrateIntegrationBlockerKind::V2)));
            assert_eq!(retained.document, document.document);
            observations.push(observation);
            documents.push(document);
            versions.push(*imported.version);
        }
        assert_eq!(store.read(None).unwrap().root, initial.root);
        let before_discovery = store.published_events().unwrap();
        let request = store.discover_schema_gaps().unwrap();
        assert_eq!(request.groups.len(), 1);
        let group = &request.groups[0];
        assert_eq!(group.declaration, "Project.health");
        assert_eq!(*group.kind, w::EkrIntegrateIntegrationBlockerKind::V2);
        assert_eq!(group.sources.len(), 2);
        assert_eq!(group.observations.len(), 2);
        assert_eq!(group.blockers.len(), 2);
        assert_eq!(request.evidence.len(), 2);
        assert_eq!(store.discover_schema_gaps().unwrap(), request);
        assert_eq!(store.published_events().unwrap(), before_discovery);
        drop(store);

        // Parked documents and independent observations survive a genuine cold full replay
        // before any human proposal exists.
        let mut store = runtime(&path, sqlite)
            .with_review_authority(human.binding.clone())
            .unwrap();
        store.set_full_replay(true);
        assert_eq!(store.discover_schema_gaps().unwrap(), request);
        assert_eq!(store.read(None).unwrap().root, initial.root);
        for index in 0..2 {
            assert_eq!(
                store.interpretation(&versions[index]).unwrap().document,
                documents[index].document
            );
            assert_eq!(
                store
                    .observation(
                        observations[index]
                            .observation
                            .observation_id
                            .0
                            .parse()
                            .unwrap()
                    )
                    .unwrap()
                    .payload,
                observations[index].payload
            );
        }
        let additions = vec![
            serde_json::json!({"kind":"AddOptionalProperty","value":{"owner_type":"Project",
            "property":{"name":"health","value":{"value_kind":"String"},"required":false,"cardinality":"One"}}}),
        ];
        let mappings = versions.iter().map(|source|serde_json::json!({
            "source":source,"source_item":"facts[0]","source_type":"Project","target_type":"Project","target_member":"health",
            "value":{"kind":"CopyField","value":{"declaration":"Project","field":"health"}}
        })).collect();
        let proposal = learning_proposal(&store, &versions, additions, mappings, vec![],
            "Repeated retained health observations justify an optional String property. Copy each explicitly selected health field.");
        let (shown, approval) = approve_learning(
            &store,
            &human,
            &proposal,
            b"Approve optional Project.health and the exact two source-field mappings.",
        );
        let report = store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approval.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(5),
            )
            .unwrap();
        assert_eq!(
            *report.progress,
            w::EkrIntegrateApplicationProgress::V0,
            "{report:?}"
        );
        assert!(!report.already_complete);
        assert!(report.remaining_items.is_empty());
        assert_eq!(report.items.len(), 2);
        let final_read = store.read(None).unwrap();
        assert_eq!(
            final_read.root.revision.get(),
            initial.root.revision.get() + 3
        );
        assert_eq!(final_read.graph.assertions.len(), 2);
        let project_type = final_read
            .graph
            .ontology
            .to_document()
            .node_types
            .into_iter()
            .find(|t| t.name == "Project")
            .unwrap();
        let property = project_type
            .properties
            .values()
            .find(|p| p.name == "health")
            .unwrap();
        assert!(!property.required);
        assert_eq!(property.value_type, ValueType::String);
        assert!(store.discover_schema_gaps().unwrap().groups.is_empty());
        let mut claims = vec![];
        for (index, (project, value)) in [("Maple", "amber"), ("Birch", "green")]
            .into_iter()
            .enumerate()
        {
            let node = final_read
                .graph
                .nodes
                .values()
                .find(|n| n.canonical_name == project)
                .unwrap();
            let claim = final_read
                .graph
                .assertions
                .values()
                .find(|a| a.subject == Subject::Node(ekr_graph::CanonicalRef::new(node.id)))
                .unwrap();
            assert_eq!(claim.predicate, Predicate::Property(property.id));
            assert_eq!(
                claim.object,
                Object::Value(ekr_graph::CanonicalValue::String(value.into()))
            );
            assert!(matches!(claim.assessment, Assessment::Accepted { .. }));
            assert_mapped_source(
                &final_read,
                claim.id,
                &versions[index],
                &documents[index],
                &observations[index],
            );
            claims.push(claim.id);
        }
        assert_repeat_does_not_write(&store, &shown, &approval, &report);
        let events = store.published_events().unwrap();
        drop(store);
        let mut reopened = runtime(&path, sqlite)
            .with_review_authority(human.binding.clone())
            .unwrap();
        reopened.set_full_replay(true);
        let cold = reopened.read(None).unwrap();
        assert_eq!(cold.root, final_read.root);
        assert_eq!(cold.graph.assertions, final_read.graph.assertions);
        for index in 0..2 {
            assert_eq!(
                cold.explain(claims[index]).unwrap(),
                final_read.explain(claims[index]).unwrap()
            );
            assert_mapped_source(
                &cold,
                claims[index],
                &versions[index],
                &documents[index],
                &observations[index],
            );
            reopened
                .import_observation(&observations[index], Timestamp::from_millis(7))
                .unwrap();
            reopened
                .import_interpretation(&documents[index], Timestamp::from_millis(7))
                .unwrap();
        }
        assert_repeat_does_not_write(&reopened, &shown, &approval, &report);
        assert_eq!(
            reopened.published_events().unwrap(),
            events,
            "reimports or resumed application duplicated writes"
        );
    }
}

#[test]
fn ownership_question_becomes_explicit_relations_and_explained_correction_without_losing_history() {
    for sqlite in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let mut initial = learning_seed();
        let project_type = initial.ontology.node_types[0].id;
        let project = initial
            .graph
            .nodes
            .values()
            .find(|n| n.canonical_name == "Maple")
            .unwrap()
            .id;
        let team_type = NodeType::new(TypeId::mint(), "Team");
        let mut broad = ekr_ontology::EdgeType::new(TypeId::mint(), "owned_by");
        broad.source_types.insert(project_type);
        broad.target_types.insert(team_type.id);
        broad.cardinality = Cardinality::One;
        let mut teams = vec![];
        let mut old_claims = vec![];
        for (team, payload) in [
            ("Operations", b"Maple is owned by Operations.".as_slice()),
            ("Business", b"Maple is owned by Business.".as_slice()),
        ] {
            let mut node = Node::new(NodeId::mint(), initial.graph.root.id, team_type.id, team);
            node.aliases.push(team.into());
            let evidence = Evidence {
                id: EvidenceId::mint(),
                source: EvidenceSource::HumanStatement { identity: None },
                content_hash: ContentHash::of_bytes(payload),
                extracted_by: context().operator,
                observed_at: Timestamp::EPOCH,
                confidence: Confidence::CERTAIN,
            };
            let claim = Assertion {
                id: AssertionId::mint(),
                root_id: initial.graph.root.id,
                subject: Subject::Node(project),
                predicate: Predicate::Relation(broad.id),
                object: Object::Node(node.id),
                evidence: [evidence.id].into(),
                proposed_by: context().operator,
                assessment: Assessment::Proposed,
                lifecycle: AssertionLifecycle::Active,
                valid_time: TemporalRange::UNBOUNDED,
                transaction_time: TransactionTime::since(Timestamp::EPOCH),
            };
            initial
                .evidence_payloads
                .insert(evidence.content_hash, payload.to_vec().into());
            initial.graph.evidence.insert(evidence.id, evidence);
            old_claims.push(claim.id);
            initial.graph.assertions.insert(claim.id, claim);
            teams.push(node.id);
            initial.graph.nodes.insert(node.id, node);
        }
        initial.ontology.node_types.push(team_type);
        initial.ontology.edge_types.push(broad);
        let (store, human) = learning_runtime(&path, sqlite, initial);
        let disputed = store.read(None).unwrap();
        for id in &old_claims {
            assert!(matches!(
                disputed.graph.assertions[id].assessment,
                Assessment::Disputed { .. }
            ));
        }
        let questions = store.attention().unwrap();
        let question = questions
            .iter()
            .find(|question| {
                question.claims.len() == 2
                    && question
                        .claims
                        .iter()
                        .all(|id| old_claims.contains(&id.0.parse().unwrap()))
            })
            .unwrap();
        assert!(!question.question.trim().is_empty());
        for id in &old_claims {
            for evidence in &disputed.graph.assertions[id].evidence {
                assert!(question
                    .evidence
                    .iter()
                    .any(|id| id.0 == evidence.id().to_string()));
            }
        }
        assert_eq!(store.attention_item(&question.subject).unwrap(), *question);
        let mut observations = vec![];
        let mut documents = vec![];
        let mut versions = vec![];
        for team in ["Operations", "Business"] {
            let observation =
                learning_observation(team, format!("Maple is owned by {team}.").as_bytes());
            store
                .import_observation(&observation, Timestamp::from_millis(2))
                .unwrap();
            let subject = serde_json::json!({"node_type":"Project","aliases":["Maple"]});
            let object = serde_json::json!({"node_type":"Team","aliases":[team]});
            let local_schema = serde_json::json!({"node_types":[
                {"name":"Project","parents":[],"abstract_type":false,"properties":[]},
                {"name":"Team","parents":[],"abstract_type":false,"properties":[]}],
                "edge_types":[{"name":"owned_by","source_types":["Project"],"target_types":["Team"],"cardinality":"One","properties":[]}]});
            let fact = serde_json::json!({"kind":"Relation","value":{"subject":subject,"relation":"owned_by","object":object}});
            let document =
                learning_interpretation(&observation, local_schema, vec![subject, object], fact);
            let imported = store
                .import_interpretation(&document, Timestamp::from_millis(2))
                .unwrap();
            observations.push(observation);
            documents.push(document);
            versions.push(*imported.version);
        }
        let relation_names = ["operationally_owned_by", "business_owned_by"];
        let additions = relation_names.iter().map(|name|serde_json::json!({"kind":"DefineRelation","value":{
            "name":name,"source_types":["Project"],"target_types":["Team"],"cardinality":"One","properties":[]
        }})).collect();
        let mappings = versions.iter().zip(relation_names).map(|(source,target)|serde_json::json!({
            "source":source,"source_item":"facts[0]","source_type":"Project","target_type":"Project","target_member":target,
            "value":{"kind":"CopyRelation","value":{"declaration":"Project","relation":"owned_by"}}
        })).collect();
        let corrections = old_claims.iter().map(|id|serde_json::json!({
            "kind":"Retract","assertion_id":id,
            "reason":"The broad ownership claim is ambiguous; retain its history and replace its meaning with the reviewed operational and business relations."
        })).collect();
        let proposal = learning_proposal(&store, &versions, additions, mappings, corrections,
            "Operations owns daily operation; Business owns business decisions. Both sources were accurate in different senses. Make those two roles explicit and retire the ambiguous claims.");
        let (shown, approval) = approve_learning(&store, &human, &proposal,
            b"Operations is the operational owner and Business the business owner. Approve these exact relation additions, mappings and retractions of both ambiguous ownership claims.");
        assert_eq!(store.read(None).unwrap().root, disputed.root);
        let report = store
            .apply_schema_proposal(
                &shown.proposal.proposal_id,
                &approval.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(5),
            )
            .unwrap();
        assert_eq!(
            *report.progress,
            w::EkrIntegrateApplicationProgress::V0,
            "{report:?}"
        );
        assert!(!report.corrections_pending);
        assert!(report.remaining_items.is_empty());
        let final_read = store.read(None).unwrap();
        assert_eq!(
            final_read.root.revision.get(),
            disputed.root.revision.get() + 4,
            "schema, two mappings, then one final correction transaction"
        );
        assert!(final_read.dispute_attention().unwrap().is_empty());
        assert!(store
            .attention()
            .unwrap()
            .iter()
            .all(|item| item.subject.dispute_id != question.subject.dispute_id));
        assert_eq!(final_read.graph.assertions.len(), 4);
        let ontology = final_read.graph.ontology.to_document();
        let mut refined = vec![];
        for (index, name) in relation_names.into_iter().enumerate() {
            let relation = ontology.edge_types.iter().find(|t| t.name == name).unwrap();
            assert_eq!(relation.cardinality, Cardinality::One);
            let claim = final_read
                .graph
                .assertions
                .values()
                .find(|a| a.predicate == Predicate::Relation(relation.id))
                .unwrap();
            assert_eq!(
                claim.subject,
                Subject::Node(ekr_graph::CanonicalRef::new(project))
            );
            assert_eq!(
                claim.object,
                Object::Node(ekr_graph::CanonicalRef::new(teams[index]))
            );
            assert!(matches!(claim.assessment, Assessment::Accepted { .. }));
            assert!(claim.is_current());
            assert_mapped_source(
                &final_read,
                claim.id,
                &versions[index],
                &documents[index],
                &observations[index],
            );
            refined.push(claim.id);
        }
        for id in &old_claims {
            let old = &disputed.graph.assertions[id];
            let retained = &final_read.graph.assertions[id];
            assert_eq!(retained.subject, old.subject);
            assert_eq!(retained.predicate, old.predicate);
            assert_eq!(retained.object, old.object);
            assert_eq!(retained.evidence, old.evidence);
            assert!(matches!(
                retained.lifecycle,
                AssertionLifecycle::Retracted { .. }
            ));
            let explanation = final_read.explain(*id).unwrap();
            assert!(explanation
                .links
                .iter()
                .any(|link| matches!(link, ekr_kernel::ExplanationLink::SchemaCorrection(_))));
            for evidence in &old.evidence {
                let evidence = &disputed.graph.evidence[&evidence.id()];
                assert_eq!(
                    final_read.content(&evidence.content_hash),
                    disputed.content(&evidence.content_hash)
                );
            }
        }
        let historical = store.read(Some(disputed.root.revision)).unwrap();
        assert_eq!(historical.root, disputed.root);
        assert_eq!(historical.graph.assertions, disputed.graph.assertions);
        assert_eq!(
            historical.graph.ontology.to_document(),
            disputed.graph.ontology.to_document()
        );
        assert_eq!(
            historical.dispute_attention().unwrap(),
            disputed.dispute_attention().unwrap()
        );
        let w::EssPresence::Present(application) = store
            .schema_proposal(&shown.proposal.proposal_id)
            .unwrap()
            .application
        else {
            panic!("missing retained application")
        };
        assert_eq!(application.steps.len(), 4);
        assert_eq!(
            application
                .steps
                .iter()
                .filter(|step| matches!(*step.step.kind, w::EkrIntegrateApplicationStepKind::V0))
                .count(),
            1
        );
        assert!(
            store.answer_history(None).unwrap().is_empty(),
            "schema review was relabeled as an attention answer"
        );
        assert_repeat_does_not_write(&store, &shown, &approval, &report);
        let events = store.published_events().unwrap();
        drop(store);
        let mut reopened = runtime(&path, sqlite)
            .with_review_authority(human.binding)
            .unwrap();
        reopened.set_full_replay(true);
        let cold = reopened.read(None).unwrap();
        assert_eq!(cold.root, final_read.root);
        assert_eq!(cold.graph.assertions, final_read.graph.assertions);
        for id in old_claims.iter().chain(&refined) {
            assert_eq!(cold.explain(*id).unwrap(), final_read.explain(*id).unwrap());
        }
        assert_eq!(
            reopened
                .read(Some(disputed.root.revision))
                .unwrap()
                .graph
                .assertions,
            disputed.graph.assertions
        );
        assert_repeat_does_not_write(&reopened, &shown, &approval, &report);
        assert_eq!(reopened.published_events().unwrap(), events);
    }
}
