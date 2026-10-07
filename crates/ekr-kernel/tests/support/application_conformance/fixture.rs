// Included beside unchanged shared native fixture helpers; no native test verdict is invoked.
use super::values::{fail, value};
use ekr_store::CommitAuthority;
use ess_conformance::target::TargetError;
use ess_primitives::consistency::QueryConsistency;
use serde_json::{json, Value as Json};
use std::path::{Path, PathBuf};

#[derive(PartialEq, Eq)]
pub(super) struct Boundary {
    pub canonical: Vec<ekr_store::RecordedOccurrence>,
    history: ekr_store::RetainedHistory,
    published: Vec<ekr_kernel::runtime::PublishedEvent>,
    reviews: Json,
    interpretation: Json,
}

pub(super) struct Prepared {
    path: PathBuf,
    sqlite: bool,
    human: Human,
    pub proposal_id: w::EkrIntegrateSchemaProposalId,
    pub fixtures: BTreeMap<String, Json>,
    pub applied: bool,
    source: Option<w::EkrIntegrateInterpretationVersion>,
    capture: Option<ekr_kernel::VerifiedRead>,
    detached_history: Option<(ekr_kernel::KernelAuthority, ekr_store::RetainedHistory)>,
    expected_objects: BTreeMap<ContentHash, Vec<u8>>,
    frozen: Vec<Box<w::EkrIntegrateRetainedApplicationStep>>,
    prefix: Vec<ekr_store::RecordedOccurrence>,
}
impl Prepared {
    pub fn new(work: &Path, sqlite: bool, scenario: &str) -> Result<Self, TargetError> {
        let path = work.join(scenario.replace('/', "-"));
        let store = runtime(&path, sqlite);
        let mut initial = seed();
        // Use the real Project subject, avoiding accidental alias assignment to another node.
        let project = initial
            .ontology
            .node_types
            .iter()
            .find(|t| t.name == "Project")
            .unwrap()
            .id;
        initial
            .graph
            .nodes
            .values_mut()
            .find(|n| n.type_id == project)
            .unwrap()
            .aliases
            .push("Maple".into());
        let seeded = store
            .seed(initial, || Timestamp::EPOCH)
            .map_err(|e| fail("fixture seed", e))?;
        let human = schema_human(seeded.seed_hash);
        let store = store
            .with_review_authority(human.binding.clone())
            .map_err(|e| fail("fixture authority", e))?;
        let preview = store
            .preview_upgrade(&human.policy)
            .map_err(|e| fail("fixture upgrade preview", e))?;
        store
            .apply_upgrade(
                &preview,
                &human.policy,
                &human.proof(&preview),
                b"reviewed contradictions and pending validations",
                || Timestamp::from_millis(1),
            )
            .map_err(|e| fail("fixture upgrade", e))?;
        let mut proposal = proposal_input(&store);
        let mut expected_objects = BTreeMap::new();
        let is_approval = scenario.ends_with("schema-proposal-requires-exact-human-approval");
        let is_resume = scenario.ends_with("interrupted-integration-resumes-without-duplicates");
        let is_detached = scenario.ends_with("canonical-derivation-has-no-transient-dependency");
        if !is_approval && !is_resume && !is_detached {
            return Err(fail("fixture setup", "unknown authored scenario"));
        }
        let mut source = None;
        if !is_approval {
            let observation = application_observation();
            let doc = application_interpretation(&observation);
            store
                .import_observation(&observation, Timestamp::from_millis(2))
                .map_err(|e| fail("fixture observe", e))?;
            let imported = store
                .import_interpretation(&doc, Timestamp::from_millis(2))
                .map_err(|e| fail("fixture incubate", e))?;
            for (digest, payload) in [
                (
                    &observation.observation.content_hash.0,
                    &observation.payload,
                ),
                (&imported.version.document_digest.0, &doc.payload),
            ] {
                expected_objects.insert(
                    digest.parse().map_err(|e| fail("fixture digest", e))?,
                    ekr_core::bytes::decode(payload).map_err(|e| fail("fixture payload", e))?,
                );
            }
            let count = if is_resume { 2 } else { 1 };
            proposal.proposal.evidence.clear();
            proposal.proposal.sources = vec![Box::new(serde_json::from_value(json!({"version":imported.version,"items":(0..count).map(|i|format!("facts[{i}]")).collect::<Vec<_>>()})).unwrap())];
            proposal.proposal.additions.push(Box::new(serde_json::from_value(json!({"kind":"AddOptionalProperty","value":{"owner_type":"Project","property":{"name":"reported_health","value":{"value_kind":"String"},"required":false,"cardinality":"Many"}}})).unwrap()));
            proposal.proposal.mappings=(0..count).map(|i|Box::new(serde_json::from_value(json!({"source":imported.version,"source_item":format!("facts[{i}]"),"source_type":"Project","target_type":"Project","target_member":"reported_health","value":{"kind":"CopyField","value":{"declaration":"Project","field":"health"}}})).unwrap())).collect();
            proposal.payload =
                ekr_core::bytes::encode(&serde_json::to_vec(&proposal.proposal).unwrap());
            source = Some(*imported.version);
        }
        let shown = store
            .submit_schema_proposal(&proposal, Timestamp::from_millis(3))
            .map_err(|e| fail("fixture proposal", e))?;
        let signed = signed_review(
            &human,
            &shown,
            true,
            ekr_core::EventId::mint(),
            b"Approve these exact fixture additions and mappings.",
        );
        let mut fixtures = BTreeMap::new();
        fixtures.insert("proposal-id".into(), json!(shown.proposal.proposal_id));
        fixtures.insert("proposal-digest".into(), json!(shown.proposal_digest));
        let mut prepared = Self {
            path,
            sqlite,
            human,
            proposal_id: (*shown.proposal.proposal_id).clone(),
            fixtures,
            applied: false,
            source,
            capture: None,
            detached_history: None,
            expected_objects,
            frozen: vec![],
            prefix: vec![],
        };
        if is_approval {
            let read = store.read(None).map_err(|e| fail("fixture read", e))?;
            let semantic = review::proof_from_document(&signed.human_proof)
                .map_err(|e| fail("fixture signed proof", e.reason))?;
            let proof_digest = review::digest(
                &review::proof_bytes(&semantic)
                    .map_err(|e| fail("fixture proof digest", e.reason))?,
            );
            let mut proof = value(&signed.human_proof)?;
            proof["intent"]["format"] = json!("HumanDecision1");
            let mut invalid = proof.clone();
            invalid["signature"] = json!(ekr_core::bytes::encode(&[0; 64]));
            prepared.fixtures.extend([
                (
                    "wrong-proposal-digest".into(),
                    json!(ContentHash::of_bytes(b"a different proposal")),
                ),
                ("review-basis".into(), json!(shown.basis)),
                ("invalid-signature-proof".into(), invalid),
                ("exact-review-proof".into(), proof),
                ("exact-review-proof-digest".into(), json!(proof_digest)),
                ("review-statement".into(), json!(signed.statement)),
                (
                    "reviewer".into(),
                    json!({"actor":context().operator,"authentication_subject":"fixture-human"}),
                ),
                ("original-revision".into(), json!(read.root.revision.get())),
                (
                    "original-knowledge-root".into(),
                    json!(read.root.knowledge_root),
                ),
            ]);
            return Ok(prepared);
        }
        let approval = store
            .approve_schema_proposal(&signed, Timestamp::from_millis(4))
            .map_err(|e| fail("fixture review", e))?;
        prepared
            .fixtures
            .insert("review-id".into(), json!(approval.review_id));
        drop(store);
        let reference = work.join(format!("{}-reference", scenario.replace('/', '-')));
        if is_resume {
            copy_closed_provider(&prepared.path, &reference);
        }
        let complete_path = if is_resume {
            &reference
        } else {
            &prepared.path
        };
        let complete = runtime(complete_path, sqlite)
            .with_review_authority(prepared.human.binding.clone())
            .map_err(|e| fail("fixture completion authority", e))?;
        complete
            .apply_schema_proposal(
                &prepared.proposal_id,
                &approval.review_id,
                &shown.proposal_digest,
                Timestamp::from_millis(5),
            )
            .map_err(|e| fail("fixture reference application", e))?;
        let (authority, history) =
            captured_application_history(complete_path, sqlite, &prepared.human);
        authority
            .verify(&history, None, None)
            .map_err(|e| fail("fixture reference replay", e))?;
        let transactions = complete
            .transactions()
            .map_err(|e| fail("fixture reference transactions", e))?;
        let election = history
            .applications
            .elections()
            .first()
            .ok_or_else(|| fail("fixture election", "absent"))?
            .clone();
        let mut steps = history.applications.steps().to_vec();
        steps.sort_by_key(|step| {
            transactions[&step.transaction.id.0.parse().unwrap()]
                .committed
                .as_ref()
                .unwrap()
                .result
                .revision
        });
        prepared.frozen = steps.clone();
        prepared
            .fixtures
            .insert("application-id".into(), json!(election.application_id));
        if is_resume {
            if steps.len() != 3 || history.applications.processing_receipts().len() != 2 {
                return Err(fail(
                    "fixture prefix",
                    "reference did not commit schema and two facts",
                ));
            }
            drop(complete);
            let retention = application_retention(&prepared.path, sqlite, &prepared.human);
            retention
                .elect_application(
                    &w::EkrStoreApplicationElectionRetention {
                        election: election.clone(),
                        objects: w::EkrStoreApplicationElectionRetentionObjects {
                            ess_extra: Default::default(),
                        },
                    },
                    Timestamp::from_millis(5),
                )
                .map_err(|e| fail("fixture elect", e))?;
            drop(retention);
            let target = prepared.open()?;
            for (index, step) in steps.iter().enumerate() {
                let retention = application_retention(&prepared.path, sqlite, &prepared.human);
                retention
                    .elect_application_step(
                        &w::EkrStoreApplicationStepRetention {
                            step: step.clone(),
                            objects: w::EkrStoreApplicationStepRetentionObjects {
                                ess_extra: Default::default(),
                            },
                        },
                        Timestamp::from_millis(5),
                    )
                    .map_err(|e| fail("fixture step", e))?;
                let attempt = history
                    .applications
                    .attempts()
                    .iter()
                    .find(|a| a.step_election_id == step.step_election_id)
                    .ok_or_else(|| fail("fixture attempt", "absent"))?;
                retention
                    .elect_application_attempt(
                        &w::EkrStoreApplicationAttemptRetention {
                            attempt: attempt.clone(),
                        },
                        Timestamp::from_millis(5),
                    )
                    .map_err(|e| fail("fixture attempt", e))?;
                drop(retention);
                if index < 2 {
                    let id = attempt.transaction_id.0.parse().unwrap();
                    commit_application_document(
                        &target,
                        id,
                        &transactions[&id].proposal.document_bytes,
                        5,
                    );
                }
            }
            let (_, prefix) = captured_application_history(&prepared.path, sqlite, &prepared.human);
            if !prefix.applications.receipts().is_empty()
                || !prefix.applications.processing_receipts().is_empty()
            {
                return Err(fail("fixture prefix", "receipts were not interrupted"));
            }
            let actual = target
                .transactions()
                .map_err(|e| fail("fixture prefix transactions", e))?;
            for (i, step) in steps.iter().enumerate() {
                let tx = actual.get(&step.transaction.id.0.parse().unwrap());
                if (i < 2 && tx.and_then(|t| t.committed.as_ref()).is_none())
                    || (i == 2 && tx.is_some())
                {
                    return Err(fail("fixture prefix", "wrong ordinary commit boundary"));
                }
            }
            prepared.prefix = prefix.occurrences;
            let schema_tx = steps[0].transaction.id.clone();
            let schema_revision = actual[&schema_tx.0.parse().unwrap()]
                .committed
                .as_ref()
                .unwrap()
                .result
                .revision;
            prepared.fixtures.extend([
                ("schema-transaction-id".into(), json!(schema_tx)),
                ("schema-revision".into(), json!(schema_revision.get())),
                (
                    "source-document-digest".into(),
                    json!(prepared.source.as_ref().unwrap().document_digest),
                ),
            ]);
            for (name, step) in [("first", &steps[1]), ("second", &steps[2])] {
                let derivation = &step.derivations[0];
                prepared.fixtures.insert(
                    format!("{name}-mapping-digest"),
                    json!(step.mappings[0].mapping_digest),
                );
                prepared.fixtures.insert(
                    format!("{name}-assertion-id"),
                    json!(derivation.assertion_id),
                );
                prepared.fixtures.insert(
                    format!("{name}-assertions"),
                    json!([derivation.assertion_id]),
                );
                prepared
                    .fixtures
                    .insert(format!("{name}-transaction-id"), json!(step.transaction.id));
            }
        } else {
            drop(complete);
            let cold = prepared.open()?;
            let capture = cold
                .read(None)
                .map_err(|e| fail("fixture cold capture", e))?;
            let step = steps
                .iter()
                .find(|s| !s.derivations.is_empty())
                .ok_or_else(|| fail("fixture derivation", "absent"))?;
            if step.derivations.len() != 1 || step.mappings.len() != 1 {
                return Err(fail(
                    "fixture explanation",
                    "expected one mapping and one derivation",
                ));
            }
            let derivation = &step.derivations[0];
            let mapping = &step.mappings[0];
            let evidence = &capture.graph.evidence[&derivation.evidence_id.0.parse().unwrap()];
            let EvidenceSource::Observation(observation) = evidence.source else {
                return Err(fail("fixture explanation", "not observation evidence"));
            };
            // Inventory: assertion + mapping + derivation + ordinary proposal/validation/commit
            // + the one distinct support evidence. No call to Explain is used as an oracle.
            if mapping.evidence.len() != 1 || step.transaction.evidence.len() != 1 {
                return Err(fail("fixture explanation", "unexpected support inventory"));
            }
            prepared.fixtures.extend([
                ("assertion-id".into(), json!(derivation.assertion_id)),
                ("mapping-id".into(), json!(mapping.mapping_id)),
                ("derivation-id".into(), json!(derivation.derivation_id)),
                ("observation-id".into(), json!(observation)),
                ("evidence-id".into(), json!(evidence.id)),
                ("mapping-evidence".into(), json!(mapping.evidence)),
                ("mapping-digest".into(), json!(mapping.mapping_digest)),
                (
                    "source-document-digest".into(),
                    json!(mapping.source_document_digest),
                ),
                (
                    "observation-content-hash".into(),
                    json!(evidence.content_hash),
                ),
                ("explanation-link-count".into(), json!(7)),
            ]);
            let (cold_authority, cold_history) =
                captured_application_history(&prepared.path, sqlite, &prepared.human);
            cold_authority
                .verify(&cold_history, None, None)
                .map_err(|e| fail("fixture cold history", e))?;
            prepared.capture = Some(capture);
            prepared.detached_history = Some((cold_authority, cold_history));
            drop(cold);
            std::fs::rename(&prepared.path, work.join("detached-provider"))
                .map_err(|e| fail("detach temporary provider", e))?;
        }
        Ok(prepared)
    }
    pub fn open(&self) -> Result<Runtime, TargetError> {
        let store = if self.sqlite {
            Runtime::sqlite_existing(&self.path, "upgrade-fixture", context(), anchor())
        } else {
            Runtime::file_existing(&self.path, "upgrade-fixture", context(), anchor())
        }
        .map_err(|e| fail("cold open", e))?;
        let mut store = store
            .with_review_authority(self.human.binding.clone())
            .map_err(|e| fail("review authority", e))?;
        store.set_full_replay(true);
        Ok(store)
    }
    pub fn boundary(&self, store: &Runtime) -> Result<Boundary, TargetError> {
        let (authority, history) =
            captured_application_history(&self.path, self.sqlite, &self.human);
        authority
            .verify(&history, None, None)
            .map_err(|e| fail("boundary verification", e))?;
        let interpretation = match &self.source {
            Some(source) => value(
                store
                    .interpretation(source)
                    .map_err(|e| fail("boundary source", e))?,
            )?,
            None => Json::Null,
        };
        Ok(Boundary {
            canonical: history.occurrences.clone(),
            history,
            published: store
                .published_events()
                .map_err(|e| fail("boundary feed", e))?,
            reviews: value(
                store
                    .schema_proposal_reviews(&self.proposal_id)
                    .map_err(|e| fail("boundary reviews", e))?,
            )?,
            interpretation,
        })
    }
    pub fn verify_report(
        &self,
        store: &Runtime,
        report: &w::EkrIntegrateApplicationReport,
        repeat: bool,
    ) -> Result<(), TargetError> {
        let (authority, history) =
            captured_application_history(&self.path, self.sqlite, &self.human);
        authority
            .verify(&history, None, None)
            .map_err(|e| fail("report replay", e))?;
        let read = store.read(None).map_err(|e| fail("report read", e))?;
        if !history.occurrences.starts_with(&self.prefix) {
            return Err(fail("report prefix", "original occurrences changed"));
        }
        if history.applications.steps().len() != self.frozen.len()
            || self
                .frozen
                .iter()
                .any(|s| !history.applications.steps().contains(s))
        {
            return Err(fail("report identities", "frozen steps changed"));
        }
        let election = history
            .applications
            .elections()
            .first()
            .ok_or_else(|| fail("report election", "missing"))?;
        let receipt = history
            .applications
            .receipts()
            .iter()
            .find(|r| report.receipt_id == w::EssPresence::Present(r.receipt_id.clone()))
            .ok_or_else(|| fail("report receipt", "not independently retained"))?;
        if value(&receipt.progress)? != json!("Complete")
            || !receipt.remaining_items.is_empty()
            || receipt.corrections_pending
            || !matches!(receipt.stop_reason, w::EssPresence::Absent)
        {
            return Err(fail("report receipt", "not complete"));
        }
        if history.applications.processing_receipts().len() != 2
            || receipt.processing_receipts.len() != 2
        {
            return Err(fail("report receipts", "incorrect exact coverage"));
        }
        let mut items = vec![];
        let mut seen = BTreeSet::new();
        for key in &election.selected_items {
            let matches: Vec<_> = history
                .applications
                .processing_receipts()
                .iter()
                .filter(|r| {
                    r.document_digest == key.source.document_digest
                        && r.item == key.item
                        && r.mapping_digest == w::EssPresence::Present(key.mapping_digest.clone())
                })
                .collect();
            if matches.len() != 1 {
                return Err(fail(
                    "report item",
                    "missing or duplicate qualified receipt",
                ));
            }
            let held = matches[0];
            if !receipt.processing_receipts.contains(&held.receipt_id)
                || !seen.insert(held.receipt_id.0.clone())
            {
                return Err(fail(
                    "report receipt links",
                    "missing or repeated processing identity",
                ));
            }
            let w::EssPresence::Present(tx) = &held.transaction_id else {
                return Err(fail("report item", "uncommitted receipt"));
            };
            let record = read
                .transactions
                .get(&tx.0.parse().map_err(|e| fail("report tx id", e))?)
                .ok_or_else(|| fail("report transaction", "absent"))?;
            let committed = record
                .committed
                .as_ref()
                .ok_or_else(|| fail("report transaction", "not committed"))?;
            if held.basis_digest.0
                != read.revisions[&committed.result.revision]
                    .record_hash
                    .to_string()
                || value(&held.disposition)? != json!("Integrated")
                || held.assertions.len() != 1
            {
                return Err(fail(
                    "report support",
                    "receipt disagrees with actual commit",
                ));
            }
            let claim = read
                .graph
                .assertions
                .get(
                    &held.assertions[0]
                        .0
                        .parse()
                        .map_err(|e| fail("report assertion id", e))?,
                )
                .ok_or_else(|| fail("report assertion", "absent"))?;
            let step = self
                .frozen
                .iter()
                .find(|s| s.transaction.id == *tx)
                .ok_or_else(|| fail("report frozen transaction", "absent"))?;
            if step.derivations.len() != 1
                || step.derivations[0].assertion_id != held.assertions[0]
                || !claim.evidence.contains(&ekr_core::CanonicalRef::new(
                    step.derivations[0]
                        .evidence_id
                        .0
                        .parse()
                        .map_err(|e| fail("report evidence id", e))?,
                ))
            {
                return Err(fail("report assertion", "frozen claim/support differs"));
            }
            let matching_commits=history.occurrences.iter().filter(|o|matches!(&o.event.payload,ekr_graph::RevisionPayload::Committed{transaction_id,..} if transaction_id.to_string()==tx.0)).count();
            if matching_commits != 1 {
                return Err(fail(
                    "report commit count",
                    "qualified mapping did not commit exactly once",
                ));
            }
            items.push(json!({"source":key.source,"item":key.item,"mapping_digest":key.mapping_digest,"disposition":"Integrated","transaction_id":tx,"assertions":held.assertions,"blockers":[]}));
        }
        let schema = read
            .transactions
            .get(
                &receipt
                    .schema_transaction
                    .0
                    .parse()
                    .map_err(|e| fail("schema tx", e))?,
            )
            .and_then(|r| r.committed.as_ref())
            .ok_or_else(|| fail("schema publication", "absent"))?;
        if receipt.schema_revision.0.as_u64() != Some(schema.result.revision.get()) {
            return Err(fail("schema publication", "wrong revision"));
        }
        let expected:w::EkrIntegrateApplicationReport=serde_json::from_value(json!({"application_id":election.application_id,"receipt_id":receipt.receipt_id,"progress":"Complete","schema_transaction":receipt.schema_transaction,"schema_revision":receipt.schema_revision,"items":items,"remaining_items":[],"corrections_pending":false,"already_complete":repeat})).map_err(|e|fail("expected generated report",e))?;
        if report != &expected {
            return Err(fail(
                "application report",
                "complete response differs from retained election/commits/receipts",
            ));
        }
        Ok(())
    }
    pub fn duplicate_write(&self, store: &Runtime) -> Result<(), TargetError> {
        let read = store
            .read(None)
            .map_err(|e| fail("duplicate control read", e))?;
        let id = self
            .frozen
            .iter()
            .find(|s| !s.derivations.is_empty())
            .unwrap()
            .derivations[0]
            .assertion_id
            .0
            .parse()
            .unwrap();
        let mut claim = read.graph.assertions[&id].clone();
        claim.id = AssertionId::mint();
        claim.assessment = Assessment::Proposed;
        claim.lifecycle = AssertionLifecycle::Active;
        claim.transaction_time = TransactionTime::since(Timestamp::from_millis(30));
        let tx = GraphTransaction {
            id: TransactionId::mint(),
            proposer: context().operator,
            evidence: claim.evidence.iter().map(|r| r.id()).collect(),
            operations: vec![GraphOperation::AddAssertion(Box::new(claim))],
            schema_version: None,
        };
        // Actual extra ordinary publication. The duplicate-write control does not forge an
        // event or ask the handler to lie about its successful application response.
        commit_application_document(store, tx.id, &encode(&tx), 30);
        Ok(())
    }
    pub fn explain(&self, id: AssertionId) -> Result<ekr_kernel::ExplanationResult, TargetError> {
        let capture = self
            .capture
            .as_ref()
            .ok_or_else(|| fail("detached explain", "capture absent"))?;
        if self.path.exists() {
            return Err(fail("detached explain", "provider still reachable"));
        }
        let (authority, history) = self.detached_history.as_ref().unwrap();
        let before = history.clone();
        authority
            .verify(history, None, None)
            .map_err(|e| fail("detached retained verification", e))?;
        for (hash, bytes) in &self.expected_objects {
            if capture.content(hash) != Some(bytes.as_slice())
                || ContentHash::of_bytes(bytes) != *hash
            {
                return Err(fail("detached content", "original payload lost or changed"));
            }
        }
        let explained = capture
            .explain(id)
            .map_err(|e| fail("detached Explain", e))?;
        if history != &before {
            return Err(fail("detached read", "captured history changed"));
        }
        let mut mapping = 0;
        let mut derivation = 0;
        let mut evidence = 0;
        let mut assertion = 0;
        let mut proposal = 0;
        let mut validation = 0;
        let mut commit = 0;
        for link in &explained.links {
            match link {
                ekr_kernel::ExplanationLink::Mapping(_) => mapping += 1,
                ekr_kernel::ExplanationLink::Derivation(_) => derivation += 1,
                ekr_kernel::ExplanationLink::Evidence(_) => evidence += 1,
                ekr_kernel::ExplanationLink::Assertion(_) => assertion += 1,
                ekr_kernel::ExplanationLink::Proposal(_) => proposal += 1,
                ekr_kernel::ExplanationLink::Validation(_) => validation += 1,
                ekr_kernel::ExplanationLink::Commit(_) => commit += 1,
                _ => return Err(fail("explanation inventory", "unexpected link kind")),
            }
        }
        if [
            mapping, derivation, evidence, assertion, proposal, validation, commit,
        ] != [1; 7]
        {
            return Err(fail(
                "explanation inventory",
                "missing or duplicate independently expected link",
            ));
        }
        Ok(explained)
    }
    pub fn rows(
        &self,
        view: &str,
        consistency: &QueryConsistency,
    ) -> Result<Vec<Json>, TargetError> {
        if let Some(capture) = &self.capture {
            check_consistency(capture, consistency)?;
            let id = serde_json::from_value(self.fixtures["assertion-id"].clone())
                .map_err(|e| fail("capture assertion", e))?;
            let explained = self.explain(id)?;
            return match view {
                "ekr.integrate.MappingRecordRecords"=>explained.links.iter().filter_map(|l|if let ekr_kernel::ExplanationLink::Mapping(m)=l{Some(mapping_row(m))}else{None}).collect(),
                "ekr.integrate.CanonicalDerivationRecords"=>explained.links.iter().filter_map(|l|if let ekr_kernel::ExplanationLink::Derivation(d)=l{Some(recorded(d))}else{None}).collect(),
                "ekr.kernel.RetainedEvidence"=>Ok(capture.graph.evidence.values().map(|e|json!({"evidence_id":e.id,"content_hash":e.content_hash,"extracted_by":e.extracted_by})).collect()),
                _=>Err(TargetError::unsupported(view,"not a detached capture view")),
            };
        }
        let store = self.open()?;
        let before = self.boundary(&store)?;
        let read = store.read(None).map_err(|e| fail("view replay", e))?;
        check_consistency(&read, consistency)?;
        let (_, history) = captured_application_history(&self.path, self.sqlite, &self.human);
        let committed_steps: Vec<_> = history
            .applications
            .steps()
            .iter()
            .filter(|s| {
                read.transactions
                    .get(&s.transaction.id.0.parse().unwrap())
                    .is_some_and(|t| t.committed.is_some())
            })
            .collect();
        let rows = match view {
            "ekr.integrate.ProposalReviewRecords" => store
                .schema_proposal_reviews(&self.proposal_id)
                .map_err(|e| fail("review rows", e))?
                .iter()
                .map(|r| recorded(&r.review))
                .collect::<Result<Vec<_>, _>>()?,
            "ekr.integrate.ApplicationReceiptRecords" => history
                .applications
                .receipts()
                .iter()
                .map(recorded)
                .collect::<Result<Vec<_>, _>>()?,
            "ekr.integrate.ProcessingReceiptRecords" => {
                let source = self
                    .source
                    .as_ref()
                    .ok_or_else(|| fail("processing rows", "source absent"))?;
                // Incubation owns original Parked receipts. Canonical application owns its
                // separate processing records. Their union preserves both histories.
                let imported = store
                    .interpretation(source)
                    .map_err(|e| fail("incubation rows", e))?;
                let mut rows = imported
                    .receipts
                    .iter()
                    .map(recorded)
                    .collect::<Result<Vec<_>, _>>()?;
                for receipt in history.applications.processing_receipts() {
                    if !rows
                        .iter()
                        .any(|row| row["receipt_id"] == json!(receipt.receipt_id))
                    {
                        rows.push(recorded(receipt)?);
                    }
                }
                rows
            }
            "ekr.integrate.MappingRecordRecords" => committed_steps
                .iter()
                .flat_map(|s| s.mappings.iter())
                .map(|m| mapping_row(m))
                .collect::<Result<Vec<_>, _>>()?,
            "ekr.integrate.CanonicalDerivationRecords" => committed_steps
                .iter()
                .flat_map(|s| s.derivations.iter())
                .map(recorded)
                .collect::<Result<Vec<_>, _>>()?,
            "ekr.graph.Assertions" => {
                let mut claims: Vec<_> = read.graph.assertions.values().collect();
                claims.sort_by_key(|c| std::cmp::Reverse(c.transaction_time.recorded_from));
                claims
                    .into_iter()
                    .map(assertion_row)
                    .collect::<Result<Vec<_>, _>>()?
            }
            _ => return Err(TargetError::unsupported(view, "not an application view")),
        };
        if self.boundary(&store)? != before {
            return Err(fail("view integrity", "retained state changed"));
        }
        Ok(rows)
    }
}
fn recorded<T: serde::Serialize>(record: T) -> Result<Json, TargetError> {
    let mut row = value(record)?;
    row["state"] = json!("Recorded");
    Ok(row)
}
fn mapping_row(mapping: &w::EkrIntegrateRetainedMappingRecord) -> Result<Json, TargetError> {
    // payload is an immutable-object transport field, not a declared view column.
    let mut row = recorded(mapping)?;
    row.as_object_mut().unwrap().remove("payload");
    Ok(row)
}
fn check_consistency(
    read: &ekr_kernel::VerifiedRead,
    request: &QueryConsistency,
) -> Result<(), TargetError> {
    if let QueryConsistency::AtLeast { token } = request {
        let (number, hash) = token
            .as_str()
            .strip_prefix("revision:")
            .and_then(|s| s.split_once(':'))
            .ok_or_else(|| fail("view consistency", "unknown token"))?;
        let revision = RevisionNumber::new(number.parse().map_err(|e| fail("revision token", e))?);
        if read
            .revisions
            .get(&revision)
            .is_none_or(|r| r.record_hash.to_string() != hash)
        {
            return Err(fail("view consistency", "boundary unavailable"));
        }
    }
    Ok(())
}
fn instant(at: Timestamp) -> Result<String, TargetError> {
    let at = time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(at.millis()) * 1_000_000)
        .map_err(|e| fail("timestamp", e))?;
    Ok(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute(),
        at.second(),
        at.millisecond()
    ))
}
fn assertion_row(claim: &Assertion) -> Result<Json, TargetError> {
    let assessment = match &claim.assessment {
        Assessment::Proposed => json!({"kind":"Proposed"}),
        Assessment::Validating {
            completed,
            required,
        } => json!({"kind":"Validating","completed":completed,"required":required}),
        Assessment::Accepted { validators } => {
            json!({"kind":"Accepted","validators":validators.iter().map(ToString::to_string).collect::<Vec<_>>()})
        }
        Assessment::Rejected { issues } => {
            json!({"kind":"Rejected","issues":issues.iter().map(ToString::to_string).collect::<Vec<_>>()})
        }
        Assessment::Disputed {
            competing_assertions,
        } => {
            json!({"kind":"Disputed","competing_assertions":competing_assertions.iter().map(|r|r.id().to_string()).collect::<Vec<_>>()})
        }
    };
    let lifecycle = match &claim.lifecycle {
        AssertionLifecycle::Active => json!({"kind":"Active"}),
        AssertionLifecycle::Retracted {
            at_revision,
            reason,
        } => json!({"kind":"Retracted","at_revision":at_revision.get(),"reason":reason.as_str()}),
        AssertionLifecycle::Superseded {
            by,
            at_revision,
            effective_from,
        } => {
            json!({"kind":"Superseded","by":by.id().to_string(),"at_revision":at_revision.get(),"effective_from":instant(*effective_from)?})
        }
    };
    Ok(
        json!({"assertion_id":claim.id,"root_id":claim.root_id,"assessment":assessment,"lifecycle":lifecycle,"recorded_from":instant(claim.transaction_time.recorded_from)?}),
    )
}
fn application_observation() -> w::EkrObserveObservationImport {
    let payload = b"Synthetic retained schema vocabulary evidence.";
    let hash = ContentHash::of_bytes(payload);
    let key = ekr_core::ObservationIdempotencyKey {
        source: "manual".into(),
        source_native_id: Some("schema-support".into()),
        content_hash: hash,
    };
    serde_json::from_value(json!({"observation":{"observation_id":key.observation_id(),"source":key.source,"source_native_id":key.source_native_id,"content_hash":hash,"captured_at":"1970-01-01T00:00:00.002Z","kind":"FeedItem"},"key":{"source":key.source,"source_native_id":key.source_native_id,"content_hash":hash},"payload":ekr_core::bytes::encode(payload)})).unwrap()
}
fn application_interpretation(
    observation: &w::EkrObserveObservationImport,
) -> w::EkrIntegrateInterpretationImport {
    use ekr_core::Canonical;
    let reference = json!({"node_type":"Project","aliases":["Maple"]});
    let evidence:Vec<_>=[7500,1200].into_iter().map(|confidence|json!({"evidence":{"id":EvidenceId::mint(),"source":{"kind":"Observation","observation":observation.observation.observation_id},"content_hash":observation.observation.content_hash,"observed_at":"1970-01-01T00:00:00.002Z","extracted_by":context().validator,"confidence_bp":confidence},"payload":observation.payload})).collect();
    let facts:Vec<_>=evidence.iter().enumerate().map(|(i,e)|json!({"kind":"Property","value":{"subject":reference,"property":"health","value":{"kind":"String","canonical_bytes":ekr_core::bytes::encode(&ekr_graph::CanonicalValue::String(format!("health-{i}")).canonical_bytes())},"evidence":[e["evidence"]["id"]]}})).collect();
    let document = json!({"version":{"interpretation_id":ekr_core::NodeId::mint(),"version":1},"root_id":ekr_core::GraphRootId::mint(),"observations":[observation.observation.observation_id],"local_schema":{"node_types":[{"name":"Project","parents":[],"abstract_type":false,"properties":[{"name":"health","value":{"value_kind":"String"},"required":false,"cardinality":"One"}]}],"edge_types":[]},"entities":[reference],"facts":facts,"evidence":evidence});
    serde_json::from_value(json!({"payload":ekr_core::bytes::encode(&serde_json::to_vec_pretty(&document).unwrap()),"document":document})).unwrap()
}
