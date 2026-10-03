//! Immutable interpretation documents and their independent processing records.
use super::*;
use ekr_core::contract_data::EkrIntegrateRetainedInterpretation;

const STREAM: &str = "ekr.integrate.incubation";
const RECORDED: &str = "ekr.integrate.InterpretationRetained";

/// Persistence port for parked knowledge, separate from canonical revision publication.
pub trait IncubationRetention {
    /// Reads immutable records and verifies the exact retained document and evidence bytes.
    /// # Errors
    /// Corrupt envelopes, duplicate coordinates, changed content or provider failure.
    fn retained_interpretations(
        &self,
    ) -> Result<Vec<EkrIntegrateRetainedInterpretation>, StoreError>;
    /// Publishes one immutable version and its pinned bytes atomically.
    /// Returns the winning record and whether this call inserted it.
    /// # Errors
    /// Reused coordinate/root with changed input, invalid content or provider failure.
    fn retain_interpretation(
        &self,
        record: &EkrIntegrateRetainedInterpretation,
        at: Timestamp,
    ) -> Result<(EkrIntegrateRetainedInterpretation, bool), StoreError>;
}

fn payloads(
    record: &EkrIntegrateRetainedInterpretation,
) -> Result<BTreeMap<ContentHash, Vec<u8>>, StoreError> {
    let mut result = BTreeMap::new();
    let bytes = ekr_core::bytes::decode(&record.payload)
        .map_err(|e| StoreError::Document(e.to_string()))?;
    let hash = ContentHash::of_bytes(&bytes);
    let document: ekr_core::contract_data::EkrIntegrateInterpretationDocument =
        serde_json::from_slice(&bytes).map_err(json_error)?;
    if hash.to_hex() != record.version.document_digest.0
        || document != *record.interpretation.document
        || document.version.interpretation_id != record.version.interpretation_id
        || document.version.version != record.version.version
        || document.root_id != record.root.id
    {
        return Err(StoreError::Document("incubation-document-mismatch".into()));
    }
    result.insert(hash, bytes);
    for evidence in &document.evidence {
        let bytes = ekr_core::bytes::decode(&evidence.payload)
            .map_err(|e| StoreError::Document(e.to_string()))?;
        let hash = ContentHash::of_bytes(&bytes);
        if hash.to_hex() != evidence.evidence.content_hash.0 {
            return Err(StoreError::Document("incubation-evidence-mismatch".into()));
        }
        result.insert(hash, bytes);
    }
    Ok(result)
}

impl<S: AtomicBlobEventStore> IncubationRetention for EventlogStore<S> {
    fn retained_interpretations(
        &self,
    ) -> Result<Vec<EkrIntegrateRetainedInterpretation>, StoreError> {
        self.entered()?;
        let stream = StreamId::new(self.tenant.clone(), STREAM, "interpretations")?;
        let mut coordinates = BTreeSet::new();
        let mut records = Vec::new();
        for event in self.read_all(&stream, MAX_READ_LIMIT)? {
            if event.name != RECORDED || event.schema_version != 1 {
                return Err(StoreError::Document("incubation-envelope".into()));
            }
            let record: EkrIntegrateRetainedInterpretation =
                serde_json::from_value(event.data).map_err(json_error)?;
            if !coordinates.insert((
                record.version.interpretation_id.0.clone(),
                record.version.version.to_string(),
            )) {
                return Err(StoreError::Document(
                    "incubation-duplicate-coordinate".into(),
                ));
            }
            for (hash, bytes) in payloads(&record)? {
                let held = self
                    .object(hash)?
                    .ok_or_else(|| StoreError::Document("incubation-missing-bytes".into()))?;
                if *held.bytes != bytes
                    || held.metadata.storage_class.retention_rank()
                        < StorageClass::Provenance.retention_rank()
                {
                    return Err(StoreError::Document("incubation-changed-bytes".into()));
                }
            }
            records.push(record);
        }
        Ok(records)
    }

    fn retain_interpretation(
        &self,
        record: &EkrIntegrateRetainedInterpretation,
        at: Timestamp,
    ) -> Result<(EkrIntegrateRetainedInterpretation, bool), StoreError> {
        self.entered()?;
        let payloads = payloads(record)?;
        for _ in 0..16 {
            let held = self.retained_interpretations()?;
            for previous in &held {
                if previous.version.interpretation_id == record.version.interpretation_id
                    && previous.version.version == record.version.version
                {
                    return if previous.version == record.version
                        && previous.payload == record.payload
                    {
                        Ok((previous.clone(), false))
                    } else {
                        Err(StoreError::PublicationInputConflict)
                    };
                }
                if previous.root.id == record.root.id
                    && previous.version.interpretation_id != record.version.interpretation_id
                {
                    return Err(StoreError::PublicationInputConflict);
                }
            }
            let mut appends = vec![StreamAppend {
                stream: StreamId::new(self.tenant.clone(), STREAM, "interpretations")?,
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
            let mut blobs = Vec::new();
            for (hash, bytes) in &payloads {
                let object = crate::PublicationObject {
                    bytes: bytes.clone(),
                    storage_class: StorageClass::Provenance,
                    stored_at: at,
                };
                if let Some(append) = self.object_append(*hash, &object)? {
                    appends.push(append);
                }
                blobs.push(BlobWrite {
                    digest: hash.to_hex(),
                    bytes: bytes.clone(),
                });
            }
            let mut meta = envelope(
                "ekr.incubate.request",
                record.version.document_digest.0.clone(),
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
                blobs,
            };
            let key = format!("ekr.incubate.{}", request.fingerprint()?);
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
