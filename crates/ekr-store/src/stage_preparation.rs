//! A stage's publication, elected in `ekr.publication-preparation/4` (design §§ 94, 107.4 and
//! 107.5): one slot per stage, whose decision is the whole suffix and whose native request is the
//! whole append group in the store's tenant.
//!
//! The group is admitted only after the store's injected [`crate::CommitAuthority`] verifies the
//! store's revision stream, at the version the group expects, with the suffix after it, exactly
//! as a single publication is admitted (invariant 1). It is retried as elected after an uncertain
//! outcome, and a definitive conflict is answered by what moved.
use super::preparation::{native_expected, private_key, require, Selection};
use super::stage_record::StagePublishedBody;
use super::*;
use crate::StageRecord;
use ekr_core::{RevisionId, RevisionNumber};
use eventlog_core::MAX_EVENTS_PER_APPEND;

/// The slot of a stage's publication (`ekr.store.StagePublicationCommandKey`): Publish and the
/// stage id.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagePublicationCommandKey {
    /// Always [`PublicationCommandKind::PublishStage`].
    pub kind: PublicationCommandKind,
    /// The stage published.
    pub stage_id: StageId,
}

impl StagePublicationCommandKey {
    /// The key of `stage`'s publication slot.
    #[must_use]
    pub const fn of(stage: StageId) -> Self {
        Self {
            kind: PublicationCommandKind::PublishStage,
            stage_id: stage,
        }
    }
    fn slot(&self) -> Result<String, StoreError> {
        require(
            self.kind == PublicationCommandKind::PublishStage,
            "preparation-command-key",
        )?;
        Ok(ContentHash::of_bytes(&serde_json::to_vec(self).map_err(json_error)?).to_hex())
    }
}

/// One object a stage's publication appends (`ekr.store.StagePublicationObject`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagePublicationObject {
    /// Its first class in the stage.
    pub storage_class: StorageClass,
    /// The class of each retention raise the stage recorded after it, in order.
    pub raised_to: Vec<StorageClass>,
    /// Its original storage time.
    pub stored_at: Timestamp,
    /// Its bytes.
    #[serde(with = "ekr_core::bytes::base64")]
    pub bytes: Vec<u8>,
}

impl StagePublicationObject {
    /// Its classes in order, the first and then each raise: strictly stronger each time.
    fn rungs(&self) -> Result<Vec<StorageClass>, StoreError> {
        let rungs: Vec<StorageClass> = std::iter::once(self.storage_class)
            .chain(self.raised_to.iter().copied())
            .collect();
        require(
            rungs
                .windows(2)
                .all(|pair| pair[1].retention_rank() > pair[0].retention_rank()),
            "stage-object-ladder",
        )?;
        Ok(rungs)
    }
    /// The strongest class it reaches.
    fn class(&self) -> StorageClass {
        self.raised_to.last().copied().unwrap_or(self.storage_class)
    }
}

/// The decision of a stage's publication (`ekr.store.StagePublication`): the whole suffix.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StagePublication {
    /// The stage published.
    pub stage_id: StageId,
    /// Its base: the store's head it was begun at, which must still be the store's head.
    pub base: RevisionNumber,
    /// The store's revision-stream version the attempt's group expects.
    pub expected_version: u64,
    /// The stage's occurrences after its copy, derived again for the store's lineage, in order.
    pub occurrences: Vec<RevisionEvent>,
    /// Every object they and the run's other writes need, by content hash.
    #[serde(deserialize_with = "ekr_core::decode::unique_map")]
    pub objects: BTreeMap<ContentHash, StagePublicationObject>,
    /// The store's head after the group.
    pub head: RevisionNumber,
    /// The first revision the group adds, absent where the run committed nothing.
    pub published_first: Option<RevisionId>,
    /// The last revision the group adds.
    pub published_last: Option<RevisionId>,
}

impl StagePublication {
    /// The `ekr.store.StagePublished` body the group appends.
    fn published(&self) -> StagePublishedBody {
        StagePublishedBody {
            stage_id: self.stage_id,
            base: self.base,
            head: self.head,
            published_first: self.published_first,
            published_last: self.published_last,
            occurrences: self.occurrences.len() as u64,
        }
    }
    /// The decision with `expected_version` set aside: what a successor attempt keeps.
    fn suffix(&self) -> Self {
        Self {
            expected_version: 0,
            ..self.clone()
        }
    }
}

/// An elected attempt of a stage's publication (`ekr.store.PublicationPreparationV4`). Its strict
/// bytes live only in the private provider namespace.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicationPreparationV4 {
    /// Exactly `ekr.publication-preparation/4`.
    pub format: String,
    /// The stage's slot.
    pub command_key: StagePublicationCommandKey,
    /// Actual command input, context and host anchor hash.
    pub input_hash: ContentHash,
    /// Immutable elected decision.
    pub decision: StagePublication,
    /// Zero-based attempt number.
    pub attempt_number: u64,
    /// Previous private record's payload address, absent at election.
    pub previous_attempt_hash: Option<ContentHash>,
    /// Exact native request: the whole append group.
    pub native_request: NativePublicationRequest,
    /// Provider-computed actual fingerprint.
    pub native_fingerprint: String,
}

impl PublicationPreparationV4 {
    /// The private payload format of a stage's publication.
    pub const FORMAT: &'static str = "ekr.publication-preparation/4";
    fn hash(&self) -> Result<ContentHash, StoreError> {
        Ok(ContentHash::of_bytes(
            &serde_json::to_vec(self).map_err(json_error)?,
        ))
    }
}

/// The retained layout: byte strings as base64, and no blob list, which is the decision's objects
/// in address order.
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordV4 {
    format: String,
    command_key: StagePublicationCommandKey,
    input_hash: ContentHash,
    decision: StagePublication,
    attempt_number: u64,
    previous_attempt_hash: Option<ContentHash>,
    native_request: RequestV4,
    native_fingerprint: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestV4 {
    tenant: String,
    appends: Vec<AppendV4>,
    meta: NativeCommandMeta,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AppendV4 {
    stream: NativeStreamId,
    expected: NativeExpected,
    events: Vec<EventV4>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct EventV4 {
    name: String,
    schema_version: u32,
    #[serde(with = "ekr_core::bytes::base64")]
    data: Vec<u8>,
}

impl Serialize for PublicationPreparationV4 {
    fn serialize<Z: serde::Serializer>(&self, serializer: Z) -> Result<Z::Ok, Z::Error> {
        RecordV4 {
            format: self.format.clone(),
            command_key: self.command_key.clone(),
            input_hash: self.input_hash,
            decision: self.decision.clone(),
            attempt_number: self.attempt_number,
            previous_attempt_hash: self.previous_attempt_hash,
            native_request: RequestV4 {
                tenant: self.native_request.tenant.clone(),
                appends: self
                    .native_request
                    .appends
                    .iter()
                    .map(|append| AppendV4 {
                        stream: append.stream.clone(),
                        expected: append.expected.clone(),
                        events: append
                            .events
                            .iter()
                            .map(|event| EventV4 {
                                name: event.name.clone(),
                                schema_version: event.schema_version,
                                data: event.data.clone(),
                            })
                            .collect(),
                    })
                    .collect(),
                meta: self.native_request.meta.clone(),
            },
            native_fingerprint: self.native_fingerprint.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PublicationPreparationV4 {
    /// Only `/4`'s layout; the blob list is rebuilt from the decision's objects in address order.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let read = RecordV4::deserialize(deserializer)?;
        if read.format != Self::FORMAT {
            return Err(serde::de::Error::custom("preparation-format"));
        }
        let blobs = blob_list(&read.decision);
        Ok(Self {
            format: read.format,
            command_key: read.command_key,
            input_hash: read.input_hash,
            decision: read.decision,
            attempt_number: read.attempt_number,
            previous_attempt_hash: read.previous_attempt_hash,
            native_request: NativePublicationRequest {
                tenant: read.native_request.tenant,
                appends: read
                    .native_request
                    .appends
                    .into_iter()
                    .map(|append| NativeStreamAppend {
                        stream: append.stream,
                        expected: append.expected,
                        events: append
                            .events
                            .into_iter()
                            .map(|event| NativeNewEvent {
                                name: event.name,
                                schema_version: event.schema_version,
                                data: event.data,
                            })
                            .collect(),
                    })
                    .collect(),
                meta: read.native_request.meta,
                blobs,
            },
            native_fingerprint: read.native_fingerprint,
        })
    }
}

/// The provider's fingerprint of `request`: of the blob group, or of the plain group where the
/// suffix carries no object and the group binds no blob (a blob group must bind one).
fn fingerprint(request: &BlobAppendGroup) -> Result<String, StoreError> {
    Ok(if request.blobs.is_empty() {
        request.group.fingerprint()?
    } else {
        request.fingerprint()?
    })
}

/// The decision's objects as the group binds them: every one, in address order.
fn blob_list(decision: &StagePublication) -> Vec<NativeBlobWrite> {
    decision
        .objects
        .iter()
        .map(|(hash, object)| NativeBlobWrite {
            digest: hash.to_hex(),
            bytes: object.bytes.clone(),
        })
        .collect()
}

/// A successor keeps the slot, the input and every member of the decision but the version its
/// group expects (design § 94.2).
fn check_stage_successor(
    prior: &PublicationPreparationV4,
    next: &PublicationPreparationV4,
) -> Result<(), StoreError> {
    require(
        prior.command_key == next.command_key
            && prior.input_hash == next.input_hash
            && next.attempt_number == prior.attempt_number + 1
            && next.previous_attempt_hash == Some(prior.hash()?)
            && prior.decision.suffix() == next.decision.suffix(),
        "preparation-successor-chain",
    )
}

/// The events that store or raise an object through `rungs` from `from`, the class the store
/// holds it at, or from nothing: an `ObjectStored` at the first rung, then a raise to each later
/// rung stronger than the class reached.
fn object_events(
    hash: ContentHash,
    object: &StagePublicationObject,
    rungs: &[StorageClass],
    from: Option<StorageClass>,
) -> Result<Vec<NewEvent>, StoreError> {
    let mut events = Vec::new();
    let mut class = match from {
        Some(class) => class,
        None => {
            let first = rungs[0];
            events.push(NewEvent::new(
                OBJECT_STORED,
                2,
                serde_json::to_value(ObjectMetadata {
                    content_hash: hash,
                    storage_class: first,
                    byte_len: object.bytes.len() as u64,
                    stored_at: object.stored_at,
                })
                .map_err(json_error)?,
            )?);
            first
        }
    };
    for rung in rungs {
        if rung.retention_rank() > class.retention_rank() {
            events.push(NewEvent::new(
                OBJECT_RETENTION_RAISED,
                1,
                serde_json::to_value(RetentionRaised {
                    content_hash: hash,
                    from: class,
                    to: *rung,
                })
                .map_err(json_error)?,
            )?);
            class = *rung;
        }
    }
    Ok(events)
}

impl<S: AtomicBlobEventStore> EventlogStore<S> {
    fn stage_preparation_stream(
        &self,
        key: &StagePublicationCommandKey,
    ) -> Result<StreamId, StoreError> {
        Ok(StreamId::new(
            self.tenant.clone(),
            "ekr.preparation",
            key.slot()?,
        )?)
    }
    /// The stage's record, which must be Sealing, for a publication's group.
    fn sealing(&self, stage: StageId) -> Result<StageRecord, StoreError> {
        let record = self
            .read_stage_record(&self.tenant, stage)?
            .ok_or(StoreError::StageNotFound(stage))?;
        if record.state == StageState::Sealing {
            Ok(record)
        } else {
            Err(StoreError::StageStateConflict {
                stage_id: stage,
                state: record.state,
            })
        }
    }
    /// The append of `object` the group makes against what the store holds now, if any.
    fn stage_object_append(
        &self,
        hash: ContentHash,
        object: &StagePublicationObject,
    ) -> Result<Option<StreamAppend>, StoreError> {
        require(
            ContentHash::of_bytes(&object.bytes) == hash,
            "staged-object-address",
        )?;
        let rungs = object.rungs()?;
        let (expected, from) = match self.object_versioned(hash)? {
            Some((CheckedObject(held), version)) => {
                require(*held.bytes == object.bytes, "object-address-collision")?;
                (Expected::Exact(version), Some(held.metadata.storage_class))
            }
            None => (Expected::NoStream, None),
        };
        let events = object_events(hash, object, &rungs, from)?;
        if events.is_empty() {
            return Ok(None);
        }
        Ok(Some(StreamAppend {
            stream: self.object_stream(hash)?,
            expected,
            events,
        }))
    }
    /// The whole group of attempt `attempt` of `decision`, against the store as it reads now.
    fn native_for_stage(
        &self,
        decision: &StagePublication,
        attempt: u64,
    ) -> Result<BlobAppendGroup, StoreError> {
        let record = self.sealing(decision.stage_id)?;
        let mut appends = Vec::new();
        for (index, chunk) in decision
            .occurrences
            .chunks(MAX_EVENTS_PER_APPEND)
            .enumerate()
        {
            let version = decision.expected_version + (index * MAX_EVENTS_PER_APPEND) as u64;
            appends.push(StreamAppend {
                stream: self.revision_stream()?,
                expected: if version == 0 {
                    Expected::NoStream
                } else {
                    Expected::Exact(version)
                },
                events: chunk
                    .iter()
                    .map(|event| {
                        Ok(NewEvent::new(
                            event.name(),
                            2,
                            serde_json::to_value(event).map_err(json_error)?,
                        )?)
                    })
                    .collect::<Result<_, StoreError>>()?,
            });
        }
        for (hash, object) in &decision.objects {
            if let Some(append) = self.stage_object_append(*hash, object)? {
                appends.push(append);
            }
        }
        appends.push(StreamAppend {
            stream: Self::stage_stream(&self.tenant, decision.stage_id)?,
            expected: Expected::Exact(record.version),
            events: vec![NewEvent::new(
                STAGE_PUBLISHED,
                1,
                serde_json::to_value(decision.published()).map_err(json_error)?,
            )?],
        });
        let digest =
            ContentHash::of_bytes(&serde_json::to_vec(decision).map_err(json_error)?).to_hex();
        Ok(BlobAppendGroup {
            group: AppendGroup {
                tenant: self.tenant.clone(),
                appends,
                meta: envelope(
                    &format!("ekr.stage-publish.{}.{attempt}", decision.stage_id),
                    digest,
                ),
            },
            blobs: decision
                .objects
                .iter()
                .map(|(hash, object)| BlobWrite {
                    digest: hash.to_hex(),
                    bytes: object.bytes.clone(),
                })
                .collect(),
        })
    }
    /// Checks an attempt against itself, the stage's record and the store, and has the store's
    /// authority verify the store's revision stream at the attempt's version with the suffix after
    /// it; returns the group it appends.
    fn authorize_stage_preparation(
        &self,
        prepared: &PublicationPreparationV4,
    ) -> Result<BlobAppendGroup, StoreError> {
        let decision = &prepared.decision;
        let stage = decision.stage_id;
        require(
            prepared.format == PublicationPreparationV4::FORMAT,
            "preparation-format",
        )?;
        require(
            prepared.command_key == StagePublicationCommandKey::of(stage),
            "preparation-command-key",
        )?;
        let request = prepared.native_request.restore()?;
        require(
            fingerprint(&request)? == prepared.native_fingerprint,
            "preparation-fingerprint",
        )?;
        require(request.group.tenant == self.tenant, "preparation-tenant")?;
        let digest =
            ContentHash::of_bytes(&serde_json::to_vec(decision).map_err(json_error)?).to_hex();
        let expected_meta = NativeCommandMeta::capture(&envelope(
            &format!("ekr.stage-publish.{stage}.{}", prepared.attempt_number),
            digest,
        ))?;
        require(
            prepared.native_request.meta == expected_meta,
            "preparation-native-metadata",
        )?;
        require(
            prepared.native_request.blobs == blob_list(decision),
            "preparation-blob-set",
        )?;
        for (hash, object) in &decision.objects {
            require(
                ContentHash::of_bytes(&object.bytes) == *hash,
                "preparation-object-address",
            )?;
            object.rungs()?;
        }
        for event in &decision.occurrences {
            require(
                event.format == RevisionEvent::FORMAT
                    && decision.objects.contains_key(&event.record_hash),
                "preparation-decision",
            )?;
        }
        let record = self
            .read_stage_record(&self.tenant, stage)?
            .ok_or(StoreError::StageNotFound(stage))?;
        let sealed_at = record
            .sealed_at
            .ok_or_else(|| StoreError::Document("preparation-stage-not-sealed".into()))?;
        let mut appends = request.group.appends.iter().peekable();
        // The revision stream, in chunks, each expecting the stream after the one before.
        let mut consumed = 0usize;
        while let Some(append) = appends.next_if(|append| {
            append.stream.stream_type() == REVISION_STREAM_TYPE
                && append.stream.stream_id() == REVISION_STREAM_ID
        }) {
            require(
                append.stream == self.revision_stream()?
                    && native_expected(append.expected)?
                        == native_expected(Expected::Exact(
                            decision.expected_version + consumed as u64,
                        ))?,
                "preparation-revision-append",
            )?;
            for event in &append.events {
                let occurrence = decision
                    .occurrences
                    .get(consumed)
                    .ok_or_else(|| StoreError::Document("preparation-revision-event".into()))?;
                require(
                    event.name == occurrence.name()
                        && event.schema_version == 2
                        && event.data == serde_json::to_value(occurrence).map_err(json_error)?,
                    "preparation-revision-event",
                )?;
                consumed += 1;
            }
        }
        require(
            consumed == decision.occurrences.len(),
            "preparation-revision-append",
        )?;
        // Each object stream, once, with exactly the events its ladder needs from what the store
        // held when the attempt was elected.
        let mut appended = BTreeSet::new();
        while let Some(append) =
            appends.next_if(|append| append.stream.stream_type() == OBJECT_STREAM_TYPE)
        {
            let hash: ContentHash = append
                .stream
                .stream_id()
                .parse()
                .map_err(|_| StoreError::Document("preparation-object-stream".into()))?;
            require(
                append.stream == self.object_stream(hash)?,
                "preparation-object-stream",
            )?;
            let object = decision
                .objects
                .get(&hash)
                .ok_or_else(|| StoreError::Document("preparation-extra-object".into()))?;
            require(appended.insert(hash), "preparation-duplicate-object-append")?;
            let from = match append.expected {
                Expected::NoStream => None,
                Expected::Exact(version) if version > 0 => Some(self.retention_at(hash, version)?),
                _ => {
                    return Err(StoreError::Document(
                        "preparation-object-expectation".into(),
                    ))
                }
            };
            let wanted = object_events(hash, object, &object.rungs()?, from)?;
            require(
                !wanted.is_empty()
                    && wanted.len() == append.events.len()
                    && wanted.iter().zip(&append.events).all(|(want, have)| {
                        want.name == have.name
                            && want.schema_version == have.schema_version
                            && want.data == have.data
                    }),
                "preparation-object-metadata",
            )?;
        }
        // The stage record, at the version it became Sealing, with this decision's record.
        let last = appends
            .next()
            .ok_or_else(|| StoreError::Document("preparation-stage-record".into()))?;
        require(
            appends.next().is_none()
                && last.stream == Self::stage_stream(&self.tenant, stage)?
                && native_expected(last.expected)? == native_expected(Expected::Exact(sealed_at))?
                && last.events.len() == 1
                && last.events[0].name == STAGE_PUBLISHED
                && last.events[0].schema_version == 1
                && last.events[0].data
                    == serde_json::to_value(decision.published()).map_err(json_error)?,
            "preparation-stage-record",
        )?;
        for (hash, object) in &decision.objects {
            if !appended.contains(hash) {
                let held = self.object(*hash)?.ok_or_else(|| {
                    StoreError::Document("preparation-object-metadata-missing".into())
                })?;
                require(
                    *held.bytes == object.bytes
                        && held.metadata.storage_class.retention_rank()
                            >= object.class().retention_rank(),
                    "preparation-unstated-object-binding",
                )?;
            }
        }
        // Admission: the store's stream at the attempt's version, then the suffix, through the
        // store's authority.
        let mut history = RetainedHistory {
            occurrences: self.occurrences(MAX_READ_LIMIT, None)?,
            objects: BTreeMap::new(),
        };
        require(
            history.occurrences.len() as u64 >= decision.expected_version,
            "preparation-missing-basis-history",
        )?;
        history.occurrences.truncate(
            usize::try_from(decision.expected_version)
                .map_err(|_| StoreError::Document("preparation-position-overflow".into()))?,
        );
        for event in &decision.occurrences {
            if let Some(held) = history
                .occurrences
                .iter()
                .find(|held| held.event.event_id == event.event_id)
            {
                return Err(StoreError::Document(if held.event == *event {
                    "occurrence-already-retained".into()
                } else {
                    "occurrence-identity-conflict".into()
                }));
            }
        }
        let mut required = BTreeSet::new();
        for held in &history.occurrences {
            required.insert(held.event.record_hash);
            if let RevisionPayload::Seeded { seed_hash, .. } = held.event.payload {
                required.insert(seed_hash);
            }
        }
        self.load_objects(&mut history, required)?;
        for (hash, object) in &decision.objects {
            history.objects.insert(
                *hash,
                RetainedObject {
                    metadata: StoredObject {
                        content_hash: *hash,
                        storage_class: object.class(),
                        byte_len: object.bytes.len() as u64,
                        stored_at: object.stored_at,
                    },
                    bytes: Arc::new(object.bytes.clone()),
                },
            );
        }
        for event in &decision.occurrences {
            history.occurrences.push(RecordedOccurrence {
                version: history.occurrences.len() as u64 + 1,
                provider_event_id: format!("prepared-stage-occurrence:{}", event.event_id),
                event: event.clone(),
            });
        }
        self.verify_replayed(&mut history)?;
        Ok(request)
    }
    /// The elected attempt of `stage`'s publication, each attempt of its chain authorized once.
    pub(super) fn read_stage_preparation(
        &self,
        stage: StageId,
    ) -> Result<Option<PublicationPreparationV4>, StoreError> {
        let key = StagePublicationCommandKey::of(stage);
        let events = self.read_all(&self.stage_preparation_stream(&key)?, MAX_READ_LIMIT)?;
        let mut previous = None;
        let mut selected: Option<PublicationPreparationV4> = None;
        for (position, event) in events.into_iter().enumerate() {
            require(
                event.name == "ekr.store.PublicationPrepared" && event.schema_version == 1,
                "preparation-selection-envelope",
            )?;
            let selection: Selection = serde_json::from_value(event.data).map_err(json_error)?;
            require(
                selection.attempt_number == position as u64
                    && selection.previous_attempt_hash == previous,
                "preparation-selection-chain",
            )?;
            let bytes = self
                .runtime()
                .block_on(
                    self.store
                        .get_blob(&self.tenant, &private_key(selection.preparation_hash)),
                )?
                .ok_or_else(|| StoreError::Document("preparation-bytes-missing".into()))?;
            require(
                ContentHash::of_bytes(&bytes) == selection.preparation_hash,
                "preparation-address",
            )?;
            let prepared: PublicationPreparationV4 =
                serde_json::from_slice(&bytes).map_err(json_error)?;
            require(
                prepared.command_key == key
                    && prepared.attempt_number == selection.attempt_number
                    && prepared.previous_attempt_hash == previous,
                "preparation-record-chain",
            )?;
            if let Some(prior) = &selected {
                check_stage_successor(prior, &prepared)?;
            }
            let authorized = self
                .authorized
                .lock()
                .is_ok_and(|held| held.contains(&selection.preparation_hash));
            if !authorized {
                self.authorize_stage_preparation(&prepared)?;
                if let Ok(mut held) = self.authorized.lock() {
                    held.insert(selection.preparation_hash);
                }
            }
            previous = Some(selection.preparation_hash);
            selected = Some(prepared);
        }
        Ok(selected)
    }
    /// Elects `decision`, or a successor of `previous`; returns an attempt already elected for the
    /// same input.
    pub(super) fn elect_stage_preparation(
        &self,
        input_hash: ContentHash,
        decision: &StagePublication,
        previous: Option<&PublicationPreparationV4>,
    ) -> Result<PublicationPreparationV4, StoreError> {
        let key = StagePublicationCommandKey::of(decision.stage_id);
        let held = self.read_stage_preparation(decision.stage_id)?;
        if let Some(held) = held.as_ref() {
            if held.input_hash != input_hash {
                return Err(StoreError::PublicationInputConflict);
            }
            if previous.is_none_or(|prior| prior != held) {
                return Ok(held.clone());
            }
        }
        if let Some(prior) = previous {
            require(
                held.as_ref() == Some(prior),
                "preparation-predecessor-not-elected",
            )?;
        }
        let attempt_number = previous.map_or(Ok(0), |prior| {
            prior
                .attempt_number
                .checked_add(1)
                .ok_or_else(|| StoreError::Document("preparation-attempt-overflow".into()))
        })?;
        let request = self.native_for_stage(decision, attempt_number)?;
        let prepared = PublicationPreparationV4 {
            format: PublicationPreparationV4::FORMAT.into(),
            command_key: key.clone(),
            input_hash,
            decision: decision.clone(),
            attempt_number,
            previous_attempt_hash: previous.map(PublicationPreparationV4::hash).transpose()?,
            native_fingerprint: fingerprint(&request)?,
            native_request: NativePublicationRequest::capture(&request)?,
        };
        if let Some(prior) = previous {
            check_stage_successor(prior, &prepared)?;
        }
        self.authorize_stage_preparation(&prepared)?;
        let bytes = serde_json::to_vec(&prepared).map_err(json_error)?;
        let hash = ContentHash::of_bytes(&bytes);
        if let Ok(mut held) = self.authorized.lock() {
            held.insert(hash);
        }
        let selection = Selection {
            preparation_hash: hash,
            attempt_number,
            previous_attempt_hash: prepared.previous_attempt_hash,
        };
        let native = BlobAppendGroup {
            group: AppendGroup {
                tenant: self.tenant.clone(),
                appends: vec![StreamAppend {
                    stream: self.stage_preparation_stream(&key)?,
                    expected: if previous.is_none() {
                        Expected::NoStream
                    } else {
                        Expected::Exact(attempt_number)
                    },
                    events: vec![NewEvent::new(
                        "ekr.store.PublicationPrepared",
                        1,
                        serde_json::to_value(selection).map_err(json_error)?,
                    )?],
                }],
                meta: envelope(
                    &format!("ekr.prepare.{}.{hash}", key.slot()?),
                    hash.to_hex(),
                ),
            },
            blobs: vec![BlobWrite {
                digest: private_key(hash),
                bytes,
            }],
        };
        match self.atomic(&native) {
            Ok(_) => Ok(prepared),
            Err(error @ (StoreError::Conflict | StoreError::UnknownCommit)) => {
                match self.read_stage_preparation(decision.stage_id)? {
                    Some(winner) if winner.input_hash == input_hash => Ok(winner),
                    Some(_) => Err(StoreError::PublicationInputConflict),
                    None => Err(error),
                }
            }
            Err(error) => Err(error),
        }
    }
    /// Appends the exact elected group; a definitive conflict is answered by what moved.
    pub(super) fn resume_stage_preparation(
        &self,
        prepared: &PublicationPreparationV4,
    ) -> Result<Appended, StoreError> {
        let elected = self
            .read_stage_preparation(prepared.decision.stage_id)?
            .ok_or_else(|| StoreError::Document("preparation-not-elected".into()))?;
        if elected != *prepared {
            return Err(StoreError::Conflict);
        }
        let request = elected.native_request.restore()?;
        match self.atomic(&request) {
            Ok(result) => Ok(if result.deduplicated {
                Appended::AlreadyRecorded
            } else {
                Appended::Written
            }),
            Err(StoreError::Conflict) => match self.what_moved(&elected, &request)? {
                None => Ok(Appended::AlreadyRecorded),
                Some(refusal) => Err(refusal),
            },
            Err(error) => Err(error),
        }
    }
    /// What moved after an attempt was elected, read again in design § 107.4's order: the stage
    /// record, the head, the revision stream, then each object stream. `None` where the stage is
    /// Published, by this slot's group.
    fn what_moved(
        &self,
        prepared: &PublicationPreparationV4,
        request: &BlobAppendGroup,
    ) -> Result<Option<StoreError>, StoreError> {
        let decision = &prepared.decision;
        let stage = decision.stage_id;
        let record = self
            .read_stage_record(&self.tenant, stage)?
            .ok_or(StoreError::StageNotFound(stage))?;
        match record.state {
            StageState::Published => return Ok(None),
            StageState::Abandoned => {
                return Ok(Some(StoreError::StageStateConflict {
                    stage_id: stage,
                    state: StageState::Abandoned,
                }))
            }
            StageState::Begun | StageState::Sealing => {}
        }
        let occurrences = self.occurrences(MAX_READ_LIMIT, None)?;
        let head = occurrences
            .iter()
            .rev()
            .find_map(|held| match held.event.payload {
                RevisionPayload::Seeded { .. } => Some(RevisionNumber::SEED),
                RevisionPayload::RevisionCommitted { number, .. } => Some(number),
                _ => None,
            })
            .unwrap_or(RevisionNumber::SEED);
        if head != decision.base {
            return Ok(Some(StoreError::StageHeadMoved {
                stage_id: stage,
                expected: decision.base,
                base: decision.base,
                current: head,
            }));
        }
        let current = occurrences.len() as u64;
        if current != decision.expected_version {
            return Ok(Some(StoreError::StageStreamMoved {
                stage_id: stage,
                captured: decision.expected_version,
                current,
            }));
        }
        for append in &request.group.appends {
            if append.stream.stream_type() != OBJECT_STREAM_TYPE {
                continue;
            }
            let version = self.read_all(&append.stream, MAX_READ_LIMIT)?.len() as u64;
            let expected = match append.expected {
                Expected::Exact(version) => version,
                _ => 0,
            };
            if version != expected {
                let content_hash = append
                    .stream
                    .stream_id()
                    .parse()
                    .map_err(|_| StoreError::Document("preparation-object-stream".into()))?;
                return Ok(Some(StoreError::StageObjectMoved {
                    stage_id: stage,
                    content_hash,
                }));
            }
        }
        Ok(Some(StoreError::Conflict))
    }
}
