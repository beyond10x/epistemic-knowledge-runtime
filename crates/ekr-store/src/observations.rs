//! Independent append-only observation retention; never a canonical revision writer.
use super::*;
use ekr_core::contract_data::EkrObserveObservationImport;

const STREAM: &str = "ekr.observe.retained";
const RECORDED: &str = "ekr.observe.ObservationRetained";

/// Persistence port for independently retained observations and their exact bytes.
pub trait ObservationRetention {
    /// Returns every immutable import, verifying retained content and unique identity.
    /// # Errors
    /// Corrupt envelope, changed bytes, duplicate identity or provider failure.
    fn retained_observations(&self) -> Result<Vec<EkrObserveObservationImport>, StoreError>;

    /// Atomically appends an import and pins its bytes; false means an identical prior import.
    /// The kernel validates the supplied interpretation of the source before calling this port.
    /// # Errors
    /// Conflicting identity, invalid bytes, read-only store or native publication failure.
    fn retain_observation(
        &self,
        input: &EkrObserveObservationImport,
        at: Timestamp,
    ) -> Result<bool, StoreError>;
}

impl<S: EventStore> EventlogStore<S> {
    pub(super) fn read_observations(&self) -> Result<Vec<EkrObserveObservationImport>, StoreError> {
        self.read_observations_selected(None)
    }
    pub(super) fn read_observations_selected(
        &self,
        selected: Option<&BTreeSet<String>>,
    ) -> Result<Vec<EkrObserveObservationImport>, StoreError> {
        self.entered()?;
        let stream = StreamId::new(self.tenant.clone(), STREAM, "observations")?;
        let mut identities = BTreeSet::new();
        let mut keys = BTreeSet::new();
        let mut imports = Vec::new();
        for event in self.read_all(&stream, MAX_READ_LIMIT)? {
            if selected.is_some_and(|ids| {
                !event.data["observation"]["observation_id"]
                    .as_str()
                    .is_some_and(|id| ids.contains(id))
            }) {
                continue;
            }
            if event.name != RECORDED || event.schema_version != 1 {
                return Err(StoreError::Document("observation-envelope".into()));
            }
            let input: EkrObserveObservationImport =
                serde_json::from_value(event.data).map_err(json_error)?;
            if !identities.insert(input.observation.observation_id.0.clone())
                || !keys.insert(serde_json::to_vec(&input.key).map_err(json_error)?)
            {
                return Err(StoreError::Document("duplicate-observation".into()));
            }
            let (hash, bytes) = content(&input)?;
            let held = self
                .object(hash)?
                .ok_or_else(|| StoreError::Document("retained-observation-bytes-missing".into()))?;
            if *held.bytes != bytes
                || held.metadata.storage_class.retention_rank()
                    < StorageClass::Provenance.retention_rank()
            {
                return Err(StoreError::Document(
                    "retained-observation-bytes-changed".into(),
                ));
            }
            imports.push(input);
        }
        Ok(imports)
    }
}
impl<S: AtomicBlobEventStore> ObservationRetention for EventlogStore<S> {
    fn retained_observations(&self) -> Result<Vec<EkrObserveObservationImport>, StoreError> {
        self.read_observations()
    }
    fn retain_observation(
        &self,
        input: &EkrObserveObservationImport,
        at: Timestamp,
    ) -> Result<bool, StoreError> {
        self.entered()?;
        let (hash, bytes) = content(input)?;
        let object = crate::PublicationObject {
            bytes,
            storage_class: StorageClass::Provenance,
            stored_at: at,
        };
        for _ in 0..16 {
            let held = self.retained_observations()?;
            for previous in &held {
                if previous.observation.observation_id == input.observation.observation_id
                    || previous.key == input.key
                {
                    return if previous == input {
                        Ok(false)
                    } else {
                        Err(StoreError::PublicationInputConflict)
                    };
                }
            }
            let mut appends = vec![StreamAppend {
                stream: StreamId::new(self.tenant.clone(), STREAM, "observations")?,
                expected: if held.is_empty() {
                    Expected::NoStream
                } else {
                    Expected::Exact(held.len() as u64)
                },
                events: vec![NewEvent::new(
                    RECORDED,
                    1,
                    serde_json::to_value(input).map_err(json_error)?,
                )?],
            }];
            if let Some(append) = self.object_append(hash, &object)? {
                appends.push(append);
            }
            let input_hash = ContentHash::of_bytes(&serde_json::to_vec(input).map_err(json_error)?);
            let mut meta = envelope("ekr.observe.request", input_hash.to_hex());
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
                    bytes: object.bytes.clone(),
                }],
            };
            // The slot names the actual attempted effects, including time and every CAS
            // expectation. Same-process uncertain outcomes retry this exact native request.
            // After restart, the immutable observation stream resolves a successful attempt;
            // a concurrent attempt cannot duplicate it because both condition that stream.
            let key = format!("ekr.observe.{}", request.fingerprint()?);
            request.group.meta.idempotency_key = key.clone();
            request.group.meta.request_id = key.clone();
            request.group.meta.trace_id = key;
            match self.atomic(&request) {
                Ok(result) => return Ok(!result.deduplicated),
                Err(StoreError::Conflict) => continue,
                Err(error) => return Err(error),
            }
        }
        Err(StoreError::Conflict)
    }
}

fn content(input: &EkrObserveObservationImport) -> Result<(ContentHash, Vec<u8>), StoreError> {
    let hash: ContentHash = input
        .key
        .content_hash
        .0
        .parse()
        .map_err(|_| StoreError::Document("observation-content-hash".into()))?;
    let bytes =
        ekr_core::bytes::decode(&input.payload).map_err(|e| StoreError::Document(e.to_string()))?;
    if input.observation.content_hash != input.key.content_hash
        || ContentHash::of_bytes(&bytes) != hash
    {
        return Err(StoreError::Document("observation-content-mismatch".into()));
    }
    Ok((hash, bytes))
}
