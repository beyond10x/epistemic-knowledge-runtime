//! Validate supplied observation metadata before independent, atomic retention.
use crate::Commit;
use ekr_core::contract_data::{
    EkrGraphObservationRecord, EkrObserveObservationImport, EkrObserveObservationImportReceipt,
    EkrObserveObservationRetentionOutcome, EkrObserveRetainedObservationRead, EssPresence,
};
use ekr_core::{ContentHash, ObservationId, Timestamp};
use ekr_store::{ObjectStore, ObservationRetention, RevisionLog, StoreError};

fn validate(input: &EkrObserveObservationImport) -> Result<ObservationId, StoreError> {
    let hash: ContentHash = input
        .key
        .content_hash
        .0
        .parse()
        .map_err(|_| StoreError::Document("observation-content-hash".into()))?;
    let id: ObservationId = input
        .observation
        .observation_id
        .0
        .parse()
        .map_err(|_| StoreError::Document("observation-identity".into()))?;
    let _ = time::OffsetDateTime::parse(
        &input.observation.captured_at,
        &time::format_description::well_known::Rfc3339,
    )
    .map_err(|_| StoreError::Document("observation-captured-at".into()))?;
    let source_native_id = match &input.key.source_native_id {
        EssPresence::Absent => None,
        EssPresence::Present(value) => Some(value.clone()),
    };
    let key = ekr_core::ObservationIdempotencyKey {
        source: input.key.source.clone(),
        source_native_id,
        content_hash: hash,
    };
    if key.source.trim().is_empty()
        || key.observation_id() != id
        || input.observation.source != input.key.source
        || input.observation.source_native_id != input.key.source_native_id
        || input.observation.content_hash != input.key.content_hash
    {
        return Err(StoreError::Document(
            "observation-source-key-mismatch".into(),
        ));
    }
    let payload =
        ekr_core::bytes::decode(&input.payload).map_err(|e| StoreError::Document(e.to_string()))?;
    if ContentHash::of_bytes(&payload) != hash {
        return Err(StoreError::Document("observation-content-mismatch".into()));
    }
    Ok(id)
}

impl<S: RevisionLog + ObjectStore + ObservationRetention> Commit<S> {
    /// Retains supplied knowledge independently of canonical revisions and interpretations.
    /// # Errors
    /// Invalid source key, digest, identity or timestamp; changed prior import; provider failure.
    pub fn import_observation(
        &self,
        input: &EkrObserveObservationImport,
        at: Timestamp,
    ) -> Result<EkrObserveObservationImportReceipt, StoreError> {
        super::observation_behavior::import(self, input, at)
    }

    pub(super) fn retain_observation(
        &self,
        input: &EkrObserveObservationImport,
        at: Timestamp,
    ) -> Result<EkrObserveObservationImportReceipt, StoreError> {
        validate(input)?;
        let inserted = self.store.retain_observation(input, at)?;
        Ok(EkrObserveObservationImportReceipt {
            already_retained: !inserted,
            observation_id: input.observation.observation_id.clone(),
            content_hash: input.key.content_hash.clone(),
            outcome: Box::new(if inserted {
                EkrObserveObservationRetentionOutcome::V1
            } else {
                EkrObserveObservationRetentionOutcome::V0
            }),
        })
    }

    /// Lists retained observations in immutable identity order, without requiring a seed.
    /// # Errors
    /// Invalid retained metadata, missing or changed payload, or provider failure.
    pub fn observations(&self) -> Result<Vec<EkrGraphObservationRecord>, StoreError> {
        super::observation_behavior::list(self)
    }

    pub(super) fn retained_observation_records(
        &self,
    ) -> Result<Vec<EkrGraphObservationRecord>, StoreError> {
        let inputs = self.store.retained_observations()?;
        let mut records = Vec::with_capacity(inputs.len());
        for input in inputs {
            validate(&input)?;
            records.push(*input.observation);
        }
        records.sort_by(|a, b| a.observation_id.0.cmp(&b.observation_id.0));
        Ok(records)
    }

    /// Reads exact retained source bytes even when no interpretation was accepted.
    /// # Errors
    /// Unknown observation, invalid retained metadata/content, or provider failure.
    pub fn observation(
        &self,
        id: ObservationId,
    ) -> Result<EkrObserveRetainedObservationRead, StoreError> {
        super::observation_behavior::show(self, id)
    }

    pub(super) fn retained_observation(
        &self,
        id: ObservationId,
    ) -> Result<EkrObserveRetainedObservationRead, StoreError> {
        for input in self.store.retained_observations()? {
            let retained_id = validate(&input)?;
            if retained_id == id {
                return Ok(EkrObserveRetainedObservationRead {
                    observation: input.observation,
                    key: input.key,
                    payload: input.payload,
                });
            }
        }
        Err(StoreError::Document("observation-not-found".into()))
    }
}
