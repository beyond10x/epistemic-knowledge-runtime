//! A stage's record and tenant (design §§ 107.1, 107.3, 107.6 and 107.7): the private stream per
//! stage in the store's own tenant, a handle joined to a stage, and the one forgetting of a
//! stage's tenant that held-bytes rule 2 admits.
//!
//! This file holds the only product call of the provider's `forget_tenant`
//! (`crates/ekr-store/tests/eventlog_object_memo.rs`,
//! `no_source_withdraws_retained_bytes_without_an_event_on_the_object_stream`). A forgotten
//! tenant leaves no stream to append the withdrawal event to, so the event a holding handle looks
//! for is the stage record's `StagePublished` or `StageAbandoned` in the store's tenant, appended
//! before the forgetting: a handle joined to a stage reads that record before every read and
//! write.
use super::*;
use crate::{StageLog, StagePublishedRecord, StageRecord};
use ekr_core::{RevisionId, RevisionNumber};

/// `ekr.store.StageBegun`'s body.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StageBegunBody {
    stage_id: StageId,
    tenant: String,
    base: RevisionNumber,
    base_revision: RevisionId,
}
/// `ekr.store.StageSealed`'s body.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StageSealedBody {
    stage_id: StageId,
    base: RevisionNumber,
}
/// `ekr.store.StagePublished`'s body.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct StagePublishedBody {
    pub(super) stage_id: StageId,
    pub(super) base: RevisionNumber,
    pub(super) head: RevisionNumber,
    pub(super) published_first: Option<RevisionId>,
    pub(super) published_last: Option<RevisionId>,
    pub(super) occurrences: u64,
}
/// `ekr.store.StageAbandoned`'s body.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StageAbandonedBody {
    stage_id: StageId,
}

fn record_disagrees(stage: StageId, why: &str) -> StoreError {
    StoreError::Document(format!("stage-record-disagrees: {stage}: {why}"))
}

/// Folds one stage's record stream, in version order, into its record. Every event must be one of
/// the four, at schema 1, about this stage, and a move its lifecycle admits.
pub(super) fn fold_record(
    stage: StageId,
    events: &[RecordedEvent],
) -> Result<Option<StageRecord>, StoreError> {
    let Some((first, rest)) = events.split_first() else {
        return Ok(None);
    };
    let body = |event: &RecordedEvent| -> Result<serde_json::Value, StoreError> {
        if event.schema_version != 1 || event.is_redacted() {
            return Err(record_disagrees(stage, "envelope"));
        }
        Ok(event.data.clone())
    };
    if first.name != STAGE_BEGUN || first.version != 1 {
        return Err(record_disagrees(
            stage,
            "the record does not open with StageBegun",
        ));
    }
    let begun: StageBegunBody = serde_json::from_value(body(first)?).map_err(json_error)?;
    if begun.stage_id != stage {
        return Err(record_disagrees(stage, "StageBegun names another stage"));
    }
    let mut record = StageRecord {
        stage_id: stage,
        tenant: begun.tenant,
        base: begun.base,
        base_revision: begun.base_revision,
        state: StageState::Begun,
        version: 1,
        sealed_at: None,
        published: None,
    };
    for event in rest {
        if event.version != record.version + 1 {
            return Err(record_disagrees(stage, "versions do not follow"));
        }
        let data = body(event)?;
        record.state = match (event.name.as_str(), record.state) {
            (STAGE_SEALED, StageState::Begun) => {
                let sealed: StageSealedBody = serde_json::from_value(data).map_err(json_error)?;
                if sealed.stage_id != stage || sealed.base != record.base {
                    return Err(record_disagrees(stage, "StageSealed disagrees"));
                }
                record.sealed_at = Some(event.version);
                StageState::Sealing
            }
            (STAGE_PUBLISHED, StageState::Sealing) => {
                let published: StagePublishedBody =
                    serde_json::from_value(data).map_err(json_error)?;
                if published.stage_id != stage || published.base != record.base {
                    return Err(record_disagrees(stage, "StagePublished disagrees"));
                }
                record.published = Some(StagePublishedRecord {
                    head: published.head,
                    published_first: published.published_first,
                    published_last: published.published_last,
                    occurrences: published.occurrences,
                });
                StageState::Published
            }
            (STAGE_ABANDONED, StageState::Begun | StageState::Sealing) => {
                let abandoned: StageAbandonedBody =
                    serde_json::from_value(data).map_err(json_error)?;
                if abandoned.stage_id != stage {
                    return Err(record_disagrees(stage, "StageAbandoned disagrees"));
                }
                StageState::Abandoned
            }
            _ => {
                return Err(record_disagrees(
                    stage,
                    "a move the lifecycle does not admit",
                ))
            }
        };
        record.version = event.version;
    }
    Ok(Some(record))
}

impl<S: EventStore> EventlogStore<S> {
    /// The record stream of `stage` in the tenant `store`.
    pub(super) fn stage_stream(store: &TenantId, stage: StageId) -> Result<StreamId, StoreError> {
        Ok(StreamId::new(
            store.clone(),
            STAGE_STREAM_TYPE,
            stage.to_string(),
        )?)
    }
    /// Every event of `stream`, which may be in another tenant of this provider than the handle's
    /// own, checked to be that stream's in version order.
    pub(super) fn read_record_stream(
        &self,
        stream: &StreamId,
    ) -> Result<Vec<RecordedEvent>, StoreError> {
        let mut events: Vec<RecordedEvent> = Vec::new();
        loop {
            let after = events.len() as u64;
            let slice =
                self.runtime()
                    .block_on(self.store.read_stream(stream, after, MAX_READ_LIMIT))?;
            for event in slice.events {
                if event.tenant != *stream.tenant()
                    || event.stream_type != stream.stream_type()
                    || event.stream_id != stream.stream_id()
                    || event.version != events.len() as u64 + 1
                {
                    return Err(StoreError::Document("stage-record-envelope".into()));
                }
                events.push(event);
            }
            if slice.end_of_stream || events.len() as u64 == after {
                return Ok(events);
            }
        }
    }
    /// The record of `stage` in the store whose tenant is `store`, folded.
    pub(super) fn read_stage_record(
        &self,
        store: &TenantId,
        stage: StageId,
    ) -> Result<Option<StageRecord>, StoreError> {
        let events = self.read_record_stream(&Self::stage_stream(store, stage)?)?;
        fold_record(stage, &events)
    }
    /// Joins this handle, opened on the tenant of stage `stage` of the store whose tenant is
    /// `store`, to that stage (design § 107.3): from now on it reads the stage's record in the
    /// store's tenant before every read and write and again after every write lands, and refuses
    /// once the stage is not Begun.
    ///
    /// # Errors
    /// `stage-tenant-not-derived` where this handle's tenant is not the stage's derived tenant,
    /// [`StoreError::StageNotFound`], or [`StoreError::StageStateConflict`] for a stage that is
    /// not Begun.
    pub fn joined(mut self, store: &str, stage: StageId) -> Result<Self, StoreError> {
        if crate::stage_tenant(store, stage)? != self.tenant.as_str() {
            return Err(StoreError::Document(format!(
                "stage-tenant-not-derived: {} is not the tenant of stage {stage}",
                self.tenant
            )));
        }
        let store_tenant = TenantId::new(store)?;
        self.joined = Some(Joined {
            stage,
            record: Self::stage_stream(&store_tenant, stage)?,
            store_tenant,
        });
        self.check_joined()?;
        Ok(self)
    }
    /// The stage this handle is joined to, if any.
    #[must_use]
    pub fn joined_stage(&self) -> Option<StageId> {
        self.joined.as_ref().map(|joined| joined.stage)
    }
    /// The joined stage's record, where this handle is joined; the record must exist.
    fn joined_record(&self, joined: &Joined) -> Result<StageRecord, StoreError> {
        let events = self.read_record_stream(&joined.record)?;
        fold_record(joined.stage, &events)?.ok_or(StoreError::StageNotFound(joined.stage))
    }
    /// What every read and write through a joined handle does first: the stage must be Begun.
    pub(super) fn check_joined(&self) -> Result<(), StoreError> {
        let Some(joined) = &self.joined else {
            return Ok(());
        };
        let record = self.joined_record(joined)?;
        if record.state == StageState::Begun {
            Ok(())
        } else {
            Err(StoreError::StageStateConflict {
                stage_id: joined.stage,
                state: record.state,
            })
        }
    }
    /// What every write through a joined handle does after it lands: a stage still Begun had not
    /// been sealed when the write landed, so the suffix a publication captures after its seal
    /// holds it. Otherwise the write is refused `stage-write-landed` with the occurrences it
    /// landed; where the stage is Published or Abandoned its tenant is forgotten again first, so a
    /// write that landed in a tenant already forgotten leaves nothing there.
    pub(super) fn landed(&self, group: &AppendGroup) -> Result<(), StoreError> {
        let Some(joined) = &self.joined else {
            return Ok(());
        };
        let record = self.joined_record(joined)?;
        if record.state == StageState::Begun {
            return Ok(());
        }
        if matches!(record.state, StageState::Published | StageState::Abandoned) {
            self.forget_stage(&joined.store_tenant, &record)?;
        }
        let event_ids = group
            .appends
            .iter()
            .filter(|append| append.stream.stream_type() == REVISION_STREAM_TYPE)
            .flat_map(|append| append.events.iter())
            .filter_map(|event| serde_json::from_value::<RevisionEvent>(event.data.clone()).ok())
            .map(|event| event.event_id)
            .collect();
        Err(StoreError::StageWriteLanded {
            stage_id: joined.stage,
            state: record.state,
            event_ids,
        })
    }
    /// Forgets the tenant `record`, a Published or Abandoned stage of the store whose tenant is
    /// `store`, names in its `StageBegun` — the one product call of the provider's
    /// `forget_tenant` (held bytes, rule 2). It forgets only a tenant that carries the stage
    /// marker, equals the tenant derived from `store` and the stage id, and is not `store`: a
    /// `StageBegun` naming the store's own tenant, another store's or any other name is refused
    /// before anything is removed.
    pub(super) fn forget_stage(
        &self,
        store: &TenantId,
        record: &StageRecord,
    ) -> Result<(), StoreError> {
        if !matches!(record.state, StageState::Published | StageState::Abandoned) {
            return Err(StoreError::StageStateConflict {
                stage_id: record.stage_id,
                state: record.state,
            });
        }
        if self.provider == ProviderKind::File {
            return Err(StoreError::StageUnsupportedProvider(ProviderKind::File));
        }
        if self.hosted_read_only {
            return Err(StoreError::ReadOnly("hosted read handle".into()));
        }
        if let Some(read_only) = &self.read_only {
            return Err(read_only.refusal());
        }
        let derived = crate::stage_tenant(store.as_str(), record.stage_id)?;
        if record.tenant != derived
            || record.tenant == store.as_str()
            || !record.tenant.starts_with(crate::STAGE_TENANT_MARKER)
        {
            return Err(StoreError::Document(format!(
                "stage-tenant-not-derived: stage {} names {:?}, not its tenant {derived:?}; \
                 nothing was forgotten",
                record.stage_id, record.tenant
            )));
        }
        let tenant = TenantId::new(derived)?;
        self.runtime().block_on(self.store.forget_tenant(&tenant))?;
        Ok(())
    }
    /// Refuses a stage operation through a handle joined to a stage, or on a stage's tenant.
    fn on_store_tenant(&self) -> Result<(), StoreError> {
        if self.joined.is_some() || self.tenant.as_str().contains(crate::STAGE_TENANT_MARKER) {
            return Err(StoreError::Document(
                "stage-command-on-stage: a stage command runs on the store's own tenant".into(),
            ));
        }
        Ok(())
    }
    /// Appends `event` named `name` to `stage`'s record stream after `version` events.
    fn append_record(
        &self,
        stage: StageId,
        version: u64,
        name: &str,
        event: serde_json::Value,
    ) -> Result<StageRecord, StoreError>
    where
        S: AtomicBlobEventStore,
    {
        let request = BlobAppendGroup {
            group: AppendGroup {
                tenant: self.tenant.clone(),
                appends: vec![StreamAppend {
                    stream: Self::stage_stream(&self.tenant, stage)?,
                    expected: if version == 0 {
                        Expected::NoStream
                    } else {
                        Expected::Exact(version)
                    },
                    events: vec![NewEvent::new(name, 1, event)?],
                }],
                // The key names the move as well as the version: a seal and an abandonment
                // appended at one version are two requests, and the one that loses meets the
                // stream's conditional append, never the other's command receipt.
                meta: envelope(
                    &format!("ekr.stage.{stage}.{version}.{name}"),
                    ContentHash::of_bytes(format!("{stage}.{version}.{name}").as_bytes()).to_hex(),
                ),
            },
            blobs: Vec::new(),
        };
        self.atomic(&request)?;
        self.read_stage_record(&self.tenant, stage)?
            .ok_or(StoreError::StageNotFound(stage))
    }
}

impl<S: AtomicBlobEventStore> StageLog for EventlogStore<S> {
    fn provider(&self) -> ProviderKind {
        self.provider
    }
    fn store_tenant(&self) -> String {
        self.tenant.to_string()
    }
    fn stage_record(&self, stage: StageId) -> Result<Option<StageRecord>, StoreError> {
        self.entered()?;
        self.on_store_tenant()?;
        self.read_stage_record(&self.tenant, stage)
    }
    fn stages(&self) -> Result<Vec<StageRecord>, StoreError> {
        self.entered()?;
        self.on_store_tenant()?;
        let mut ids = BTreeSet::new();
        for event in self.published_events()? {
            if event.stream_type == STAGE_STREAM_TYPE {
                ids.insert(event.stream_id);
            }
        }
        let mut records = Vec::with_capacity(ids.len());
        for id in ids {
            let stage: StageId = id
                .parse()
                .map_err(|_| StoreError::Document(format!("stage-record-stream: {id}")))?;
            if let Some(record) = self.read_stage_record(&self.tenant, stage)? {
                records.push(record);
            }
        }
        Ok(records)
    }
    fn record_stage_begun(
        &self,
        stage: StageId,
        tenant: &str,
        base: RevisionNumber,
        base_revision: RevisionId,
    ) -> Result<StageRecord, StoreError> {
        self.entered()?;
        self.on_store_tenant()?;
        if crate::stage_tenant(self.tenant.as_str(), stage)? != tenant {
            return Err(StoreError::Document(format!(
                "stage-tenant-not-derived: {tenant:?} is not the tenant of stage {stage}"
            )));
        }
        let body = StageBegunBody {
            stage_id: stage,
            tenant: tenant.to_owned(),
            base,
            base_revision,
        };
        self.append_record(
            stage,
            0,
            STAGE_BEGUN,
            serde_json::to_value(body).map_err(json_error)?,
        )
    }
    fn record_stage_sealed(&self, record: &StageRecord) -> Result<StageRecord, StoreError> {
        self.entered()?;
        self.on_store_tenant()?;
        if record.state != StageState::Begun {
            return Err(StoreError::StageStateConflict {
                stage_id: record.stage_id,
                state: record.state,
            });
        }
        let body = StageSealedBody {
            stage_id: record.stage_id,
            base: record.base,
        };
        self.append_record(
            record.stage_id,
            record.version,
            STAGE_SEALED,
            serde_json::to_value(body).map_err(json_error)?,
        )
    }
    fn record_stage_abandoned(&self, record: &StageRecord) -> Result<StageRecord, StoreError> {
        self.entered()?;
        self.on_store_tenant()?;
        if !matches!(record.state, StageState::Begun | StageState::Sealing) {
            return Err(StoreError::StageStateConflict {
                stage_id: record.stage_id,
                state: record.state,
            });
        }
        let body = StageAbandonedBody {
            stage_id: record.stage_id,
        };
        self.append_record(
            record.stage_id,
            record.version,
            STAGE_ABANDONED,
            serde_json::to_value(body).map_err(json_error)?,
        )
    }
    fn stage_preparation(
        &self,
        stage: StageId,
    ) -> Result<Option<PublicationPreparationV4>, StoreError> {
        self.entered()?;
        self.on_store_tenant()?;
        self.read_stage_preparation(stage)
    }
    fn prepare_stage_publication(
        &self,
        input_hash: ContentHash,
        decision: &StagePublication,
        previous: Option<&PublicationPreparationV4>,
    ) -> Result<PublicationPreparationV4, StoreError> {
        self.entered()?;
        self.on_store_tenant()?;
        self.elect_stage_preparation(input_hash, decision, previous)
    }
    fn resume_stage_publication(
        &self,
        prepared: &PublicationPreparationV4,
    ) -> Result<Appended, StoreError> {
        self.entered()?;
        self.on_store_tenant()?;
        self.resume_stage_preparation(prepared)
    }
    fn held_classes(
        &self,
        hashes: &BTreeSet<ContentHash>,
    ) -> Result<BTreeMap<ContentHash, StorageClass>, StoreError> {
        self.entered()?;
        let mut held = BTreeMap::new();
        for hash in hashes {
            if let Some((CheckedObject(object), _)) = self.object_versioned(*hash)? {
                held.insert(*hash, object.metadata.storage_class);
            }
        }
        Ok(held)
    }
    fn revision_stream_version(&self) -> Result<u64, StoreError> {
        self.entered()?;
        Ok(self.occurrences(MAX_READ_LIMIT, None)?.len() as u64)
    }
    fn forget_stage_tenant(&self, stage: StageId) -> Result<(), StoreError> {
        self.entered()?;
        self.on_store_tenant()?;
        let record = self
            .read_stage_record(&self.tenant, stage)?
            .ok_or(StoreError::StageNotFound(stage))?;
        self.forget_stage(&self.tenant, &record)
    }
}
