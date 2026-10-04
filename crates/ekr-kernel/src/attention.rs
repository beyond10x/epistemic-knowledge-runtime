//! Read-only questions derived from admitted disputes and retained integration gaps.
use crate::{Commit, VerifiedRead};
use ekr_core::contract_data::*;
use ekr_core::{AssertionId, ContentHash, EvidenceId, ObservationId, RevisionNumber};
use ekr_graph::{Assertion, AssertionLifecycle, Assessment, EvidenceSource, Predicate, Subject};
use ekr_store::{
    HumanDecisionRetention, IncubationRetention, ObjectStore, ObservationRetention,
    ProposalReviewRetention, RevisionLog, SchemaProposalRetention, StoreError,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn error(detail: impl std::fmt::Display) -> StoreError {
    StoreError::Document(format!("attention-refused: {detail}"))
}
fn digest(
    domain: &str,
    material: &impl Serialize,
) -> Result<Box<EkrKernelContentHash>, StoreError> {
    let bytes = serde_json::to_vec(&(domain, material)).map_err(error)?;
    Ok(Box::new(EkrKernelContentHash(
        ContentHash::of_bytes(&bytes).to_string(),
    )))
}
fn subject(kind: EkrKernelAttentionKind) -> EkrKernelAttentionSubject {
    EkrKernelAttentionSubject {
        kind: Box::new(kind),
        dispute_id: EssPresence::Absent,
        blocker_id: EssPresence::Absent,
        proposal_id: EssPresence::Absent,
    }
}
fn checked_subject(value: &EkrKernelAttentionSubject) -> Result<(), StoreError> {
    let id = match (
        &*value.kind,
        &value.dispute_id,
        &value.blocker_id,
        &value.proposal_id,
    ) {
        (
            EkrKernelAttentionKind::V0,
            EssPresence::Absent,
            EssPresence::Present(id),
            EssPresence::Absent,
        ) => &id.0,
        (
            EkrKernelAttentionKind::V1,
            EssPresence::Present(id),
            EssPresence::Absent,
            EssPresence::Absent,
        ) => &id.0,
        (
            EkrKernelAttentionKind::V2,
            EssPresence::Absent,
            EssPresence::Absent,
            EssPresence::Present(id),
        ) => &id.0,
        _ => return Err(error("subject kind and identity disagree")),
    };
    id.parse::<AssertionId>().map_err(error)?;
    Ok(())
}

impl<
        S: RevisionLog
            + ObjectStore
            + ObservationRetention
            + IncubationRetention
            + HumanDecisionRetention
            + ProposalReviewRetention
            + SchemaProposalRetention,
    > Commit<S>
{
    /// Derive unresolved questions from current admitted state; this never creates queue state.
    /// # Errors
    /// Invalid canonical history, missing retained evidence or provider failure.
    pub fn attention(&self) -> Result<Vec<EkrKernelAttentionItem>, StoreError> {
        super::attention_behavior::list(self)
    }

    /// Inspect a current question by its typed identity. A changed component can retire a handle.
    /// # Errors
    /// A mismatched or missing identity, invalid retained data or provider failure.
    pub fn attention_item(
        &self,
        subject: &EkrKernelAttentionSubject,
    ) -> Result<EkrKernelAttentionItem, StoreError> {
        checked_subject(subject)?;
        super::attention_behavior::show(self, subject)
    }

    pub(super) fn project_attention(&self) -> Result<Vec<EkrKernelAttentionItem>, StoreError> {
        let read = self
            .head()?
            .map(|_| self.read(None))
            .transpose()
            .map_err(error)?;
        let revision = read
            .as_ref()
            .map_or(RevisionNumber::SEED, |r| r.root.revision);
        let mut items = match &read {
            Some(read) => disputes(read)?,
            None => Vec::new(),
        };
        for version in self.retained_interpretation_versions()? {
            let interpretation = self.retained_interpretation(&version)?;
            let mut observations = BTreeMap::new();
            for id in &interpretation.document.observations {
                observations.insert(
                    id.0.clone(),
                    self.observation(id.0.parse().map_err(error)?)?,
                );
            }
            for blocker in &interpretation.blockers {
                // Preserve the historical finding, but hide a question once the exact source
                // item has a verified mapping commit in this capture's application history.
                if read
                    .as_ref()
                    .is_some_and(|read| read.integrated_source_item(&version, &blocker.item))
                    || interpretation.receipts.iter().any(|receipt| {
                        receipt.item == blocker.item
                            && matches!(
                                *receipt.disposition,
                                EkrIntegrateProcessingDisposition::V0
                                    | EkrIntegrateProcessingDisposition::V1
                                    | EkrIntegrateProcessingDisposition::V3
                            )
                    })
                {
                    continue;
                }
                let mut subject = subject(EkrKernelAttentionKind::V0);
                subject.blocker_id = EssPresence::Present(blocker.blocker_id.clone());
                let evidence = interpretation
                    .document
                    .evidence
                    .iter()
                    .map(|value| (value.evidence.id.0.clone(), value.evidence.id.clone()))
                    .collect::<BTreeMap<_, _>>()
                    .into_values()
                    .collect();
                items.push(EkrKernelAttentionItem {
                    subject: Box::new(subject),
                    question: format!(
                        "How should {} ({}) be represented? {}",
                        blocker.declaration, blocker.item, blocker.reason
                    ),
                    basis: Box::new(EkrKernelReviewBasis {
                        observed_revision: Box::new(EkrKernelRevisionNumber(revision.get().into())),
                        evidence_digest: digest(
                            "ekr.attention.blocker-evidence/1",
                            &(&version, &interpretation.document.evidence, &observations),
                        )?,
                        options_digest: digest(
                            "ekr.attention.blocker-options/1",
                            &(&version, blocker),
                        )?,
                        effects_digest: digest(
                            "ekr.attention.blocker-effects/1",
                            &(&version, &blocker.item, &interpretation.receipts),
                        )?,
                    }),
                    claims: Vec::new(),
                    evidence,
                    observations: observations
                        .keys()
                        .map(|id| Box::new(EkrGraphObservationId(id.clone())))
                        .collect(),
                });
            }
        }
        for retained in self.store.retained_schema_proposals()? {
            let proposal = self.retained_schema_proposal(&retained.proposal.proposal_id)?;
            if proposal
                .reviews
                .last()
                .is_some_and(|review| matches!(*review.decision, EkrIntegrateReviewDecision::V1))
            {
                continue;
            }
            if let Some(prefix) = read.as_ref().and_then(|read| {
                read.application_prefixes
                    .get(&proposal.proposal.proposal_id.0)
            }) {
                let remaining = prefix.remaining(&proposal.proposal)?;
                if prefix.schema_done()
                    && remaining.mappings.is_empty()
                    && remaining.corrections.is_empty()
                {
                    continue;
                }
            }
            let mut subject = subject(EkrKernelAttentionKind::V2);
            subject.proposal_id = EssPresence::Present(proposal.proposal.proposal_id.clone());
            let mut observations: BTreeSet<_> = proposal
                .proposal
                .observations
                .iter()
                .map(|id| id.0.clone())
                .collect();
            let mut evidence: BTreeSet<_> = proposal
                .proposal
                .evidence
                .iter()
                .map(|id| id.0.clone())
                .collect();
            for source in &proposal.proposal.sources {
                let source = self.retained_interpretation(&source.version)?;
                observations.extend(source.document.observations.iter().map(|id| id.0.clone()));
                evidence.extend(
                    source
                        .document
                        .evidence
                        .iter()
                        .map(|entry| entry.evidence.id.0.clone()),
                );
            }
            items.push(EkrKernelAttentionItem {
                subject: Box::new(subject),
                question: if proposal.reviews.last().is_some_and(|review| {
                    matches!(*review.decision, EkrIntegrateReviewDecision::V0)
                }) {
                    format!(
                        "What still needs integration or renewed review for this approved proposal? {}",
                        proposal.proposal.explanation
                    )
                } else {
                    format!(
                        "Should this vocabulary proposal be approved? {}",
                        proposal.proposal.explanation
                    )
                },
                basis: proposal.basis,
                claims: Vec::new(),
                evidence: evidence
                    .into_iter()
                    .map(|id| Box::new(EkrGraphEvidenceId(id)))
                    .collect(),
                observations: observations
                    .into_iter()
                    .map(|id| Box::new(EkrGraphObservationId(id)))
                    .collect(),
            });
        }
        items.sort_by_key(|item| match &*item.subject.kind {
            EkrKernelAttentionKind::V0 => (
                0,
                match &item.subject.blocker_id {
                    EssPresence::Present(id) => id.0.clone(),
                    _ => unreachable!(),
                },
            ),
            EkrKernelAttentionKind::V1 => (
                1,
                match &item.subject.dispute_id {
                    EssPresence::Present(id) => id.0.clone(),
                    _ => unreachable!(),
                },
            ),
            EkrKernelAttentionKind::V2 => (
                2,
                match &item.subject.proposal_id {
                    EssPresence::Present(id) => id.0.clone(),
                    _ => unreachable!(),
                },
            ),
        });
        Ok(items)
    }
}

impl VerifiedRead {
    /// Dispute questions from this exact immutable read boundary, excluding independent
    /// incubation/proposal streams. Useful when rendering claims and questions together.
    /// # Errors
    /// Missing evidence or inconsistent competing references in the selected graph.
    pub fn dispute_attention(&self) -> Result<Vec<EkrKernelAttentionItem>, StoreError> {
        disputes(self)
    }
}

fn disputes(read: &VerifiedRead) -> Result<Vec<EkrKernelAttentionItem>, StoreError> {
    disputes_at(&read.graph, read.root.revision, |hash| {
        read.content(hash)
            .ok_or_else(|| error("missing retained evidence bytes"))
    })
}

pub(crate) fn disputes_at<'a>(
    graph: &ekr_graph::CanonicalGraph,
    revision: RevisionNumber,
    content: impl Fn(&ContentHash) -> Result<&'a [u8], StoreError>,
) -> Result<Vec<EkrKernelAttentionItem>, StoreError> {
    let mut adjacency = BTreeMap::<AssertionId, BTreeSet<AssertionId>>::new();
    for claim in graph.assertions.values() {
        if !matches!(claim.lifecycle, AssertionLifecycle::Active)
            || !claim.transaction_time.is_open()
        {
            continue;
        }
        if let Assessment::Disputed {
            competing_assertions,
        } = &claim.assessment
        {
            for other in competing_assertions {
                let peer = graph
                    .assertions
                    .get(&other.id())
                    .ok_or_else(|| error("dangling competing claim"))?;
                if peer.id == claim.id
                    || peer.subject != claim.subject
                    || peer.predicate != claim.predicate
                    || !matches!(&peer.assessment, Assessment::Disputed { competing_assertions } if competing_assertions.iter().any(|id| id.id() == claim.id))
                {
                    return Err(error("inconsistent competing claims"));
                }
                adjacency.entry(claim.id).or_default().insert(peer.id);
            }
        }
    }
    let mut visited = BTreeSet::new();
    let mut items = Vec::new();
    for start in adjacency.keys() {
        if visited.contains(start) {
            continue;
        }
        let mut pending = vec![*start];
        let mut component = BTreeSet::new();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            component.insert(id);
            pending.extend(
                adjacency
                    .get(&id)
                    .ok_or_else(|| error("missing competing claim"))?,
            );
        }
        let claims: Vec<&Assertion> = component.iter().map(|id| &graph.assertions[id]).collect();
        let first = claims.first().ok_or_else(|| error("empty dispute"))?;
        let mut subject = subject(EkrKernelAttentionKind::V1);
        subject.dispute_id =
            EssPresence::Present(Box::new(EkrKernelDisputeId(first.id.to_string())));
        let mut evidence_ids = BTreeSet::<EvidenceId>::new();
        let mut support = Vec::new();
        for claim in &claims {
            evidence_ids.extend(claim.evidence.iter().map(|e| e.id()));
            evidence_ids.extend(graph.attached(claim.id).map(|a| a.evidence.id()));
            support.push((
                claim.id,
                &claim.evidence,
                graph.attached(claim.id).collect::<Vec<_>>(),
            ));
        }
        let mut evidence_material = Vec::new();
        let mut observation_ids = BTreeSet::<ObservationId>::new();
        for id in &evidence_ids {
            let evidence = graph
                .evidence
                .get(id)
                .ok_or_else(|| error("missing claim evidence"))?;
            let payload = content(&evidence.content_hash)?;
            evidence_material.push((evidence, payload));
            if let EvidenceSource::Observation(id) = evidence.source {
                observation_ids.insert(id);
            }
        }
        let (name, declaration) = match first.predicate {
            Predicate::Property(id) => {
                let definition = match first.subject {
                    Subject::Node(node) => graph.nodes.get(&node.id()).and_then(|node| {
                        graph
                            .ontology
                            .properties_of(node.type_id)
                            .get(&id)
                            .map(|definition| (**definition).clone())
                    }),
                    Subject::Edge(edge) => graph
                        .edges
                        .get(&edge.id())
                        .and_then(|edge| graph.ontology.edge_type(edge.type_id))
                        .and_then(|edge| edge.properties.get(&id))
                        .cloned(),
                    Subject::Type(_) => None,
                }
                .ok_or_else(|| error("missing disputed property declaration"))?;
                (
                    definition.name.clone(),
                    serde_json::to_value(definition).map_err(error)?,
                )
            }
            Predicate::Relation(id) => {
                let definition = graph
                    .ontology
                    .edge_type(id)
                    .ok_or_else(|| error("missing disputed relation declaration"))?;
                (
                    definition.name.clone(),
                    serde_json::to_value((
                        &definition.source_types,
                        &definition.target_types,
                        &definition.cardinality,
                    ))
                    .map_err(error)?,
                )
            }
        };
        let about = match first.subject {
            Subject::Node(node) => graph
                .nodes
                .get(&node.id())
                .map_or_else(|| node.id().to_string(), |node| node.canonical_name.clone()),
            Subject::Edge(edge) => edge.id().to_string(),
            Subject::Type(id) => id.to_string(),
        };
        items.push(EkrKernelAttentionItem {
            subject: Box::new(subject),
            question: format!("Which value for {name} on {about} is correct, and during which dates? Select a supported claim, retract an incorrect claim, correct its time, or leave this unresolved."),
            basis: Box::new(EkrKernelReviewBasis {
                observed_revision: Box::new(EkrKernelRevisionNumber(revision.get().into())),
                evidence_digest: digest("ekr.attention.dispute-evidence/1", &(&support, &evidence_material))?,
                options_digest: digest("ekr.attention.dispute-options/1", &claims)?,
                effects_digest: digest("ekr.attention.dispute-effects/1", &(&component, &declaration))?,
            }),
            claims: component.iter().map(|id| Box::new(EkrGraphAssertionId(id.to_string()))).collect(),
            evidence: evidence_ids.iter().map(|id| Box::new(EkrGraphEvidenceId(id.to_string()))).collect(),
            observations: observation_ids.iter().map(|id| Box::new(EkrGraphObservationId(id.to_string()))).collect(),
        });
    }
    Ok(items)
}
