//! Schema evidence scenario: SDK-built input, real kernel commands, actual rendered output.
use super::*;
use ekr_kernel::{CommitCommandResult, ValidationCommandResult};
use ekr_sdk::document::{
    Confidence, EvidenceAddition, EvidenceSource, NodeType, TransactionBuilder,
};

pub(super) fn prepare(
    runtime: &Runtime,
    human: &fixture::Human,
    directory: &Path,
) -> Result<(), TargetError> {
    let preview = runtime
        .preview_upgrade(&human.policy)
        .map_err(|e| unavailable("previewing schema fixture upgrade", e))?;
    let proof = human_review::proof_from_document(&human.proof(&preview))
        .map_err(|e| unavailable("decoding fixture proof", e.reason))?;
    runtime
        .apply_upgrade(&preview, &human.policy, &proof, fixture::STATEMENT, || {
            Timestamp::from_millis(1)
        })
        .map_err(|e| unavailable("upgrading schema fixture", e))?;
    let mut ty = NodeType::new("HealthObservation");
    ty.id = fixture::id(702);
    let mut addition = EvidenceAddition::new(
        EvidenceSource::human("fixture"),
        fixture::context().operator,
        Timestamp::from_millis(2),
        Confidence::CERTAIN,
        b"Reviewed schema evidence".to_vec(),
    );
    addition.evidence.id = fixture::id(703);
    let document = TransactionBuilder::new(fixture::context().operator)
        .with_id(fixture::id(701))
        .with_schema_version(fixture::id(700))
        .with_schema_evidence([fixture::id(500)])
        .push(ty.into())
        .push(addition.into())
        .build()
        .map_err(|e| unavailable("building SDK schema fixture", e))?;
    std::fs::write(
        directory.join("schema-evidence.yaml"),
        document
            .to_yaml()
            .map_err(|e| unavailable("encoding SDK document", e))?,
    )
    .map_err(|e| unavailable("staging schema input", e))?;
    Ok(())
}

pub(super) fn execute(
    target: &UpgradeTarget,
    request: &SemanticCommandRequest,
) -> Result<SemanticCommandResult, TargetError> {
    let command = request.command.to_string();
    let (actor, accepted) = match command.as_str() {
        "ekr.kernel.Propose" => ("ekr.kernel.Proposer", "proposed"),
        "ekr.kernel.Validate" => ("ekr.kernel.Validator", "validated"),
        "ekr.kernel.Commit" => ("ekr.kernel.Committer", "committed"),
        "ekr.views.ProjectGraph" => ("ekr.views.Reader", "projected"),
        _ => unreachable!(),
    };
    if request
        .actor
        .as_ref()
        .is_some_and(|value| value.to_string() != actor)
    {
        return Err(TargetError::unsupported(
            command,
            "fixture does not grant this actor",
        ));
    }
    let state = target.state()?;
    let runtime = target.open(&state)?;
    let before = runtime
        .published_events()
        .map_err(|e| unavailable("reading command boundary", e))?;
    let mut result = SemanticCommandResult::undeclared();
    let mut outcome = accepted;
    if !target.no_op {
        let input = |name: &str| {
            request
                .input
                .get(name)
                .ok_or_else(|| unavailable("reading schema command", format!("missing {name}")))
                .and_then(json_input)
        };
        let transaction = || {
            input("transaction_id")?
                .as_str()
                .ok_or_else(|| unavailable("decoding transaction", "not a UUID string"))?
                .parse::<ekr_core::TransactionId>()
                .map_err(|e| unavailable("decoding transaction", e))
        };
        match command.as_str() {
            "ekr.kernel.Propose" => {
                if input("transaction_document")? != json!("schema-evidence.yaml") {
                    return Err(TargetError::unsupported(
                        command,
                        "fixture only stages schema-evidence.yaml",
                    ));
                }
                let file =
                    std::fs::File::open(state.path.parent().unwrap().join("schema-evidence.yaml"))
                        .map_err(|e| unavailable("opening supplied document", e))?;
                runtime
                    .propose_reader(file, fixture::context().operator, || {
                        Timestamp::from_millis(3)
                    })
                    .map_err(|e| unavailable("proposing real schema document", e))?;
            }
            "ekr.kernel.Validate" => {
                let against = input("against")?
                    .as_u64()
                    .ok_or_else(|| unavailable("decoding against", "not unsigned"))?;
                outcome = match runtime
                    .validate(transaction()?, RevisionNumber::new(against), || {
                        Timestamp::from_millis(4)
                    })
                    .map_err(|e| unavailable("validating schema proposal", e))?
                {
                    ValidationCommandResult::Validated(_) => "validated",
                    ValidationCommandResult::Rejected(_) => "rejected",
                };
            }
            "ekr.kernel.Commit" => {
                outcome = match runtime
                    .commit(transaction()?, fixture::context().operator, || {
                        Timestamp::from_millis(5)
                    })
                    .map_err(|e| unavailable("committing schema proposal", e))?
                {
                    CommitCommandResult::Committed(_) => "committed",
                    CommitCommandResult::Stale(_) => "stale",
                };
            }
            "ekr.views.ProjectGraph" => {
                if input("store")? != json!("schema-store") {
                    return Err(TargetError::unsupported(command, "unknown fixture store"));
                }
                let at = request
                    .input
                    .get("at")
                    .map(json_input)
                    .transpose()?
                    .and_then(|n| n.as_u64())
                    .map(RevisionNumber::new);
                let rendered = ekr_views::project(&runtime, at)
                    .map_err(|e| unavailable("rendering persisted schema history", e))?;
                let s = rendered.summary;
                let event = target.event(request, "ekr.views.GraphProjected", json!({
                    "revision":s.revision,"nodes":s.nodes,"edges":s.edges,"assertions":s.assertions,"evidence":s.evidence,
                    "schema_versions":s.schema_versions,"revisions":s.revisions,"transactions":s.transactions,
                    "node_types":s.node_types,"edge_types":s.edge_types,"properties":s.properties,
                    "edge_assertions":s.edge_assertions,"retracted_assertions":s.retracted_assertions,
                    "retained_evidence":s.retained_evidence,"projection_hash":s.projection_hash,
                    "supporting_evidence":s.supporting_evidence
                }))?;
                result.direct_events.push(event);
            }
            _ => unreachable!(),
        }
        // Only occurrences actually appended by the command are exposed. Reopening below
        // validates their records; no successful return can invent a durable event.
        for event in runtime
            .published_events()
            .map_err(|e| unavailable("reading completed publications", e))?
            .into_iter()
            .skip(before.len())
        {
            if !event.name.starts_with("ekr.kernel.") {
                continue;
            }
            let envelope: RevisionEvent = serde_json::from_value(event.data)
                .map_err(|e| unavailable("decoding occurrence", e))?;
            let mut payload = match envelope.payload {
                RevisionPayload::TransactionProposed {
                    transaction_id,
                    proposer,
                    operations_hash,
                } => {
                    json!({"transaction_id":transaction_id,"proposer":proposer,"operations_hash":operations_hash})
                }
                RevisionPayload::TransactionValidated {
                    transaction_id,
                    against,
                    validation_hash,
                } => {
                    json!({"transaction_id":transaction_id,"against":against.get(),"validation_hash":validation_hash})
                }
                RevisionPayload::TransactionRejected {
                    transaction_id,
                    issues,
                } => json!({"transaction_id":transaction_id,"issues":issues}),
                RevisionPayload::TransactionStale {
                    transaction_id,
                    validated_against,
                    current,
                } => {
                    json!({"transaction_id":transaction_id,"validated_against":validated_against.get(),"current":current.get()})
                }
                RevisionPayload::RevisionCommitted {
                    transaction_id,
                    revision_id,
                    number,
                    knowledge_root,
                } => {
                    json!({"transaction_id":transaction_id,"revision_id":revision_id,"number":number.get(),"knowledge_root":knowledge_root})
                }
                _ => {
                    return Err(unavailable(
                        "projecting schema occurrence",
                        "unexpected kind",
                    ))
                }
            };
            payload["event_id"] = json!(envelope.event_id);
            payload["record_hash"] = json!(envelope.record_hash);
            result
                .direct_events
                .push(target.event(request, &event.name, payload)?);
        }
    }
    result.outcome = Some(
        serde_json::from_value(json!({"command": command, "outcome": outcome}))
            .map_err(|e| unavailable("naming schema outcome", e))?,
    );
    result.consistency =
        Some(consistency(&runtime.read(None).map_err(|e| {
            unavailable("verifying completed schema command", e)
        })?)?);
    Ok(result)
}
