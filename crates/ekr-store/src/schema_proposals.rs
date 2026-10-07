//! Immutable proposal bytes, retained independently of canonical state and human review.
use super::*;
use ekr_core::contract_data::EkrIntegrateRetainedSchemaProposal;
use ekr_core::generated_identity::{Identity, SchemaProposalId};

const STREAM: &str = "ekr.integrate.schema-proposals";
const RECORDED: &str = "ekr.integrate.SchemaProposalRetained";

/// Physical retention only: this port does not admit or approve a schema proposal.
pub trait SchemaProposalRetention {
    /// Reads records and checks their envelopes, identities and pinned bytes.
    /// # Errors
    /// Corrupt envelopes, duplicate identities, missing or changed bytes, or provider failure.
    fn retained_schema_proposals(
        &self,
    ) -> Result<Vec<EkrIntegrateRetainedSchemaProposal>, StoreError>;
    /// Atomically retains an immutable proposal and its bytes at Provenance rank or stronger.
    /// Returns the winning record and whether this call inserted it.
    /// # Errors
    /// Invalid input, changed bytes under an existing identity, or provider failure.
    fn retain_schema_proposal(
        &self,
        record: &EkrIntegrateRetainedSchemaProposal,
        at: Timestamp,
    ) -> Result<(EkrIntegrateRetainedSchemaProposal, bool), StoreError>;
}

fn payload(
    record: &EkrIntegrateRetainedSchemaProposal,
) -> Result<(ContentHash, Vec<u8>), StoreError> {
    SchemaProposalId::parse_identity(&record.proposal.proposal_id.0)
        .map_err(|e| StoreError::Document(e.to_string()))?;
    let bytes = ekr_core::bytes::decode(&record.payload)
        .map_err(|e| StoreError::Document(e.to_string()))?;
    let hash = ContentHash::of_bytes(&bytes);
    let document: ekr_core::contract_data::EkrIntegrateSchemaProposalDocument =
        serde_json::from_slice(&bytes).map_err(json_error)?;
    if hash.to_hex() != record.proposal_digest.0 || document != *record.proposal {
        return Err(StoreError::Document(
            "schema-proposal-document-mismatch".into(),
        ));
    }
    Ok((hash, bytes))
}

impl<S: EventStore> EventlogStore<S> {
    pub(super) fn read_schema_proposals(
        &self,
    ) -> Result<Vec<EkrIntegrateRetainedSchemaProposal>, StoreError> {
        self.read_schema_proposals_selected(None)
    }
    pub(super) fn read_schema_proposals_selected(
        &self,
        selected: Option<&str>,
    ) -> Result<Vec<EkrIntegrateRetainedSchemaProposal>, StoreError> {
        self.entered()?;
        let stream = StreamId::new(self.tenant.clone(), STREAM, "proposals")?;
        let mut identities = BTreeSet::new();
        let mut records = Vec::new();
        for event in self.read_all(&stream, MAX_READ_LIMIT)? {
            if selected.is_some_and(|id| event.data["proposal"]["proposal_id"].as_str() != Some(id))
            {
                continue;
            }
            if event.name != RECORDED || event.schema_version != 1 {
                return Err(StoreError::Document("schema-proposal-envelope".into()));
            }
            let record: EkrIntegrateRetainedSchemaProposal =
                serde_json::from_value(event.data).map_err(json_error)?;
            if !identities.insert(record.proposal.proposal_id.0.clone()) {
                return Err(StoreError::Document(
                    "schema-proposal-duplicate-identity".into(),
                ));
            }
            let (hash, bytes) = payload(&record)?;
            let held = self
                .object(hash)?
                .ok_or_else(|| StoreError::Document("schema-proposal-missing-bytes".into()))?;
            if *held.bytes != bytes
                || held.metadata.storage_class.retention_rank()
                    < StorageClass::Provenance.retention_rank()
            {
                return Err(StoreError::Document("schema-proposal-changed-bytes".into()));
            }
            records.push(record);
        }
        Ok(records)
    }
}
impl<S: AtomicBlobEventStore> SchemaProposalRetention for EventlogStore<S> {
    fn retained_schema_proposals(
        &self,
    ) -> Result<Vec<EkrIntegrateRetainedSchemaProposal>, StoreError> {
        self.read_schema_proposals()
    }
    fn retain_schema_proposal(
        &self,
        record: &EkrIntegrateRetainedSchemaProposal,
        at: Timestamp,
    ) -> Result<(EkrIntegrateRetainedSchemaProposal, bool), StoreError> {
        self.entered()?;
        let (hash, bytes) = payload(record)?;
        for _ in 0..16 {
            let held = self.retained_schema_proposals()?;
            if let Some(previous) = held
                .iter()
                .find(|previous| previous.proposal.proposal_id == record.proposal.proposal_id)
            {
                return if previous == record {
                    Ok((previous.clone(), false))
                } else {
                    Err(StoreError::PublicationInputConflict)
                };
            }
            let mut appends = vec![StreamAppend {
                stream: StreamId::new(self.tenant.clone(), STREAM, "proposals")?,
                expected: if held.is_empty() {
                    Expected::NoStream
                } else {
                    Expected::Exact(held.len() as u64)
                },
                events: vec![NewEvent::new(
                    RECORDED,
                    1,
                    serde_json::to_value(record).map_err(json_error)?,
                )?],
            }];
            let object = crate::PublicationObject {
                bytes: bytes.clone(),
                storage_class: StorageClass::Provenance,
                stored_at: at,
            };
            if let Some(append) = self.object_append(hash, &object)? {
                appends.push(append);
            }
            let mut meta = envelope(
                "ekr.schema-proposal.request",
                record.proposal_digest.0.clone(),
            );
            meta.occurred_at =
                OffsetDateTime::from_unix_timestamp_nanos(i128::from(at.millis()) * 1_000_000)
                    .map_err(|e| StoreError::Document(e.to_string()))?;
            let mut request = BlobAppendGroup {
                group: AppendGroup {
                    tenant: self.tenant.clone(),
                    appends,
                    meta,
                },
                blobs: vec![BlobWrite {
                    digest: hash.to_hex(),
                    bytes: bytes.clone(),
                }],
            };
            let key = format!("ekr.schema-proposal.{}", request.fingerprint()?);
            request.group.meta.idempotency_key = key.clone();
            request.group.meta.request_id = key.clone();
            request.group.meta.trace_id = key;
            match self.atomic(&request) {
                Ok(result) => return Ok((record.clone(), !result.deduplicated)),
                Err(StoreError::Conflict) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(StoreError::Conflict)
    }
}
