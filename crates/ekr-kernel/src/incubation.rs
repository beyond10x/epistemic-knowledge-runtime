//! Validate and retain immutable interpretations without changing canonical knowledge.
use super::incubation_document::{error, project};
use crate::Commit;
use ekr_core::contract_data::*;
use ekr_core::generated_identity::{Identity, InterpretationId};
use ekr_core::{ContentHash, GraphRootId, SchemaVersionId, Timestamp};
use ekr_store::{IncubationRetention, ObjectStore, ObservationRetention, RevisionLog, StoreError};
use std::collections::BTreeSet;

fn coordinate(
    input: &EkrIntegrateInterpretationDocument,
) -> Result<(InterpretationId, u64), StoreError> {
    let id = InterpretationId::parse_identity(&input.version.interpretation_id.0).map_err(error)?;
    let version = input
        .version
        .version
        .as_u64()
        .filter(|v| *v > 0)
        .ok_or_else(|| error("version must be a positive integer"))?;
    let _: GraphRootId = input.root_id.0.parse().map_err(error)?;
    Ok((id, version))
}

fn empty_ontology(at: Timestamp) -> Result<ekr_ontology::Ontology, StoreError> {
    ekr_ontology::Ontology::load(ekr_ontology::OntologyDocument {
        version: ekr_ontology::SchemaVersion::seed(SchemaVersionId::mint(), at),
        node_types: vec![],
        edge_types: vec![],
    })
    .map_err(error)
}

impl<S: RevisionLog + ObjectStore + ObservationRetention + IncubationRetention> Commit<S> {
    fn checked_document(
        &self,
        document: &EkrIntegrateInterpretationDocument,
        at: Timestamp,
    ) -> Result<ekr_integrate::ExtractionDocument, StoreError> {
        coordinate(document)?;
        let mut sources = BTreeSet::new();
        for id in &document.observations {
            let id: ekr_core::ObservationId = id.0.parse().map_err(error)?;
            if !sources.insert(id) {
                return Err(error("duplicate observation reference"));
            }
            self.observation(id)?;
        }
        let projection = project(document)?;
        projection
            .check_incubation(&empty_ontology(at)?)
            .map_err(error)?;
        for evidence in &projection.evidence {
            if let ekr_graph::EvidenceSource::Observation(id) = evidence.evidence.source {
                if !sources.contains(&id) {
                    return Err(error(
                        "evidence observation is not declared by the document",
                    ));
                }
                let retained = self.observation(id)?;
                if retained.observation.content_hash.0 != evidence.evidence.content_hash.to_hex()
                    || ekr_core::bytes::decode(&retained.payload).map_err(error)?
                        != evidence.payload
                {
                    return Err(error("evidence differs from its retained observation"));
                }
            }
        }
        // The older extraction checker checks references but canonical schema publication used
        // to catch parent cycles. Incubation has no publication, so check them here as well.
        for node in &projection.ontology.node_types {
            let mut pending = node.parents.clone();
            let mut seen = BTreeSet::new();
            while let Some(name) = pending.pop() {
                if name == node.name {
                    return Err(error("local node type inheritance cycle"));
                }
                if seen.insert(name.clone()) {
                    if let Some(parent) = projection
                        .ontology
                        .node_types
                        .iter()
                        .find(|n| n.name == name)
                    {
                        pending.extend(parent.parents.clone());
                    }
                }
            }
        }
        Ok(projection)
    }

    /// Retains exact document bytes, locally checked declarations and durable integration gaps.
    /// # Errors
    /// Malformed local shape, missing source, inadmissible evidence, immutable version conflict
    /// or provider failure. A canonical vocabulary gap is retained as a blocker.
    pub fn import_interpretation(
        &self,
        input: &EkrIntegrateInterpretationImport,
        at: Timestamp,
    ) -> Result<EkrIntegrateIncubationImportReceipt, StoreError> {
        super::incubation_behavior::import(self, input, at)
    }

    pub(super) fn retain_interpretation(
        &self,
        input: &EkrIntegrateInterpretationImport,
        at: Timestamp,
    ) -> Result<EkrIntegrateIncubationImportReceipt, StoreError> {
        let bytes = ekr_core::bytes::decode(&input.payload).map_err(error)?;
        if bytes.len() > 8 * 1024 * 1024 {
            return Err(error("document exceeds eight MiB"));
        }
        let parsed: EkrIntegrateInterpretationDocument =
            serde_json::from_slice(&bytes).map_err(error)?;
        if parsed != *input.document {
            return Err(error("typed document differs from exact supplied bytes"));
        }
        coordinate(&parsed)?;
        let version = EkrIntegrateInterpretationVersion {
            interpretation_id: parsed.version.interpretation_id.clone(),
            version: parsed.version.version.clone(),
            document_digest: Box::new(EkrKernelContentHash(ContentHash::of_bytes(&bytes).to_hex())),
        };
        // Resolve retries before recomputing gaps against a newer canonical schema.
        for held in self.store.retained_interpretations()? {
            if held.version.interpretation_id == version.interpretation_id
                && held.version.version == version.version
            {
                if *held.version != version || held.payload != input.payload {
                    return Err(StoreError::PublicationInputConflict);
                }
                return Ok(receipt(&held, false));
            }
        }
        let projection = self.checked_document(&parsed, at)?;
        let current = if self.head()?.is_some() {
            Some(self.snapshot()?)
        } else {
            None
        };
        if current
            .as_ref()
            .is_some_and(|g| g.root.id.to_string() == parsed.root_id.0)
        {
            return Err(error("root is canonical"));
        }
        let ontology = match &current {
            Some(g) => g.ontology.clone(),
            None => empty_ontology(at)?,
        };
        let basis = current
            .as_ref()
            .map_or(ContentHash::of_bytes(b"ekr.incubation.unseeded/1"), |g| {
                ContentHash::of(&g.ontology)
            });
        let (blockers, receipts) = gaps(&projection, &ontology, &version, basis);
        let created_at =
            time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(at.millis()) * 1_000_000)
                .map(EssTimestamp)
                .map_err(error)?;
        let record = EkrIntegrateRetainedInterpretation {
            version: Box::new(version),
            payload: input.payload.clone(),
            root: Box::new(EkrGraphGraphRootRecord {
                id: parsed.root_id.clone(),
                space: Box::new(EkrGraphSpace::V1),
                schema_version_id: Box::new(EkrOntologySchemaVersionId(
                    SchemaVersionId::mint().to_string(),
                )),
                parent: EssPresence::Absent,
                created_at,
            }),
            interpretation: Box::new(EkrIntegrateInterpretationRead {
                document: input.document.clone(),
                blockers,
                receipts,
            }),
        };
        let (retained, inserted) = self.store.retain_interpretation(&record, at)?;
        Ok(receipt(&retained, inserted))
    }

    /// Lists immutable interpretation versions in identity and numeric version order.
    /// # Errors
    /// Corrupt retained records or provider failure.
    pub fn interpretations(&self) -> Result<Vec<EkrIntegrateInterpretationVersion>, StoreError> {
        super::incubation_behavior::list(self)
    }

    pub(super) fn retained_interpretation_versions(
        &self,
    ) -> Result<Vec<EkrIntegrateInterpretationVersion>, StoreError> {
        let mut result = Vec::new();
        for record in self.store.retained_interpretations()? {
            coordinate(&record.interpretation.document)?;
            result.push(*record.version);
        }
        result.sort_by(|a, b| {
            a.interpretation_id
                .0
                .cmp(&b.interpretation_id.0)
                .then_with(|| a.version.as_u64().cmp(&b.version.as_u64()))
        });
        Ok(result)
    }

    /// Reads parked facts, source references and their processing history after reopening.
    /// # Errors
    /// Unknown coordinate, changed digest, invalid retained source or provider failure.
    pub fn interpretation(
        &self,
        version: &EkrIntegrateInterpretationVersion,
    ) -> Result<EkrIntegrateInterpretationRead, StoreError> {
        super::incubation_behavior::show(self, version)
    }

    pub(super) fn retained_interpretation(
        &self,
        version: &EkrIntegrateInterpretationVersion,
    ) -> Result<EkrIntegrateInterpretationRead, StoreError> {
        for record in self.store.retained_interpretations()? {
            if *record.version == *version {
                self.checked_document(&record.interpretation.document, Timestamp::EPOCH)?;
                return Ok(*record.interpretation);
            }
        }
        Err(error("unknown interpretation version or changed digest"))
    }

    /// Materializes the existing transient graph representation for local inspection. Candidate
    /// identities are ephemeral per read; persistent references use document coordinate/digest
    /// and item path. This graph has no canonical address and grants no commit authority.
    /// # Errors
    /// Unknown version, changed source, invalid retained root or provider failure.
    pub fn incubation_graph(
        &self,
        version: &EkrIntegrateInterpretationVersion,
    ) -> Result<ekr_graph::TransientGraph, StoreError> {
        for record in self.store.retained_interpretations()? {
            if *record.version == *version {
                if *record.root.space != EkrGraphSpace::V1 {
                    return Err(error("retained root is not transient"));
                }
                let created_at =
                    super::incubation_document::timestamp_value(&record.root.created_at)?;
                let document =
                    self.checked_document(&record.interpretation.document, created_at)?;
                let root = ekr_graph::GraphRoot {
                    id: record.root.id.0.parse().map_err(error)?,
                    space: ekr_graph::Space::Transient,
                    schema_version_id: record.root.schema_version_id.0.parse().map_err(error)?,
                    parent: None,
                    created_at,
                };
                return Ok(super::incubation_graph::materialize(
                    root,
                    &document,
                    self.authority.context.operator,
                ));
            }
        }
        Err(error("unknown interpretation version or changed digest"))
    }
}

fn receipt(
    record: &EkrIntegrateRetainedInterpretation,
    inserted: bool,
) -> EkrIntegrateIncubationImportReceipt {
    EkrIntegrateIncubationImportReceipt {
        version: record.version.clone(),
        already_retained: !inserted,
        outcome: Box::new(if !inserted {
            EkrIntegrateIncubationOutcome::V0
        } else if record.interpretation.blockers.is_empty() {
            EkrIntegrateIncubationOutcome::V1
        } else {
            EkrIntegrateIncubationOutcome::V2
        }),
        blockers: record
            .interpretation
            .blockers
            .iter()
            .map(|b| b.blocker_id.clone())
            .collect(),
    }
}

// The generated contract owns these boxed collection element types.
#[allow(clippy::vec_box)]
fn gaps(
    document: &ekr_integrate::ExtractionDocument,
    ontology: &ekr_ontology::Ontology,
    version: &EkrIntegrateInterpretationVersion,
    basis: ContentHash,
) -> (
    Vec<Box<EkrIntegrateIntegrationBlockerSnapshot>>,
    Vec<Box<EkrIntegrateProcessingReceiptSnapshot>>,
) {
    use ekr_integrate::ExtractedFact;
    use EkrIntegrateIntegrationBlockerKind as K;
    let schema = ontology.to_document();
    let mut blockers = Vec::new();
    let mut receipts = Vec::new();
    let mut add = |item: String, declaration: String, kind: K, reason: &str| {
        blockers.push(Box::new(EkrIntegrateIntegrationBlockerSnapshot {
            blocker_id: Box::new(EkrIntegrateIntegrationBlockerId::mint()),
            document_digest: version.document_digest.clone(),
            item,
            declaration,
            kind: Box::new(kind),
            reason: reason.into(),
            basis_digest: Box::new(EkrKernelContentHash(basis.to_hex())),
        }));
    };
    for (index, entity) in document.entities.iter().enumerate() {
        if !schema.node_types.iter().any(|t| t.name == entity.node_type) {
            add(
                format!("entities[{index}]"),
                entity.node_type.clone(),
                K::V4,
                "Local type has no canonical declaration",
            );
        }
    }
    for (index, fact) in document.facts.iter().enumerate() {
        let item = format!("facts[{index}]");
        let subject = match fact {
            ExtractedFact::Property(p) => &p.subject,
            ExtractedFact::Relation(r) => &r.subject,
        };
        match schema
            .node_types
            .iter()
            .find(|t| t.name == subject.node_type)
        {
            None => add(
                item.clone(),
                subject.node_type.clone(),
                K::V4,
                "Local subject type has no canonical declaration",
            ),
            Some(t) => match fact {
                ExtractedFact::Property(p) => {
                    let properties = ontology.properties_of(t.id);
                    if !properties.values().any(|v| v.name == p.property) {
                        add(
                            item.clone(),
                            format!("{}.{}", subject.node_type, p.property),
                            K::V2,
                            "Local property has no canonical declaration",
                        );
                    } else {
                        add(
                            item.clone(),
                            subject.node_type.clone(),
                            K::V5,
                            "Fact is parked pending an explicit reviewed identity mapping",
                        );
                    }
                }
                ExtractedFact::Relation(r) => {
                    if !schema.edge_types.iter().any(|t| t.name == r.relation) {
                        add(
                            item.clone(),
                            r.relation.clone(),
                            K::V3,
                            "Local relation has no canonical declaration",
                        );
                    } else {
                        add(
                            item.clone(),
                            subject.node_type.clone(),
                            K::V5,
                            "Fact is parked pending an explicit reviewed identity mapping",
                        );
                    }
                }
            },
        }
        receipts.push(Box::new(EkrIntegrateProcessingReceiptSnapshot {
            receipt_id: Box::new(EkrIntegrateProcessingReceiptId::mint()),
            mapping_digest: EssPresence::Absent,
            document_digest: version.document_digest.clone(),
            item,
            disposition: Box::new(EkrIntegrateProcessingDisposition::V2),
            transaction_id: EssPresence::Absent,
            assertions: vec![],
            basis_digest: Box::new(EkrKernelContentHash(basis.to_hex())),
        }));
    }
    (blockers, receipts)
}
