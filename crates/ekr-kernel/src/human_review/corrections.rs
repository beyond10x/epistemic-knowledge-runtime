//! Deterministic ordinary operations for an exact reviewed answer; no publication authority.
use super::{corrections_bytes, digest, model as m, refuse, Refusal, VerifiedDecision};
use crate::{GraphOperation, Retraction, VerifiedRead};
use ekr_core::{contract_data as w, AgentId, AssertionId, ContentHash, EvidenceId, Timestamp};
use ekr_graph::{
    Assertion, AssertionLifecycle, Assessment, CanonicalGraph, Object, RetractionReason, Subject,
    TemporalRange, TransactionTime,
};
use std::collections::{BTreeMap, BTreeSet};

impl VerifiedDecision {
    /// Derive only the corrections in the immutable, exactly signed schema proposal.
    /// The caller separately verifies the effective review and its historical/continuation basis.
    /// Like attention derivation, this grants no transaction or publication authority; temporal
    /// additions retain the ordinary pre-publication EPOCH transaction-time placeholder.
    pub(crate) fn schema_correction_operations(
        &self,
        read: &VerifiedRead,
        proposal: &w::EkrIntegrateRetainedSchemaProposal,
        replacements: &[m::ClaimReplacement],
        statement_evidence: EvidenceId,
    ) -> Result<Vec<GraphOperation>, Refusal> {
        let m::HumanDecisionTarget::ApproveSchemaProposal(target) = &self.intent().target else {
            return Err(refuse("review-target", "proof is not a schema approval"));
        };
        let bytes = ekr_core::bytes::decode(&proposal.payload)
            .map_err(|_| invalid("invalid retained proposal bytes"))?;
        let document: w::EkrIntegrateSchemaProposalDocument = serde_json::from_slice(&bytes)
            .map_err(|_| invalid("invalid retained proposal document"))?;
        if document != *proposal.proposal
            || ContentHash::of_bytes(&bytes).to_string() != proposal.proposal_digest.0
            || target.proposal_digest.0 != proposal.proposal_digest.0
            || target.proposal_id.0 .0 != document.proposal_id.0
        {
            return Err(refuse(
                "review-target",
                "proposal differs from the signed exact bytes",
            ));
        }
        let corrections = document
            .corrections
            .iter()
            .map(|c| super::correction_from_document(c))
            .collect::<Result<Vec<_>, _>>()?;
        corrections_bytes(&corrections)?;
        if corrections
            .iter()
            .any(|c| c.kind == m::ClaimCorrectionKind::Unresolved)
        {
            return Err(refuse(
                "schema-correction-unresolved",
                "selected uncertainty remains pending; no correction effects were derived",
            ));
        }
        let operator = self
            .operator()
            .actor
            .0
             .0
            .parse()
            .map_err(|_| invalid("invalid operator"))?;
        schema_derive(
            read,
            &document,
            &corrections,
            replacements,
            operator,
            statement_evidence,
        )
    }

    /// Derive the operations of this exact answer against a captured review basis.
    /// Replacement identities are supplied once by the publisher and retained in the answer.
    /// This neither validates a transaction nor grants publication authority: the command must
    /// revalidate the host enrollment, predecessor, current basis and operations atomically.
    pub fn correction_operations(
        &self,
        read: &VerifiedRead,
        corrections: &[m::ClaimCorrection],
        replacements: &[m::ClaimReplacement],
        statement_evidence: EvidenceId,
    ) -> Result<Vec<GraphOperation>, Refusal> {
        let m::HumanDecisionTarget::AnswerAttention(target) = &self.intent().target else {
            return Err(refuse("review-target", "proof is not an attention answer"));
        };
        if digest(&corrections_bytes(corrections)?).to_string() != target.corrections_digest.0 {
            return Err(refuse(
                "review-target",
                "corrections differ from the signed answer",
            ));
        }
        let items = read
            .dispute_attention()
            .map_err(|_| invalid("cannot project current dispute"))?;
        let current = items
            .iter()
            .map(crate::attention_behavior::item)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| invalid("invalid dispute projection"))?
            .into_iter()
            .find(|item| item.subject.dispute_id.as_ref() == Some(&target.dispute_id))
            .ok_or_else(|| {
                refuse(
                    "answer-review-required",
                    "reviewed dispute is no longer current",
                )
            })?;
        self.corrections_at(
            &read.graph,
            &current,
            corrections,
            replacements,
            statement_evidence,
        )
    }
    pub(crate) fn corrections_at(
        &self,
        graph: &CanonicalGraph,
        current: &m::AttentionItem,
        corrections: &[m::ClaimCorrection],
        replacements: &[m::ClaimReplacement],
        statement_evidence: EvidenceId,
    ) -> Result<Vec<GraphOperation>, Refusal> {
        let m::HumanDecisionTarget::AnswerAttention(target) = &self.intent().target else {
            return Err(refuse("review-target", "proof is not an attention answer"));
        };
        if current.subject.dispute_id.as_ref() != Some(&target.dispute_id)
            || digest(&corrections_bytes(corrections)?).to_string() != target.corrections_digest.0
        {
            return Err(refuse(
                "review-target",
                "corrections or dispute differ from the signed answer",
            ));
        }
        let old = &target.basis;
        let now = &current.basis;
        if old.observed_revision.0 > now.observed_revision.0
            || old.evidence_digest != now.evidence_digest
            || old.options_digest != now.options_digest
            || old.effects_digest != now.effects_digest
        {
            return Err(refuse(
                "answer-review-required",
                "reviewed evidence, options or effects changed",
            ));
        }
        let claims = current
            .claims
            .iter()
            .map(|id| parse(&id.0 .0))
            .collect::<Result<_, _>>()?;
        let operator = self
            .operator()
            .actor
            .0
             .0
            .parse()
            .map_err(|_| invalid("invalid operator"))?;
        derive(
            graph,
            &claims,
            corrections,
            replacements,
            operator,
            statement_evidence,
        )
    }
}

/// Check the whole proposal before partitioning: per-dispute validation alone cannot detect a
/// replacement UUID reused across two otherwise valid independent temporal corrections.
fn schema_replacements(
    graph: &CanonicalGraph,
    corrections: &[m::ClaimCorrection],
    replacements: &[m::ClaimReplacement],
) -> Result<BTreeMap<AssertionId, m::ClaimReplacement>, Refusal> {
    let mut selected = BTreeSet::new();
    let mut temporal = BTreeSet::new();
    for correction in corrections {
        let id = parse(&correction.assertion_id.0 .0)?;
        if !selected.insert(id) {
            return Err(invalid("multiple corrections name one claim"));
        }
        if correction.kind == m::ClaimCorrectionKind::CorrectTime {
            temporal.insert(id);
        }
    }
    let mut mapped = BTreeMap::new();
    let mut fresh = BTreeSet::new();
    for replacement in replacements {
        let old = parse(&replacement.previous.0 .0)?;
        let new = parse(&replacement.replacement.0 .0)?;
        if !temporal.contains(&old)
            || graph.assertions.contains_key(&new)
            || !fresh.insert(new)
            || mapped.insert(old, replacement.clone()).is_some()
        {
            return Err(invalid("replacement identities must be globally fresh, distinct and exactly cover temporal corrections"));
        }
    }
    if mapped.len() != temporal.len() {
        return Err(invalid(
            "temporal correction lacks its fresh replacement identity",
        ));
    }
    Ok(mapped)
}

fn schema_derive(
    read: &VerifiedRead,
    document: &w::EkrIntegrateSchemaProposalDocument,
    corrections: &[m::ClaimCorrection],
    replacements: &[m::ClaimReplacement],
    operator: AgentId,
    statement: EvidenceId,
) -> Result<Vec<GraphOperation>, Refusal> {
    let replacements = schema_replacements(&read.graph, corrections, replacements)?;
    let mut assigned = BTreeSet::new();
    let mut operations = BTreeMap::new();
    let components = crate::schema_proposal_corrections::components(read, document)
        .map_err(|_| invalid("cannot project current disputes"))?;
    for component in components {
        let claims = component
            .claims
            .iter()
            .map(|id| parse(&id.0))
            .collect::<Result<BTreeSet<_>, _>>()?;
        let mut selected = Vec::new();
        let mut local_replacements = Vec::new();
        for correction in corrections {
            let id = parse(&correction.assertion_id.0 .0)?;
            if !claims.contains(&id) {
                continue;
            }
            if !assigned.insert(id) {
                return Err(invalid("claim belongs to multiple correction components"));
            }
            selected.push(correction.clone());
            if let Some(replacement) = replacements.get(&id) {
                local_replacements.push(replacement.clone());
            }
        }
        for operation in derive(
            &read.graph,
            &claims,
            &selected,
            &local_replacements,
            operator,
            statement,
        )? {
            let key = match &operation {
                GraphOperation::RetractAssertion(row) => (0, row.assertion),
                GraphOperation::AddAssertion(row) => (1, row.id),
                _ => return Err(invalid("unsupported correction effect")),
            };
            if operations.insert(key, operation).is_some() {
                return Err(invalid("duplicate correction effect across disputes"));
            }
        }
    }
    if assigned.len() != corrections.len() {
        return Err(invalid(
            "corrections must name distinct active claims in current disputes",
        ));
    }
    Ok(operations.into_values().collect())
}

fn invalid(reason: &str) -> Refusal {
    refuse("answer-correction", reason)
}
fn parse(text: &str) -> Result<AssertionId, Refusal> {
    text.parse()
        .map_err(|_| invalid("invalid assertion identity"))
}
fn interval(correction: &m::ClaimCorrection) -> Result<TemporalRange, Refusal> {
    let bound = |value: &Option<ekr_core::contracts::primitives::Timestamp>| {
        value
            .as_ref()
            .map(|value| {
                crate::incubation_document::timestamp(&value.0)
                    .map_err(|_| invalid("invalid correction instant"))
            })
            .transpose()
    };
    TemporalRange::new(bound(&correction.valid_from)?, bound(&correction.valid_to)?)
        .ok_or_else(|| invalid("inverted correction interval"))
}
fn instructions<'a>(
    graph: &CanonicalGraph,
    claims: &BTreeSet<AssertionId>,
    corrections: &'a [m::ClaimCorrection],
) -> Result<BTreeMap<AssertionId, &'a m::ClaimCorrection>, Refusal> {
    if corrections.is_empty() {
        return Err(invalid(
            "an answer needs a correction or explicit uncertainty",
        ));
    }
    let mut instructions = BTreeMap::new();
    for correction in corrections {
        let id = parse(&correction.assertion_id.0 .0)?;
        let claim = graph
            .assertions
            .get(&id)
            .filter(|claim| {
                claims.contains(&id)
                    && matches!(claim.lifecycle, AssertionLifecycle::Active)
                    && claim.transaction_time.is_open()
                    && matches!(claim.assessment, Assessment::Disputed { .. })
            })
            .ok_or_else(|| invalid("correction must name an active claim in this dispute"))?;
        if instructions.insert(claim.id, correction).is_some() {
            return Err(invalid("multiple corrections name one claim"));
        }
        if correction.reason.trim().is_empty() {
            return Err(invalid("a correction needs an explanation"));
        }
        if correction.kind != m::ClaimCorrectionKind::CorrectTime
            && (correction.valid_from.is_some() || correction.valid_to.is_some())
        {
            return Err(invalid(
                "only a temporal correction may specify time bounds",
            ));
        }
        if correction.kind == m::ClaimCorrectionKind::Unresolved && corrections.len() != 1 {
            return Err(invalid("uncertainty cannot be combined with claim changes"));
        }
    }
    for (id, correction) in &instructions {
        match correction.kind {
            m::ClaimCorrectionKind::Choose => {
                let Assessment::Disputed {
                    competing_assertions,
                } = &graph.assertions[id].assessment
                else {
                    unreachable!()
                };
                for peer in competing_assertions {
                    let peer = peer.id();
                    if !claims.contains(&peer) {
                        return Err(invalid("competitor is outside the reviewed dispute"));
                    }
                    if instructions
                        .get(&peer)
                        .is_some_and(|c| c.kind != m::ClaimCorrectionKind::Retract)
                    {
                        return Err(invalid(
                            "chosen claim conflicts with another requested effect",
                        ));
                    }
                }
            }
            m::ClaimCorrectionKind::CorrectTime => {
                if interval(correction)? == graph.assertions[id].valid_time {
                    return Err(invalid("temporal correction does not change the interval"));
                }
            }
            m::ClaimCorrectionKind::Retract | m::ClaimCorrectionKind::Unresolved => {}
        }
    }
    Ok(instructions)
}

/// Validate applicability without allocating replacement identities or claiming a signed answer.
pub(crate) fn validate_corrections(
    graph: &CanonicalGraph,
    claims: &BTreeSet<AssertionId>,
    corrections: &[m::ClaimCorrection],
) -> Result<(), Refusal> {
    corrections_bytes(corrections)?;
    instructions(graph, claims, corrections).map(|_| ())
}

fn derive(
    graph: &CanonicalGraph,
    claims: &BTreeSet<AssertionId>,
    corrections: &[m::ClaimCorrection],
    replacements: &[m::ClaimReplacement],
    operator: AgentId,
    statement: EvidenceId,
) -> Result<Vec<GraphOperation>, Refusal> {
    let instructions = instructions(graph, claims, corrections)?;
    let mut mapped = BTreeMap::new();
    let mut new_ids = BTreeSet::new();
    for replacement in replacements {
        let old = parse(&replacement.previous.0 .0)?;
        let new = parse(&replacement.replacement.0 .0)?;
        if !instructions
            .get(&old)
            .is_some_and(|c| c.kind == m::ClaimCorrectionKind::CorrectTime)
            || graph.assertions.contains_key(&new)
            || !new_ids.insert(new)
            || mapped.insert(old, new).is_some()
        {
            return Err(invalid(
                "replacement identities must be fresh and exactly match temporal corrections",
            ));
        }
    }
    let mut retract = BTreeMap::new();
    let mut additions = BTreeMap::new();
    for (id, correction) in &instructions {
        match correction.kind {
            m::ClaimCorrectionKind::Unresolved => {}
            m::ClaimCorrectionKind::Choose => {
                let Assessment::Disputed {
                    competing_assertions,
                } = &graph.assertions[id].assessment
                else {
                    unreachable!()
                };
                for peer in competing_assertions {
                    let peer = peer.id();
                    if !claims.contains(&peer) {
                        return Err(invalid("competitor is outside the reviewed dispute"));
                    }
                    if instructions
                        .get(&peer)
                        .is_some_and(|c| c.kind != m::ClaimCorrectionKind::Retract)
                    {
                        return Err(invalid(
                            "chosen claim conflicts with another requested effect",
                        ));
                    }
                    // If several supported choices withdraw one shared competitor, identity order
                    // selects a stable explanation; an explicit retraction supplies its own reason.
                    retract
                        .entry(peer)
                        .or_insert_with(|| correction.reason.clone());
                }
            }
            m::ClaimCorrectionKind::Retract => {
                retract.insert(*id, correction.reason.clone());
            }
            m::ClaimCorrectionKind::CorrectTime => {
                let replacement = mapped.get(id).ok_or_else(|| {
                    invalid("temporal correction lacks its fresh replacement identity")
                })?;
                let old = &graph.assertions[id];
                let time = interval(correction)?;
                if time == old.valid_time {
                    return Err(invalid("temporal correction does not change the interval"));
                }
                let mut evidence: BTreeSet<_> = old.evidence.iter().map(|e| e.id()).collect();
                evidence.extend(graph.attached(*id).map(|e| e.evidence.id()));
                evidence.insert(statement);
                let subject = match old.subject {
                    Subject::Node(id) => Subject::Node(id.id()),
                    Subject::Edge(id) => Subject::Edge(id.id()),
                    Subject::Type(id) => Subject::Type(id),
                };
                let object = match &old.object {
                    Object::Value(value) => Object::Value(value.clone().into()),
                    Object::Node(node) => Object::Node(node.id()),
                    Object::Type(id) => Object::Type(*id),
                };
                additions.insert(
                    *id,
                    GraphOperation::AddAssertion(Box::new(Assertion {
                        id: *replacement,
                        root_id: old.root_id,
                        subject,
                        predicate: old.predicate,
                        object,
                        evidence,
                        proposed_by: operator,
                        assessment: Assessment::Proposed,
                        lifecycle: AssertionLifecycle::Active,
                        valid_time: time,
                        transaction_time: TransactionTime::since(Timestamp::EPOCH),
                    })),
                );
                retract.insert(*id, correction.reason.clone());
            }
        }
    }
    // Explicit reasons prevail independent of assertion identity and input order.
    for (id, correction) in instructions {
        if correction.kind == m::ClaimCorrectionKind::Retract {
            retract.insert(id, correction.reason.clone());
        }
    }
    let mut operations: Vec<_> = retract
        .into_iter()
        .map(|(assertion, reason)| {
            GraphOperation::RetractAssertion(Retraction {
                assertion,
                reason: RetractionReason::new(reason),
            })
        })
        .collect();
    operations.extend(additions.into_values());
    Ok(operations)
}
