//! Deterministic, read-only discovery over retained interpretation blockers.
use crate::{Commit, CommitError};
use ekr_core::contract_data as w;
use ekr_core::contracts::integrate as m;
use ekr_core::contracts::integrate::obligations::DiscoverSchemaGapsBehavior;
use ekr_core::contracts::{
    graph as g, obligation::UnmetObligation, ontology as o, primitives::Uuid,
};
use ekr_core::ContentHash;
use ekr_store::{IncubationRetention, ObjectStore, ObservationRetention, RevisionLog, StoreError};
use std::collections::{BTreeMap, BTreeSet};

fn error(detail: impl std::fmt::Display) -> StoreError {
    StoreError::Document(format!("schema-proposal-refused: {detail}"))
}

impl<S: RevisionLog + ObjectStore + ObservationRetention + IncubationRetention> Commit<S> {
    /// Derives a stable request from pending blockers without changing any retained state.
    /// # Errors
    /// An unseeded store, invalid history, missing source bytes or provider failure.
    pub fn discover_schema_gaps(&self) -> Result<w::EkrIntegrateSchemaLearningRequest, StoreError> {
        let mut behavior = Discovery {
            commit: self,
            fault: None,
        };
        let result = behavior.discover_schema_gaps(m::DiscoverSchemaGaps {});
        if let Some(fault) = behavior.fault {
            return Err(fault);
        }
        match result.map_err(error)? {
            m::DiscoverSchemaGapsOutcome::Answered {
                discover_schema_gaps_result,
            } => Ok(wire_request(discover_schema_gaps_result.request)),
            m::DiscoverSchemaGapsOutcome::Refused { error: refusal } => Err(error(refusal.reason)),
        }
    }

    fn schema_gap_request(&self) -> Result<m::SchemaLearningRequest, StoreError> {
        let read = self.read(None).map_err(|err| match err {
            CommitError::NotSeeded => error("not-seeded"),
            CommitError::Store(err) => err,
            err => error(err),
        })?;
        let mut groups = BTreeMap::<(String, String), m::GapGroup>::new();
        let mut evidence = BTreeSet::new();
        for version in self.retained_interpretation_versions()? {
            let held = self.retained_interpretation(&version)?;
            let source = super::incubation_projection::version(&version)?;
            let document = super::incubation_document::project(&held.document)?;
            let current = super::incubation::classify_gaps(&document, &read.graph.ontology);
            for blocker in &held.blockers {
                if blocker.document_digest != version.document_digest {
                    return Err(error("blocker differs from its source document"));
                }
                if read.integrated_source_item(&version, &blocker.item)
                    || held.receipts.iter().any(|receipt| {
                        receipt.item == blocker.item
                            && matches!(
                                *receipt.disposition,
                                w::EkrIntegrateProcessingDisposition::V0
                                    | w::EkrIntegrateProcessingDisposition::V1
                                    | w::EkrIntegrateProcessingDisposition::V3
                            )
                    })
                {
                    continue;
                }
                let Some((_, declaration, kind, _)) =
                    current.iter().find(|(item, ..)| item == &blocker.item)
                else {
                    continue;
                };
                let key = (
                    serde_json::to_string(kind).map_err(error)?,
                    declaration.clone(),
                );
                if !groups.contains_key(&key) {
                    // A projection handle, never a minted canonical entity identity. Include the
                    // original seed occurrence so independent stores cannot share a handle.
                    let material =
                        serde_json::to_vec(&("ekr.schema-gap-group/1", read.seed.event_id, &key))
                            .map_err(error)?;
                    let hash = ContentHash::of_bytes(&material);
                    let digest = hash.to_string();
                    let variant = (hash.as_bytes()[8] & 0x3f) | 0x80;
                    let id = format!(
                        "{}-{}-8{}-{:02x}{}-{}",
                        &digest[..8],
                        &digest[8..12],
                        &digest[13..16],
                        variant,
                        &digest[18..20],
                        &digest[20..32]
                    );
                    groups.insert(
                        key.clone(),
                        m::GapGroup {
                            group_id: m::GapGroupId(Uuid(id)),
                            kind: super::incubation_projection::blocker_kind(kind),
                            declaration: declaration.clone(),
                            blockers: Vec::new(),
                            observations: Vec::new(),
                            sources: Vec::new(),
                        },
                    );
                }
                let group = groups.get_mut(&key).expect("inserted group");
                // The immutable finding anchors this exact source item; its historical
                // classification is not overwritten by the current group's classification.
                group
                    .blockers
                    .push(m::IntegrationBlockerId(Uuid(blocker.blocker_id.0.clone())));
                if !group.sources.contains(&source) {
                    group.sources.push(source.clone());
                }
                for id in &held.document.observations {
                    group
                        .observations
                        .push(g::ObservationId(Uuid(id.0.clone())));
                }
                evidence.extend(
                    held.document
                        .evidence
                        .iter()
                        .map(|e| e.evidence.id.0.clone()),
                );
            }
        }
        for group in groups.values_mut() {
            group.blockers.sort_by(|a, b| a.0 .0.cmp(&b.0 .0));
            group.blockers.dedup();
            group.observations.sort_by(|a, b| a.0 .0.cmp(&b.0 .0));
            group.observations.dedup();
            // Versions arrive in the retention API's identity/version order.
        }
        Ok(m::SchemaLearningRequest {
            base_schema: o::SchemaVersionId(Uuid(read.graph.ontology.version().id.to_string())),
            groups: groups.into_values().collect(),
            evidence: evidence
                .into_iter()
                .map(|id| g::EvidenceId(Uuid(id)))
                .collect(),
        })
    }
}

struct Discovery<'a, S: RevisionLog + ObjectStore> {
    commit: &'a Commit<S>,
    fault: Option<StoreError>,
}
impl<S: RevisionLog + ObjectStore + ObservationRetention + IncubationRetention>
    DiscoverSchemaGapsBehavior for Discovery<'_, S>
{
    fn discover_schema_gaps(
        &mut self,
        _: m::DiscoverSchemaGaps,
    ) -> Result<m::DiscoverSchemaGapsOutcome, UnmetObligation> {
        match self.commit.schema_gap_request() {
            Ok(request) => Ok(m::DiscoverSchemaGapsOutcome::Answered {
                discover_schema_gaps_result: m::DiscoverSchemaGapsResult { request },
            }),
            Err(err @ StoreError::Document(_)) => Ok(m::DiscoverSchemaGapsOutcome::Refused {
                error: m::KnowledgeRefused {
                    code: "schema-proposal-refused".into(),
                    reason: err.to_string(),
                },
            }),
            Err(err) => {
                self.fault = Some(err);
                Err(UnmetObligation {
                    capability: "available knowledge persistence",
                    source: "ekr.integrate.DiscoverSchemaGaps",
                })
            }
        }
    }
}

fn wire_request(request: m::SchemaLearningRequest) -> w::EkrIntegrateSchemaLearningRequest {
    w::EkrIntegrateSchemaLearningRequest {
        base_schema: Box::new(w::EkrOntologySchemaVersionId(request.base_schema.0 .0)),
        evidence: request
            .evidence
            .into_iter()
            .map(|id| Box::new(w::EkrGraphEvidenceId(id.0 .0)))
            .collect(),
        groups: request
            .groups
            .into_iter()
            .map(|group| {
                Box::new(w::EkrIntegrateGapGroup {
                    group_id: Box::new(w::EkrIntegrateGapGroupId(group.group_id.0 .0)),
                    kind: Box::new(match group.kind {
                        m::IntegrationBlockerKind::Contradiction => {
                            w::EkrIntegrateIntegrationBlockerKind::V0
                        }
                        m::IntegrationBlockerKind::RejectedInterpretation => {
                            w::EkrIntegrateIntegrationBlockerKind::V1
                        }
                        m::IntegrationBlockerKind::UnknownProperty => {
                            w::EkrIntegrateIntegrationBlockerKind::V2
                        }
                        m::IntegrationBlockerKind::UnknownRelation => {
                            w::EkrIntegrateIntegrationBlockerKind::V3
                        }
                        m::IntegrationBlockerKind::UnknownType => {
                            w::EkrIntegrateIntegrationBlockerKind::V4
                        }
                        m::IntegrationBlockerKind::UnresolvedReference => {
                            w::EkrIntegrateIntegrationBlockerKind::V5
                        }
                        m::IntegrationBlockerKind::ValueMismatch => {
                            w::EkrIntegrateIntegrationBlockerKind::V6
                        }
                    }),
                    declaration: group.declaration,
                    blockers: group
                        .blockers
                        .into_iter()
                        .map(|id| Box::new(w::EkrIntegrateIntegrationBlockerId(id.0 .0)))
                        .collect(),
                    observations: group
                        .observations
                        .into_iter()
                        .map(|id| Box::new(w::EkrGraphObservationId(id.0 .0)))
                        .collect(),
                    sources: group
                        .sources
                        .into_iter()
                        .map(|source| Box::new(super::incubation_projection::wire_version(source)))
                        .collect(),
                })
            })
            .collect(),
    }
}
