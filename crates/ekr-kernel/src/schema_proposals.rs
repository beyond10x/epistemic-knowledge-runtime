//! Proposal admission and read-only material review bases over independently retained sources.
use crate::{Commit, CommitError, VerifiedRead};
use ekr_core::contract_data as w;
use ekr_core::{ContentHash, Timestamp};
use ekr_store::{
    HumanDecisionRetention, IncubationRetention, ObjectStore, ObservationRetention,
    ProposalReviewRetention, RevisionLog, SchemaProposalRetention, StoreError,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn error(detail: impl std::fmt::Display) -> StoreError {
    StoreError::Document(format!("schema-proposal-refused: {detail}"))
}
fn read_error(error_value: CommitError) -> StoreError {
    match error_value {
        CommitError::Store(value) => value,
        other => error(other),
    }
}
fn digest(
    domain: &str,
    value: &impl Serialize,
) -> Result<Box<w::EkrKernelContentHash>, StoreError> {
    Ok(Box::new(w::EkrKernelContentHash(
        ContentHash::of_bytes(&serde_json::to_vec(&(domain, value)).map_err(error)?).to_string(),
    )))
}
pub(super) fn coordinate(
    version: &w::EkrIntegrateInterpretationVersion,
) -> Result<String, StoreError> {
    serde_json::to_string(version).map_err(error)
}
pub(super) fn fact_index(item: &str) -> Result<usize, StoreError> {
    let index = item
        .strip_prefix("facts[")
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| error("source selector must name facts[index]"))?;
    let parsed: usize = index.parse().map_err(error)?;
    if parsed.to_string() != index {
        return Err(error("source selector must use a canonical index"));
    }
    Ok(parsed)
}

pub(super) struct Source {
    pub retained_document: w::EkrIntegrateInterpretationDocument,
    pub document: ekr_integrate::ExtractionDocument,
    pub selected: BTreeSet<String>,
}

pub(super) struct SourceSupport {
    pub sources: BTreeMap<String, Source>,
    pub enums: Vec<BTreeSet<String>>,
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
    /// Retains an exact typed schema proposal after validating its sources and additive mappings.
    /// No schema or fact becomes canonical until an exact human decision is applied separately.
    /// # Errors
    /// Invalid or conflicting input, unavailable sources, incompatible schema or provider failure.
    pub fn submit_schema_proposal(
        &self,
        input: &w::EkrIntegrateSchemaProposalImport,
        at: Timestamp,
    ) -> Result<w::EkrIntegrateSchemaProposalRead, StoreError> {
        super::schema_proposal_behavior::submit(self, input, at)
    }

    /// Inspects retained proposal bytes, current mapping blockers and exact review material.
    /// # Errors
    /// Unknown identity, corrupt retained input or provider failure.
    pub fn schema_proposal(
        &self,
        id: &w::EkrIntegrateSchemaProposalId,
    ) -> Result<w::EkrIntegrateSchemaProposalRead, StoreError> {
        super::schema_proposal_behavior::show(self, id)
    }

    pub(super) fn retain_schema_proposal(
        &self,
        input: &w::EkrIntegrateSchemaProposalImport,
        at: Timestamp,
    ) -> Result<w::EkrIntegrateSchemaProposalRead, StoreError> {
        let payload = ekr_core::bytes::decode(&input.payload).map_err(error)?;
        if payload.len() > 8 * 1024 * 1024 {
            return Err(error("proposal exceeds eight MiB"));
        }
        let parsed: w::EkrIntegrateSchemaProposalDocument =
            serde_json::from_slice(&payload).map_err(error)?;
        if parsed != *input.proposal {
            return Err(error("typed proposal differs from exact supplied bytes"));
        }
        for previous in self.store.retained_schema_proposals()? {
            if previous.proposal.proposal_id == input.proposal.proposal_id {
                if previous.payload != input.payload {
                    return Err(StoreError::PublicationInputConflict);
                }
                return self.project_schema_proposal(&previous, false);
            }
        }
        let record = w::EkrIntegrateRetainedSchemaProposal {
            proposal: input.proposal.clone(),
            payload: input.payload.clone(),
            proposal_digest: Box::new(w::EkrKernelContentHash(
                ContentHash::of_bytes(&payload).to_string(),
            )),
        };
        let projected = self.project_schema_proposal(&record, true)?;
        let (retained, _) = self.store.retain_schema_proposal(&record, at)?;
        if retained != record {
            return Err(error("retention result differs from validated input"));
        }
        Ok(projected)
    }

    pub(super) fn retained_schema_proposal(
        &self,
        id: &w::EkrIntegrateSchemaProposalId,
    ) -> Result<w::EkrIntegrateSchemaProposalRead, StoreError> {
        use ekr_core::generated_identity::Identity;
        w::EkrIntegrateSchemaProposalId::parse_identity(&id.0).map_err(error)?;
        let record = self
            .store
            .retained_schema_proposals()?
            .into_iter()
            .find(|r| r.proposal.proposal_id.as_ref() == id)
            .ok_or_else(|| error("unknown proposal"))?;
        self.project_schema_proposal(&record, false)
    }

    fn project_schema_proposal(
        &self,
        record: &w::EkrIntegrateRetainedSchemaProposal,
        strict: bool,
    ) -> Result<w::EkrIntegrateSchemaProposalRead, StoreError> {
        let read = self.read(None).map_err(read_error)?;
        let mut result = self.project_schema_proposal_at(record, strict, &read)?;
        if !strict {
            let reviews = self.verified_proposal_reviews(record)?;
            result.expected_previous_decision =
                reviews.last().map_or(w::EssPresence::Absent, |r| {
                    w::EssPresence::Present(r.review.human_proof_digest.clone())
                });
            result.reviews = reviews.into_iter().map(|r| r.review).collect();
            let (history, _) = self.replayed_state().map_err(error)?;
            result.application =
                crate::application_auth::read(&history, &record.proposal.proposal_id);
            if let w::EssPresence::Present(application) = &result.application {
                result.receipts = application.receipts.clone();
            }
        }
        Ok(result)
    }

    pub(super) fn project_schema_proposal_at(
        &self,
        record: &w::EkrIntegrateRetainedSchemaProposal,
        strict: bool,
        read: &VerifiedRead,
    ) -> Result<w::EkrIntegrateSchemaProposalRead, StoreError> {
        let proposal = &record.proposal;
        let support = source_support(
            proposal,
            read,
            |version| {
                self.retained_interpretation(version)
                    .map(|held| *held.document)
            },
            |id| self.observation(id.0.parse().map_err(error)?).map(|_| ()),
        )?;
        let history = self
            .schema_history(read.root.revision)
            .map_err(read_error)?;
        let base = history
            .schemas
            .values()
            .find(|schema| schema.version().id.to_string() == proposal.base_schema.0)
            .ok_or_else(|| error("unknown base schema"))?;
        let observed = proposal
            .observations
            .iter()
            .map(|id| self.observation(id.0.parse().map_err(error)?))
            .collect::<Result<Vec<_>, StoreError>>()?;
        project_captured(record, strict, read, base, support, &observed)
    }
}

pub(super) fn source_support(
    proposal: &w::EkrIntegrateSchemaProposalDocument,
    read: &VerifiedRead,
    mut interpretation: impl FnMut(
        &w::EkrIntegrateInterpretationVersion,
    ) -> Result<w::EkrIntegrateInterpretationDocument, StoreError>,
    mut observation_record: impl FnMut(&w::EkrGraphObservationId) -> Result<(), StoreError>,
) -> Result<SourceSupport, StoreError> {
    let mut sources = BTreeMap::new();
    let mut supported_enums = Vec::new();
    let mut evidence = BTreeMap::new();
    for source in &proposal.sources {
        let retained_document = interpretation(&source.version)?;
        let document = super::incubation_document::project(&retained_document)?;
        let selected: BTreeSet<_> = source.items.iter().cloned().collect();
        if selected.len() != source.items.len() {
            return Err(error("duplicate source item"));
        }
        for item in &selected {
            let fact = document
                .facts
                .get(fact_index(item)?)
                .ok_or_else(|| error("selected fact does not exist"))?;
            if let ekr_integrate::ExtractedFact::Property(fact) = fact {
                if let Some(value) =
                    local_property(&document.ontology, &fact.subject.node_type, &fact.property)
                {
                    super::schema_proposal_schema::enum_sets(value, &mut supported_enums);
                }
            }
        }
        for support in &retained_document.evidence {
            let encoded = serde_json::to_vec(support).map_err(error)?;
            if let Some(previous) = evidence.insert(support.evidence.id.0.clone(), encoded.clone())
            {
                if previous != encoded {
                    return Err(error("supporting evidence identity has competing records"));
                }
            }
        }
        if sources
            .insert(
                coordinate(&source.version)?,
                Source {
                    retained_document,
                    document,
                    selected,
                },
            )
            .is_some()
        {
            return Err(error("duplicate proposal source"));
        }
    }
    let mut seen = BTreeSet::new();
    for observation in &proposal.observations {
        if !seen.insert(&observation.0) {
            return Err(error("duplicate supporting observation"));
        }
        observation_record(observation)?;
    }
    let mut seen = BTreeSet::new();
    for id in &proposal.evidence {
        if !seen.insert(&id.0) {
            return Err(error("duplicate supporting evidence"));
        }
        if !evidence.contains_key(&id.0) {
            let evidence = read
                .graph
                .evidence
                .get(&id.0.parse().map_err(error)?)
                .ok_or_else(|| error("missing supporting evidence"))?;
            let bytes = read
                .content(&evidence.content_hash)
                .ok_or_else(|| error("missing supporting evidence bytes"))?;
            if ContentHash::of_bytes(bytes) != evidence.content_hash {
                return Err(error("changed supporting evidence bytes"));
            }
        }
    }
    if sources.is_empty() && proposal.observations.is_empty() && proposal.evidence.is_empty() {
        return Err(error("proposal has no retained support"));
    }
    Ok(SourceSupport {
        sources,
        enums: supported_enums,
    })
}

/// Project exact review material from captured inputs without reentering provider reads.
pub(super) fn project_captured(
    record: &w::EkrIntegrateRetainedSchemaProposal,
    strict: bool,
    read: &VerifiedRead,
    base: &ekr_ontology::Ontology,
    support: SourceSupport,
    observed: &[w::EkrObserveRetainedObservationRead],
) -> Result<w::EkrIntegrateSchemaProposalRead, StoreError> {
    use ekr_core::generated_identity::Identity;
    let proposal = &record.proposal;
    if strict {
        super::schema_proposal_corrections::validate(read, proposal)?;
    }
    w::EkrIntegrateSchemaProposalId::parse_identity(&proposal.proposal_id.0).map_err(error)?;
    let SourceSupport {
        sources,
        enums: supported,
    } = support;
    // Original input must always remain interpretable, including after its schema has applied.
    let original = super::schema_proposal_schema::candidate(base, proposal, &supported)?;
    let current =
        super::schema_proposal_schema::candidate(&read.graph.ontology, proposal, &supported);
    let (candidate, stale) = match current {
        Ok(candidate) => (candidate, None),
        Err(err @ StoreError::Document(_)) if !strict => (original, Some(err.to_string())),
        Err(err) => return Err(err),
    };
    let (mut preview, effects) =
        super::schema_proposal_mapping::preview(read, proposal, &candidate, &sources, strict)?;
    let declarations =
        super::schema_proposal_material::relevant(&read.graph.ontology, &candidate, proposal)?;
    if let Some(reason) = &stale {
        for item in &mut preview {
            item.blockers.push(reason.clone());
        }
    }
    let sources_material: Vec<_> = sources
        .values()
        .map(|source| &source.retained_document)
        .collect();
    let canonical_evidence: Vec<_> = proposal
        .evidence
        .iter()
        .filter_map(|id| {
            id.0.parse()
                .ok()
                .and_then(|id| read.graph.evidence.get(&id))
        })
        .collect();
    let corrections = super::schema_proposal_corrections::components(read, proposal)?;
    let correction_evidence: Vec<_> = corrections
        .iter()
        .map(|item| &item.basis.evidence_digest)
        .collect();
    let correction_options: Vec<_> = corrections
        .iter()
        .map(|item| &item.basis.options_digest)
        .collect();
    let correction_effects: Vec<_> = corrections
        .iter()
        .map(|item| &item.basis.effects_digest)
        .collect();
    let basis = w::EkrKernelReviewBasis {
        observed_revision: Box::new(w::EkrKernelRevisionNumber(read.root.revision.get().into())),
        evidence_digest: digest(
            "ekr.schema-proposal.evidence/1",
            &(
                &sources_material,
                &observed,
                &canonical_evidence,
                &correction_evidence,
            ),
        )?,
        options_digest: digest(
            "ekr.schema-proposal.options/1",
            &(
                &record.proposal_digest,
                &proposal.additions,
                &proposal.mappings,
                &proposal.corrections,
                &correction_options,
            ),
        )?,
        effects_digest: digest(
            "ekr.schema-proposal.effects/1",
            &(
                &preview,
                &effects,
                &declarations,
                &stale,
                &correction_effects,
            ),
        )?,
    };
    Ok(w::EkrIntegrateSchemaProposalRead {
        proposal: record.proposal.clone(),
        proposal_digest: record.proposal_digest.clone(),
        preview,
        reviews: Vec::new(),
        receipts: Vec::new(),
        basis: Box::new(basis),
        expected_previous_decision: w::EssPresence::Absent,
        application: w::EssPresence::Absent,
    })
}
fn local_property<'a>(
    schema: &'a ekr_integrate::extraction::OntologySpec,
    name: &str,
    property: &str,
) -> Option<&'a ekr_integrate::extraction::ValueSpec> {
    let mut pending = vec![name];
    let mut seen = BTreeSet::new();
    while let Some(name) = pending.pop() {
        if !seen.insert(name) {
            continue;
        }
        let Some(node) = schema.node_types.iter().find(|node| node.name == name) else {
            continue;
        };
        if let Some(property) = node.properties.iter().find(|p| p.name == property) {
            return Some(&property.value);
        }
        pending.extend(node.parents.iter().map(String::as_str).rev());
    }
    None
}
