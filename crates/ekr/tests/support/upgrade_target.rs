//! Native Runtime adapter. Reopens and fully replays on every command and view query.
#[path = "upgrade_fixture.rs"]
mod fixture;
use super::knowledge_target::{json_input, node, unavailable};
use ekr::conformance::Provider;
use ekr_core::{contract_data as wire, ContentHash, RevisionNumber, Timestamp};
use ekr_graph::{Assertion, AssertionLifecycle, Assessment, RevisionEvent, RevisionPayload};
use ekr_kernel::runtime::PublishedEvent;
use ekr_kernel::{human_review, Runtime, VerifiedRead};
use ess_conformance::target::*;
use ess_primitives::consistency::{ConsistencyToken, QueryConsistency};
use ess_primitives::node::Node;
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub struct UpgradeTarget {
    provider: Provider,
    work: PathBuf,
    no_op: bool,
    state: RefCell<Option<State>>,
    observed: RefCell<Vec<ObservedEvent>>,
}
struct State {
    path: PathBuf,
    human: fixture::Human,
    original_events: Vec<PublishedEvent>,
    original_bytes: BTreeMap<ContentHash, Vec<u8>>,
}
fn map(value: Value) -> Result<ViewRow, TargetError> {
    node(value)?
        .as_map()
        .cloned()
        .ok_or_else(|| unavailable("projecting a row", "expected record"))
}
fn value<T: serde::Serialize>(input: T) -> Result<Value, TargetError> {
    serde_json::to_value(input).map_err(|e| unavailable("projecting retained value", e))
}
fn instant(at: Timestamp) -> Result<String, TargetError> {
    let at = time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(at.millis()) * 1_000_000)
        .map_err(|e| unavailable("projecting timestamp", e))?;
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
fn consistency(read: &VerifiedRead) -> Result<ConsistencyToken, TargetError> {
    let revision = read.root.revision;
    ConsistencyToken::new(format!(
        "revision:{}:{}",
        revision.get(),
        read.revisions[&revision].record_hash
    ))
    .map_err(|e| unavailable("capturing verified read boundary", e))
}
fn check_consistency(read: &VerifiedRead, requested: &QueryConsistency) -> Result<(), TargetError> {
    if let QueryConsistency::AtLeast { token } = requested {
        let (number, hash) = token
            .as_str()
            .strip_prefix("revision:")
            .and_then(|token| token.split_once(':'))
            .ok_or_else(|| unavailable("reading a consistent view", "unknown token format"))?;
        let number = number
            .parse::<u64>()
            .map_err(|e| unavailable("reading token revision", e))?;
        let hash: ContentHash = hash
            .parse()
            .map_err(|e| unavailable("reading token record hash", e))?;
        if !read
            .revisions
            .get(&RevisionNumber::new(number))
            .is_some_and(|held| held.record_hash == hash)
        {
            return Err(unavailable(
                "reading a consistent view",
                "verified history does not contain the requested boundary",
            ));
        }
    }
    Ok(())
}
impl UpgradeTarget {
    pub fn new(provider: Provider, work: &Path, no_op: bool) -> Self {
        Self {
            provider,
            work: work.into(),
            no_op,
            state: RefCell::new(None),
            observed: RefCell::new(vec![]),
        }
    }
    fn open(&self, state: &State) -> Result<Runtime, TargetError> {
        let runtime = match self.provider {
            Provider::File => Runtime::file_existing(
                &state.path,
                fixture::TENANT,
                fixture::context(),
                fixture::anchor(),
            ),
            Provider::Sqlite => Runtime::sqlite_existing(
                &state.path,
                fixture::TENANT,
                fixture::context(),
                fixture::anchor(),
            ),
        }
        .map_err(|e| unavailable("reopening native store", e))?;
        let mut runtime = runtime
            .with_review_authority(state.human.binding.clone())
            .map_err(|e| unavailable("binding independent host trust", e))?;
        runtime.set_full_replay(true);
        Ok(runtime)
    }
    fn state(&self) -> Result<std::cell::Ref<'_, State>, TargetError> {
        std::cell::Ref::filter_map(self.state.borrow(), Option::as_ref)
            .map_err(|_| unavailable("opening scenario", "fixtures have not been provisioned"))
    }
    fn event(
        &self,
        request: &SemanticCommandRequest,
        name: &str,
        payload: Value,
    ) -> Result<ObservedEvent, TargetError> {
        let mut event =
            ObservedEvent::new(name.parse().map_err(|e| unavailable("naming event", e))?)
                .in_activity(request.correlation.clone())
                .at(self.observed.borrow().len().try_into().unwrap());
        event.payload = map(payload)?;
        self.observed.borrow_mut().push(event.clone());
        Ok(event)
    }
}
impl ConformanceTarget for UpgradeTarget {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        Ok(ImplementationIdentity::new(
            format!(
                "EKR native Runtime {:?}, no-op={}",
                self.provider, self.no_op
            ),
            env!("CARGO_PKG_VERSION"),
        ))
    }
    fn fixture_values(
        &self,
        scenario: &ScenarioContext,
        contract: &ess_conformance::fixtures::Contract,
    ) -> Result<BTreeMap<String, Node>, TargetError> {
        let exclusions = scenario
            .scenario
            .to_string()
            .ends_with("equal-disjoint-and-many-claims-do-not-conflict");
        let directory = self
            .work
            .join(scenario.scenario.to_string().replace('/', "-"));
        std::fs::create_dir_all(&directory)
            .map_err(|e| unavailable("creating fixture namespace", e))?;
        let path = directory.join(match self.provider {
            Provider::File => "store",
            Provider::Sqlite => "store.sqlite",
        });
        let runtime = match self.provider {
            Provider::File => Runtime::file(
                &path,
                fixture::TENANT,
                fixture::context(),
                fixture::anchor(),
            ),
            Provider::Sqlite => Runtime::sqlite(
                &path,
                fixture::TENANT,
                fixture::context(),
                fixture::anchor(),
            ),
        }
        .map_err(|e| unavailable("provisioning fixture store", e))?;
        let seeded = runtime
            .seed(fixture::seed(exclusions), || Timestamp::EPOCH)
            .map_err(|e| unavailable("admitting original fixture", e))?;
        let human = fixture::Human::new(seeded.seed_hash);
        let runtime = runtime
            .with_review_authority(human.binding.clone())
            .map_err(|e| unavailable("provisioning trusted test host", e))?;
        let read = runtime
            .read(None)
            .map_err(|e| unavailable("reading pre-execution baseline", e))?;
        let preview = runtime
            .preview_upgrade(&human.policy)
            .map_err(|e| unavailable("preparing reviewed fixture input", e))?;
        let mut proof = value(human.proof(&preview))?;
        // ESS semantic enum member vs. its declared serialized wire label. No signature bytes change.
        proof["intent"]["format"] = json!("HumanDecision1");
        let mut supplied = BTreeMap::from([
            ("upgrade-preview".to_owned(), node(value(&preview)?)?),
            ("upgrade-proof".to_owned(), node(proof)?),
            ("target-version".to_owned(), node(value(&preview.to)?)?),
            (
                "original-knowledge-root".to_owned(),
                node(json!(read.root.knowledge_root.to_string()))?,
            ),
            (
                "original-evidence-root".to_owned(),
                node(json!(read.root.evidence_root.to_string()))?,
            ),
            (
                "original-agent-root".to_owned(),
                node(json!(read.root.agent_root.to_string()))?,
            ),
        ]);
        supplied.retain(|name, _| {
            contract
                .fields
                .iter()
                .any(|field| field.name.as_str() == name)
        });
        let mut original_bytes = BTreeMap::new();
        for hash in [
            seeded.seed_hash,
            read.revisions[&RevisionNumber::SEED].record_hash,
        ]
        .into_iter()
        .chain(read.seed_input.evidence_payloads.keys().copied())
        {
            let bytes = runtime
                .content(&hash)
                .map_err(|e| unavailable("retaining pre-execution bytes", e))?
                .ok_or_else(|| {
                    unavailable("retaining pre-execution bytes", format!("missing {hash}"))
                })?;
            original_bytes.insert(hash, bytes);
        }
        let original_events = runtime
            .published_events()
            .map_err(|e| unavailable("reading original publication prefix", e))?;
        *self.state.borrow_mut() = Some(State {
            path,
            human,
            original_events,
            original_bytes,
        });
        Ok(supplied)
    }
    fn begin_scenario(&self, _: &ScenarioContext) -> Result<(), TargetError> {
        drop(self.state()?);
        self.observed.borrow_mut().clear();
        Ok(())
    }
    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        let command = request.command.to_string();
        let actor = match command.as_str() {
            "ekr.kernel.PreviewUpgrade" | "ekr.kernel.ListAttention" => {
                "ekr.kernel.KnowledgeReader"
            }
            "ekr.kernel.ApplyUpgrade" => "ekr.kernel.HumanOperator",
            "ekr.kernel.Snapshot" => "ekr.kernel.Operator",
            _ => {
                return Err(TargetError::unsupported(
                    command,
                    "no upgrade fixture handler",
                ))
            }
        };
        if request
            .actor
            .as_ref()
            .is_some_and(|supplied| supplied.to_string() != actor)
        {
            return Err(TargetError::unsupported(
                command,
                "actor does not match the independently provisioned role",
            ));
        }
        let outcome = if command == "ekr.kernel.Snapshot" {
            "taken"
        } else {
            "answered"
        };
        let mut result = SemanticCommandResult::undeclared();
        result.outcome = Some(
            serde_json::from_value(json!({"command": command, "outcome": outcome}))
                .map_err(|e| unavailable("naming outcome", e))?,
        );
        // Instrument check only: this deliberately lying target accepts everything, does nothing
        // and returns no state. Its reports are required to fail all named scenarios.
        let state = self.state()?;
        let runtime = self.open(&state)?;
        if self.no_op {
            result.consistency =
                Some(consistency(&runtime.read(None).map_err(|e| {
                    unavailable("reading inert fixture boundary", e)
                })?)?);
            return Ok(result);
        }
        let input = |name: &str| {
            request
                .input
                .get(name)
                .ok_or_else(|| unavailable("reading input", format!("missing {name}")))
        };
        let (event, payload) = match command.as_str() {
            "ekr.kernel.PreviewUpgrade" => {
                let preview = runtime
                    .preview_upgrade(&state.human.policy)
                    .map_err(|e| unavailable("previewing real upgrade", e))?;
                if json_input(input("target")?)? != value(&preview.to)? {
                    return Err(TargetError::unsupported(
                        command,
                        "requested target is not the runtime's supported authority",
                    ));
                }
                (
                    "ekr.kernel.PreviewUpgradeResult",
                    json!({"preview": preview}),
                )
            }
            "ekr.kernel.ApplyUpgrade" => {
                let preview: wire::EkrKernelUpgradePreview =
                    serde_json::from_value(json_input(input("preview")?)?)
                        .map_err(|e| unavailable("decoding generated preview", e))?;
                let mut proof = json_input(input("human_proof")?)?;
                if proof["intent"]["format"] != "HumanDecision1" {
                    return Err(unavailable(
                        "decoding proof",
                        "unknown semantic human-decision format",
                    ));
                }
                proof["intent"]["format"] = json!("ekr.human-decision/1");
                let proof: wire::EkrKernelSignedHumanDecision = serde_json::from_value(proof)
                    .map_err(|e| unavailable("decoding generated proof", e))?;
                let proof = human_review::proof_from_document(&proof).map_err(|e| {
                    unavailable("decoding signature", format!("{}: {}", e.code, e.reason))
                })?;
                let statement = json_input(input("statement")?)?;
                let statement = ekr_core::bytes::decode(
                    statement
                        .as_str()
                        .ok_or_else(|| unavailable("decoding statement", "expected base64"))?,
                )
                .map_err(|e| unavailable("decoding statement", e))?;
                let before = runtime
                    .published_events()
                    .map_err(|e| unavailable("reading publication boundary", e))?;
                let record = runtime
                    .apply_upgrade(&preview, &state.human.policy, &proof, &statement, || {
                        Timestamp::from_millis(1)
                    })
                    .map_err(|e| unavailable("applying real signed upgrade", e))?;
                // Observe an actual new durable event. A successful return cannot invent publication.
                for event in runtime
                    .published_events()
                    .map_err(|e| unavailable("reading durable events", e))?
                    .into_iter()
                    .skip(before.len())
                {
                    if event.name != "ekr.kernel.AuthorityUpgraded" {
                        continue;
                    }
                    let envelope: RevisionEvent = serde_json::from_value(event.data)
                        .map_err(|e| unavailable("decoding retained upgrade event", e))?;
                    if let RevisionPayload::AuthorityUpgraded {
                        transition_id,
                        revision_id,
                        number,
                        knowledge_root,
                    } = envelope.payload
                    {
                        result.direct_events.push(self.event(&request, "ekr.kernel.AuthorityUpgraded", json!({
                            "event_id": envelope.event_id.to_string(), "record_hash": envelope.record_hash.to_string(),
                            "transition_id": transition_id.to_string(), "revision_id": revision_id.to_string(),
                            "number": number.get(), "knowledge_root": knowledge_root.to_string(),
                        }))?);
                    }
                }
                (
                    "ekr.kernel.ApplyUpgradeResult",
                    json!({"transition_id": record.transition_id}),
                )
            }
            "ekr.kernel.ListAttention" => (
                "ekr.kernel.ListAttentionResult",
                json!({"items": runtime.attention().map_err(|e| unavailable("reading actual attention", e))?}),
            ),
            "ekr.kernel.Snapshot" => {
                let at = request
                    .input
                    .get("at")
                    .map(json_input)
                    .transpose()?
                    .and_then(|value| value.as_u64())
                    .map(RevisionNumber::new);
                let read = runtime
                    .read(at)
                    .map_err(|e| unavailable("reading original revision", e))?;
                (
                    "ekr.kernel.SnapshotTaken",
                    json!({"number": read.root.revision.get(), "knowledge_root": read.root.knowledge_root.to_string()}),
                )
            }
            _ => unreachable!(),
        };
        let observed = self.event(&request, event, payload)?;
        result.response = Some(observed.payload.clone());
        result.direct_events.push(observed);
        result.consistency =
            Some(consistency(&runtime.read(None).map_err(|e| {
                unavailable("reading completed command boundary", e)
            })?)?);
        Ok(result)
    }
    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        if !request.params.is_empty() {
            return Err(TargetError::unsupported(
                request.view.to_string(),
                "this fixture exposes only unparameterized views",
            ));
        }
        let state = self.state()?;
        let read = self
            .open(&state)?
            .read(None)
            .map_err(|e| unavailable("replaying persisted view", e))?;
        check_consistency(&read, &request.consistency)?;
        if self.no_op {
            return Ok(SemanticViewResult::default());
        }
        let rows = match request.view.to_string().as_str() {
            "ekr.graph.Assertions" | "ekr.graph.SettledAssertions" => {
                let settled = request.view.to_string() == "ekr.graph.SettledAssertions";
                let mut assertions: Vec<_> = read
                    .graph
                    .assertions
                    .values()
                    .filter(|claim| !settled || claim.is_current())
                    .collect();
                assertions
                    .sort_by_key(|claim| std::cmp::Reverse(claim.transaction_time.recorded_from));
                assertions
                    .into_iter()
                    .map(|claim| assertion_row(claim, settled))
                    .collect::<Result<Vec<_>, _>>()?
            }
            "ekr.kernel.Revisions" => revision_rows(&read)?,
            _ => {
                return Err(TargetError::unsupported(
                    request.view.to_string(),
                    "no native projection in upgrade target",
                ))
            }
        };
        Ok(SemanticViewResult::of(rows))
    }
    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        Ok(self
            .observed
            .borrow()
            .iter()
            .filter(|event| {
                event.event == request.event
                    && event.correlation.as_ref() == Some(&request.correlation)
            })
            .cloned()
            .collect())
    }
    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> Result<(), TargetError> {
        Err(TargetError::unsupported(
            request.force.to_string(),
            "this target never forces an outcome",
        ))
    }
    fn redeliver_event(&self, request: RedeliveryRequest) -> Result<(), TargetError> {
        Err(TargetError::unsupported(
            request.event.to_string(),
            "no redelivery binding",
        ))
    }
    fn end_scenario(&self, _: &ScenarioContext) -> Result<(), TargetError> {
        let state = self.state()?;
        let runtime = self.open(&state)?;
        let events = runtime
            .published_events()
            .map_err(|e| unavailable("reopening historical events", e))?;
        if !events.starts_with(&state.original_events) {
            return Err(unavailable(
                "checking historical publication",
                "original prefix changed",
            ));
        }
        for (hash, before) in &state.original_bytes {
            let after = runtime
                .content(hash)
                .map_err(|e| unavailable("reopening original bytes", e))?;
            if after.as_ref() != Some(before) {
                return Err(unavailable(
                    "checking historical content",
                    format!("original {hash} changed or disappeared"),
                ));
            }
        }
        Ok(())
    }
}

fn assertion_row(claim: &Assertion, settled: bool) -> Result<ViewRow, TargetError> {
    let assessment = match &claim.assessment {
        Assessment::Proposed => json!({"kind": "Proposed"}),
        Assessment::Validating {
            completed,
            required,
        } => json!({"kind": "Validating", "completed": completed, "required": required}),
        Assessment::Accepted { validators } => {
            json!({"kind": "Accepted", "validators": validators.iter().map(ToString::to_string).collect::<Vec<_>>()})
        }
        Assessment::Rejected { issues } => {
            json!({"kind": "Rejected", "issues": issues.iter().map(ToString::to_string).collect::<Vec<_>>()})
        }
        Assessment::Disputed {
            competing_assertions,
        } => {
            json!({"kind": "Disputed", "competing_assertions": competing_assertions.iter().map(|id| id.id().to_string()).collect::<Vec<_>>()})
        }
    };
    let lifecycle = match &claim.lifecycle {
        AssertionLifecycle::Active => json!({"kind": "Active"}),
        AssertionLifecycle::Retracted {
            at_revision,
            reason,
        } => {
            json!({"kind": "Retracted", "at_revision": at_revision.get(), "reason": reason.as_str()})
        }
        AssertionLifecycle::Superseded {
            by,
            at_revision,
            effective_from,
        } => {
            json!({"kind": "Superseded", "by": by.id().to_string(), "at_revision": at_revision.get(), "effective_from": instant(*effective_from)?})
        }
    };
    let mut row = json!({"assertion_id": claim.id.to_string(), "root_id": claim.root_id.to_string(),
        "assessment": assessment, "lifecycle": lifecycle, "recorded_from": instant(claim.transaction_time.recorded_from)?});
    if settled {
        use ekr_core::canonical::Canonical;
        use ekr_graph::{Object, Predicate, Subject};
        let (kind, id) = match claim.subject {
            Subject::Node(id) => ("Node", id.id().to_string()),
            Subject::Edge(id) => ("Edge", id.id().to_string()),
            Subject::Type(id) => ("Type", id.to_string()),
        };
        row["subject_kind"] = json!(kind);
        row["subject"] = json!(id);
        let (kind, id) = match claim.predicate {
            Predicate::Property(id) => ("Property", id.to_string()),
            Predicate::Relation(id) => ("Relation", id.to_string()),
        };
        row["predicate_kind"] = json!(kind);
        row["predicate"] = json!(id);
        let (kind, literal, reference) = match &claim.object {
            Object::Value(v) => (
                "Value",
                json!({"kind": ekr_ontology::Value::from(v.clone()).kind().to_string(), "canonical_bytes": ekr_core::bytes::encode(&v.canonical_bytes())}),
                Value::Null,
            ),
            Object::Node(id) => ("Node", Value::Null, json!(id.id().to_string())),
            Object::Type(id) => ("Type", Value::Null, json!(id.to_string())),
        };
        row["object_kind"] = json!(kind);
        row["object_value"] = literal;
        row["object_ref"] = reference;
        row["proposed_by"] = json!(claim.proposed_by.to_string());
        row["valid_from"] = json!(claim.valid_time.from.map(instant).transpose()?);
        row["valid_to"] = json!(claim.valid_time.to.map(instant).transpose()?);
        row["recorded_to"] = json!(claim
            .transaction_time
            .recorded_to
            .map(instant)
            .transpose()?);
    }
    map(row)
}
fn revision_rows(read: &VerifiedRead) -> Result<Vec<ViewRow>, TargetError> {
    let producers: BTreeMap<_, _> = read
        .transactions
        .values()
        .filter_map(|record| {
            record
                .committed
                .as_ref()
                .map(|receipt| (receipt.revision_id, record.proposal.transaction_id))
        })
        .collect();
    let mut parent = None;
    read.revisions.iter().map(|(number, revision)| {
        let row = map(json!({"revision_id": revision.revision_id.to_string(), "number": number.get(),
            "parent": parent, "ontology_root": revision.root.ontology_root.to_string(),
            "knowledge_root": revision.root.knowledge_root.to_string(), "evidence_root": revision.root.evidence_root.to_string(),
            "agent_root": revision.root.agent_root.to_string(), "transaction_id": producers.get(&revision.revision_id).map(ToString::to_string),
            "committed_at": instant(revision.committed_at)?, "state": "Committed"}));
        parent = Some(revision.revision_id.to_string());
        row
    }).collect()
}

#[test]
fn consistency_requires_the_same_verified_revision_record() {
    for provider in [Provider::File, Provider::Sqlite] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("store");
        let runtime = match provider {
            Provider::File => Runtime::file(
                &path,
                fixture::TENANT,
                fixture::context(),
                fixture::anchor(),
            ),
            Provider::Sqlite => Runtime::sqlite(
                &path,
                fixture::TENANT,
                fixture::context(),
                fixture::anchor(),
            ),
        }
        .unwrap();
        runtime
            .seed(fixture::seed(false), || Timestamp::EPOCH)
            .unwrap();
        let read = runtime.read(None).unwrap();
        check_consistency(
            &read,
            &QueryConsistency::at_least(consistency(&read).unwrap()),
        )
        .unwrap();
        for token in [
            "unrecognized".to_owned(),
            format!(
                "revision:1:{}",
                read.revisions[&RevisionNumber::SEED].record_hash
            ),
            format!("revision:0:{}", ContentHash::of_bytes(b"different history")),
        ] {
            assert!(check_consistency(
                &read,
                &QueryConsistency::at_least(ConsistencyToken::new(token).unwrap())
            )
            .is_err());
        }
    }
}
