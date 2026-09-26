//! Private, versioned publication recovery. These records never confer canonical authority.
use super::*;
use ekr_core::{EventId, TransactionId};
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use std::fmt;

/// Logical command slot; input and fresh identities never allocate competing slots.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationCommandKey {
    /// Supported command family.
    pub kind: PublicationCommandKind,
    /// Required except for the one bootstrap slot.
    pub transaction_id: Option<TransactionId>,
    /// Required for validation and commit, tying the slot to its retained predecessor.
    pub predecessor_event_id: Option<EventId>,
    /// Complete predecessor payload address.
    pub predecessor_record_hash: Option<ContentHash>,
}
/// Command families which elect immutable decisions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PublicationCommandKind {
    /// One slot for the entire lineage.
    Bootstrap,
    /// One slot per proposed transaction.
    Propose,
    /// One slot per retained proposal.
    Validate,
    /// One slot per retained validation.
    Commit,
}
impl PublicationCommandKey {
    fn check(&self) -> Result<(), StoreError> {
        let valid = match self.kind {
            PublicationCommandKind::Bootstrap => {
                self.transaction_id.is_none()
                    && self.predecessor_event_id.is_none()
                    && self.predecessor_record_hash.is_none()
            }
            PublicationCommandKind::Propose => {
                self.transaction_id.is_some()
                    && self.predecessor_event_id.is_none()
                    && self.predecessor_record_hash.is_none()
            }
            PublicationCommandKind::Validate | PublicationCommandKind::Commit => {
                self.transaction_id.is_some()
                    && self.predecessor_event_id.is_some()
                    && self.predecessor_record_hash.is_some()
            }
        };
        require(valid, "preparation-command-key")
    }
    fn slot(&self) -> Result<String, StoreError> {
        self.check()?;
        Ok(ContentHash::of_bytes(&serde_json::to_vec(self).map_err(json_error)?).to_hex())
    }
}
/// Strict native expectation discriminator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NativeExpectedKind {
    /// Unconditional provider expectation; not used by EKR publication.
    Any,
    /// The stream must not exist.
    NoStream,
    /// Exact retained stream position.
    Exact,
}
/// Complete native expectation, with absent versus present version checked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeExpected {
    /// Expected kind.
    pub kind: NativeExpectedKind,
    /// Present only for Exact.
    pub version: Option<u64>,
}
/// Native stream coordinates.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeStreamId {
    /// Tenant identity.
    pub tenant: String,
    /// Stream family.
    pub stream_type: String,
    /// Opaque stream identity.
    pub stream_id: String,
}
/// Complete native event; JSON object bytes are decoded strictly before use.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeNewEvent {
    /// Exact event name.
    pub name: String,
    /// Backend schema discriminator.
    pub schema_version: u32,
    /// JSON object bytes, retaining integer values without a float conversion.
    pub data: Vec<u8>,
}
/// One ordered stream append.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeStreamAppend {
    /// Exact stream coordinates.
    pub stream: NativeStreamId,
    /// Exact conditional expectation.
    pub expected: NativeExpected,
    /// Ordered complete events.
    pub events: Vec<NativeNewEvent>,
}
/// A native single-stream claim; atomic group recovery refuses any present value.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeClaim {
    /// Scope.
    pub scope: String,
    /// Key.
    pub key: String,
    /// Digest.
    pub digest: String,
}
/// Every native metadata field, including exact nanoseconds and original UTC offset.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeCommandMeta {
    /// Native retry identity.
    pub idempotency_key: String,
    /// Caller request digest.
    pub request_hash: String,
    /// Native subject.
    pub subject: String,
    /// Native actor.
    pub actor: String,
    /// Request identity.
    pub request_id: String,
    /// Trace identity.
    pub trace_id: String,
    /// Optional originating event.
    pub causation_id: Option<String>,
    /// Native causal depth.
    pub causation_depth: u32,
    /// Exact signed Unix nanoseconds in decimal.
    pub occurred_at_unix_nanos: String,
    /// Original offset in seconds.
    pub occurred_at_offset_seconds: i32,
    /// Must be absent for an atomic group.
    pub claim: Option<NativeClaim>,
}
/// One ordered native blob binding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeBlobWrite {
    /// Exact native address.
    pub digest: String,
    /// Exact payload bytes.
    pub bytes: Vec<u8>,
}
/// Exact native request, independent of future provider/object stream movement.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativePublicationRequest {
    /// Native tenant.
    pub tenant: String,
    /// Ordered appends.
    pub appends: Vec<NativeStreamAppend>,
    /// Complete native metadata.
    pub meta: NativeCommandMeta,
    /// Ordered retained bindings.
    pub blobs: Vec<NativeBlobWrite>,
}
/// An elected immutable attempt. Its strict bytes live only in the private provider namespace.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicationPreparationV1 {
    /// Must equal `ekr.publication-preparation/1`.
    pub format: String,
    /// Logical CAS slot.
    pub command_key: PublicationCommandKey,
    /// Actual command input, context and host anchor hash.
    pub input_hash: ContentHash,
    /// Immutable elected domain decision.
    pub decision: Publication,
    /// Zero-based attempt number.
    pub attempt_number: u64,
    /// Previous private record's payload address, absent at election.
    pub previous_attempt_hash: Option<ContentHash>,
    /// Exact native request.
    pub native_request: NativePublicationRequest,
    /// Provider-computed actual fingerprint.
    pub native_fingerprint: String,
}
impl PublicationPreparationV1 {
    /// Exact private payload format.
    pub const FORMAT: &'static str = "ekr.publication-preparation/1";
    fn hash(&self) -> Result<ContentHash, StoreError> {
        Ok(ContentHash::of_bytes(
            &serde_json::to_vec(self).map_err(json_error)?,
        ))
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    preparation_hash: ContentHash,
    attempt_number: u64,
    previous_attempt_hash: Option<ContentHash>,
}
fn require(condition: bool, code: &str) -> Result<(), StoreError> {
    if condition {
        Ok(())
    } else {
        Err(StoreError::Document(code.into()))
    }
}
fn private_key(hash: ContentHash) -> String {
    format!("ekr.private.preparation.{hash}")
}
/// The preparation's record of an expectation. Eventlog 0.4.0 adds a merge expectation for forked
/// streams, which no store this crate opens can hold and no preparation can record, so it and any
/// later kind are refused rather than recorded as a kind they are not.
pub(super) fn native_expected(value: Expected) -> Result<NativeExpected, StoreError> {
    Ok(match value {
        Expected::Any => NativeExpected {
            kind: NativeExpectedKind::Any,
            version: None,
        },
        Expected::NoStream => NativeExpected {
            kind: NativeExpectedKind::NoStream,
            version: None,
        },
        Expected::Exact(n) => NativeExpected {
            kind: NativeExpectedKind::Exact,
            version: Some(n),
        },
        Expected::Merge(_) => {
            return Err(StoreError::Document("preparation-merge-expectation".into()))
        }
        _ => {
            return Err(StoreError::Document(
                "preparation-unknown-expectation".into(),
            ))
        }
    })
}
impl NativeCommandMeta {
    fn capture(meta: &CommandMeta) -> Result<Self, StoreError> {
        let CommandMeta {
            idempotency_key,
            request_hash,
            subject,
            actor,
            request_id,
            trace_id,
            causation_id,
            causation_depth,
            occurred_at,
            claim,
        } = meta;
        require(claim.is_none(), "preparation-native-claim")?;
        Ok(Self {
            idempotency_key: idempotency_key.clone(),
            request_hash: request_hash.clone(),
            subject: subject.clone(),
            actor: actor.clone(),
            request_id: request_id.clone(),
            trace_id: trace_id.clone(),
            causation_id: causation_id.clone(),
            causation_depth: *causation_depth,
            occurred_at_unix_nanos: occurred_at.unix_timestamp_nanos().to_string(),
            occurred_at_offset_seconds: occurred_at.offset().whole_seconds(),
            claim: None,
        })
    }
    fn restore(&self) -> Result<CommandMeta, StoreError> {
        require(self.claim.is_none(), "preparation-native-claim")?;
        let nanos = self
            .occurred_at_unix_nanos
            .parse::<i128>()
            .map_err(|_| StoreError::Document("preparation-time".into()))?;
        require(
            nanos.to_string() == self.occurred_at_unix_nanos,
            "preparation-time-spelling",
        )?;
        let offset = time::UtcOffset::from_whole_seconds(self.occurred_at_offset_seconds)
            .map_err(|_| StoreError::Document("preparation-offset".into()))?;
        let occurred_at = OffsetDateTime::from_unix_timestamp_nanos(nanos)
            .map_err(|_| StoreError::Document("preparation-time".into()))?
            .to_offset(offset);
        Ok(CommandMeta {
            idempotency_key: self.idempotency_key.clone(),
            request_hash: self.request_hash.clone(),
            subject: self.subject.clone(),
            actor: self.actor.clone(),
            request_id: self.request_id.clone(),
            trace_id: self.trace_id.clone(),
            causation_id: self.causation_id.clone(),
            causation_depth: self.causation_depth,
            occurred_at,
            claim: None,
        })
    }
}
impl NativePublicationRequest {
    fn capture(request: &BlobAppendGroup) -> Result<Self, StoreError> {
        Ok(Self {
            tenant: request.group.tenant.to_string(),
            appends: request
                .group
                .appends
                .iter()
                .map(|append| {
                    Ok(NativeStreamAppend {
                        stream: NativeStreamId {
                            tenant: append.stream.tenant().to_string(),
                            stream_type: append.stream.stream_type().into(),
                            stream_id: append.stream.stream_id().into(),
                        },
                        expected: native_expected(append.expected)?,
                        events: append
                            .events
                            .iter()
                            .map(|event| {
                                Ok(NativeNewEvent {
                                    name: event.name.clone(),
                                    schema_version: event.schema_version,
                                    data: serde_json::to_vec(&event.data).map_err(json_error)?,
                                })
                            })
                            .collect::<Result<_, StoreError>>()?,
                    })
                })
                .collect::<Result<_, StoreError>>()?,
            meta: NativeCommandMeta::capture(&request.group.meta)?,
            blobs: request
                .blobs
                .iter()
                .map(|blob| NativeBlobWrite {
                    digest: blob.digest.clone(),
                    bytes: blob.bytes.clone(),
                })
                .collect(),
        })
    }
    fn restore(&self) -> Result<BlobAppendGroup, StoreError> {
        let tenant = TenantId::new(self.tenant.clone())?;
        let appends = self
            .appends
            .iter()
            .map(|append| {
                let expected = match (append.expected.kind, append.expected.version) {
                    (NativeExpectedKind::Any, None) => Expected::Any,
                    (NativeExpectedKind::NoStream, None) => Expected::NoStream,
                    (NativeExpectedKind::Exact, Some(version)) => Expected::Exact(version),
                    _ => return Err(StoreError::Document("preparation-expectation".into())),
                };
                let events = append
                    .events
                    .iter()
                    .map(|event| {
                        Ok(NewEvent::new(
                            &event.name,
                            event.schema_version,
                            strict_json(&event.data)?,
                        )?)
                    })
                    .collect::<Result<_, StoreError>>()?;
                Ok(StreamAppend {
                    stream: StreamId::new(
                        TenantId::new(append.stream.tenant.clone())?,
                        &append.stream.stream_type,
                        &append.stream.stream_id,
                    )?,
                    expected,
                    events,
                })
            })
            .collect::<Result<_, StoreError>>()?;
        Ok(BlobAppendGroup {
            group: AppendGroup {
                tenant,
                appends,
                meta: self.meta.restore()?,
            },
            blobs: self
                .blobs
                .iter()
                .map(|blob| BlobWrite {
                    digest: blob.digest.clone(),
                    bytes: blob.bytes.clone(),
                })
                .collect(),
        })
    }
}
// Serde's ordinary Value decoder replaces duplicate object keys. Recovery may not discard them.
struct Json(serde_json::Value);
impl<'de> Deserialize<'de> for Json {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        d.deserialize_any(JsonVisitor)
    }
}
struct JsonVisitor;
impl<'de> Visitor<'de> for JsonVisitor {
    type Value = Json;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("JSON with unique object keys")
    }
    fn visit_bool<E: de::Error>(self, v: bool) -> Result<Json, E> {
        Ok(Json(v.into()))
    }
    fn visit_i64<E: de::Error>(self, v: i64) -> Result<Json, E> {
        Ok(Json(v.into()))
    }
    fn visit_u64<E: de::Error>(self, v: u64) -> Result<Json, E> {
        Ok(Json(v.into()))
    }
    fn visit_f64<E: de::Error>(self, v: f64) -> Result<Json, E> {
        serde_json::Number::from_f64(v)
            .map(|n| Json(n.into()))
            .ok_or_else(|| E::custom("invalid JSON number"))
    }
    fn visit_str<E: de::Error>(self, v: &str) -> Result<Json, E> {
        Ok(Json(v.into()))
    }
    fn visit_unit<E: de::Error>(self) -> Result<Json, E> {
        Ok(Json(serde_json::Value::Null))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut values = Vec::new();
        while let Some(Json(value)) = seq.next_element()? {
            values.push(value)
        }
        Ok(Json(values.into()))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut values = serde_json::Map::new();
        while let Some((key, Json(value))) = map.next_entry::<String, Json>()? {
            if values.insert(key, value).is_some() {
                return Err(de::Error::custom("duplicate JSON key"));
            }
        }
        Ok(Json(values.into()))
    }
}
fn strict_json(bytes: &[u8]) -> Result<serde_json::Value, StoreError> {
    let Json(value) = serde_json::from_slice(bytes).map_err(json_error)?;
    require(value.is_object(), "preparation-native-data-is-not-object")?;
    Ok(value)
}

impl<S: AtomicBlobEventStore> EventlogStore<S> {
    fn preparation_stream(&self, key: &PublicationCommandKey) -> Result<StreamId, StoreError> {
        Ok(StreamId::new(
            self.tenant.clone(),
            "ekr.preparation",
            key.slot()?,
        )?)
    }
    fn authorize_preparation(
        &self,
        prepared: &PublicationPreparationV1,
    ) -> Result<BlobAppendGroup, StoreError> {
        prepared.command_key.check()?;
        require(
            prepared.format == PublicationPreparationV1::FORMAT,
            "preparation-format",
        )?;
        let request = prepared.native_request.restore()?;
        require(
            request.fingerprint()? == prepared.native_fingerprint,
            "preparation-fingerprint",
        )?;
        require(request.group.tenant == self.tenant, "preparation-tenant")?;
        let decision = &prepared.decision;
        require(
            decision.event.format == RevisionEvent::FORMAT && !decision.objects.is_empty(),
            "preparation-decision",
        )?;
        let tx = prepared.command_key.transaction_id;
        let matches = match (&prepared.command_key.kind, &decision.event.payload) {
            (PublicationCommandKind::Bootstrap, RevisionPayload::Seeded { .. }) => {
                decision.expected_version == 0
            }
            (
                PublicationCommandKind::Propose,
                RevisionPayload::TransactionProposed { transaction_id, .. },
            )
            | (
                PublicationCommandKind::Validate,
                RevisionPayload::TransactionValidated { transaction_id, .. },
            )
            | (
                PublicationCommandKind::Validate,
                RevisionPayload::TransactionRejected { transaction_id, .. },
            )
            | (
                PublicationCommandKind::Commit,
                RevisionPayload::RevisionCommitted { transaction_id, .. },
            )
            | (
                PublicationCommandKind::Commit,
                RevisionPayload::TransactionStale { transaction_id, .. },
            ) => tx == Some(*transaction_id),
            _ => false,
        };
        require(matches, "preparation-command-decision")?;
        let expected_meta = NativeCommandMeta::capture(&envelope(
            &format!(
                "ekr.occurrence.{}.{}",
                decision.event.event_id, prepared.attempt_number
            ),
            ContentHash::of(&decision.event).to_hex(),
        ))?;
        require(
            prepared.native_request.meta == expected_meta,
            "preparation-native-metadata",
        )?;
        let expected_blobs = decision
            .objects
            .iter()
            .map(|(hash, object)| NativeBlobWrite {
                digest: hash.to_hex(),
                bytes: object.bytes.clone(),
            })
            .collect::<Vec<_>>();
        require(
            prepared.native_request.blobs == expected_blobs,
            "preparation-blob-set",
        )?;
        for (hash, object) in &decision.objects {
            require(
                ContentHash::of_bytes(&object.bytes) == *hash,
                "preparation-object-address",
            )?;
        }
        let Some(first) = request.group.appends.first() else {
            return Err(StoreError::Document("preparation-empty-request".into()));
        };
        let expected_revision = if decision.expected_version == 0 {
            Expected::NoStream
        } else {
            Expected::Exact(decision.expected_version)
        };
        require(
            first.stream == self.revision_stream()?
                && native_expected(first.expected)? == native_expected(expected_revision)?
                && first.events.len() == 1,
            "preparation-revision-append",
        )?;
        let event = &first.events[0];
        require(
            event.name == decision.event.name()
                && event.schema_version == 2
                && event.data == serde_json::to_value(&decision.event).map_err(json_error)?,
            "preparation-revision-event",
        )?;
        let mut object_appends = BTreeSet::new();
        for append in request.group.appends.iter().skip(1) {
            require(
                append.stream.tenant() == &self.tenant
                    && append.stream.stream_type() == OBJECT_STREAM_TYPE
                    && append.events.len() == 1,
                "preparation-extra-stream",
            )?;
            let hash: ContentHash = append
                .stream
                .stream_id()
                .parse()
                .map_err(|_| StoreError::Document("preparation-object-stream".into()))?;
            let object = decision
                .objects
                .get(&hash)
                .ok_or_else(|| StoreError::Document("preparation-extra-object".into()))?;
            require(
                object_appends.insert(hash),
                "preparation-duplicate-object-append",
            )?;
            let event = &append.events[0];
            match append.expected {
                Expected::NoStream => {
                    let meta: ObjectMetadata =
                        serde_json::from_value(event.data.clone()).map_err(json_error)?;
                    require(
                        event.name == OBJECT_STORED
                            && event.schema_version == 2
                            && meta.content_hash == hash
                            && meta.storage_class == object.storage_class
                            && meta.byte_len == object.bytes.len() as u64
                            && meta.stored_at == object.stored_at,
                        "preparation-object-metadata",
                    )?;
                }
                Expected::Exact(version) if version > 0 => {
                    let raised: RetentionRaised =
                        serde_json::from_value(event.data.clone()).map_err(json_error)?;
                    require(
                        event.name == OBJECT_RETENTION_RAISED
                            && event.schema_version == 1
                            && raised.content_hash == hash
                            && raised.to == object.storage_class
                            && raised.to.retention_rank() > raised.from.retention_rank()
                            && self.retention_at(hash, version)? == raised.from,
                        "preparation-retention-metadata",
                    )?;
                }
                _ => {
                    return Err(StoreError::Document(
                        "preparation-object-expectation".into(),
                    ));
                }
            }
        }
        for (hash, object) in &decision.objects {
            if !object_appends.contains(hash) {
                let held = self.object(*hash)?.ok_or_else(|| {
                    StoreError::Document("preparation-object-metadata-missing".into())
                })?;
                require(
                    held.bytes == object.bytes
                        && held.metadata.storage_class.retention_rank()
                            >= object.storage_class.retention_rank(),
                    "preparation-unstated-object-binding",
                )?;
            }
        }
        // Admission uses the original prefix, never a later head as a substitute validation basis.
        let mut history = RetainedHistory::default();
        if decision.expected_version > 0 {
            history.occurrences = self.occurrences(MAX_READ_LIMIT, None)?;
            require(
                history.occurrences.len() as u64 >= decision.expected_version,
                "preparation-missing-basis-history",
            )?;
            history.occurrences.truncate(
                usize::try_from(decision.expected_version)
                    .map_err(|_| StoreError::Document("preparation-position-overflow".into()))?,
            );
        }
        // § 89, the lookup `publish` makes: an occurrence identity already retained in the basis
        // prefix with different content refuses before any native append. The elected decision's
        // own publication lies after its prefix, so a genuine record never meets itself here.
        // An identical event in the prefix is already retained; appending it again would record
        // one occurrence twice, so that refuses too, under its own name.
        if let Some(held) = history
            .occurrences
            .iter()
            .find(|held| held.event.event_id == decision.event.event_id)
        {
            return Err(StoreError::Document(if held.event == decision.event {
                "occurrence-already-retained".into()
            } else {
                "occurrence-identity-conflict".into()
            }));
        }
        if let Some(predecessor) = prepared.command_key.predecessor_event_id {
            require(
                history.occurrences.iter().any(|held| {
                    held.event.event_id == predecessor
                        && Some(held.event.record_hash)
                            == prepared.command_key.predecessor_record_hash
                        && match held.event.payload {
                            RevisionPayload::TransactionProposed { transaction_id, .. } => {
                                prepared.command_key.kind == PublicationCommandKind::Validate
                                    && tx == Some(transaction_id)
                            }
                            RevisionPayload::TransactionValidated { transaction_id, .. } => {
                                prepared.command_key.kind == PublicationCommandKind::Commit
                                    && tx == Some(transaction_id)
                            }
                            _ => false,
                        }
                }),
                "preparation-predecessor",
            )?;
        }
        let mut required = BTreeSet::new();
        for held in &history.occurrences {
            required.insert(held.event.record_hash);
            if let RevisionPayload::Seeded { seed_hash, .. } = held.event.payload {
                required.insert(seed_hash);
            }
        }
        self.load_objects(&mut history, required)?;
        if !history.occurrences.is_empty() {
            let required = self.authority()?.required_objects(&history)?;
            self.load_objects(&mut history, required)?;
        }
        self.authority()?
            .replay(&history, self.ontology.as_ref(), None)?;
        for (hash, object) in &decision.objects {
            history.objects.insert(
                *hash,
                RetainedObject {
                    metadata: StoredObject {
                        content_hash: *hash,
                        storage_class: object.storage_class,
                        byte_len: object.bytes.len() as u64,
                        stored_at: object.stored_at,
                    },
                    bytes: object.bytes.clone(),
                },
            );
        }
        history.occurrences.push(RecordedOccurrence {
            version: decision.expected_version + 1,
            provider_event_id: "prepared-domain-occurrence".into(),
            event: decision.event.clone(),
        });
        self.load_object(&mut history, decision.event.record_hash)?;
        let required = self.authority()?.required_objects(&history)?;
        self.load_objects(&mut history, required)?;
        self.authority()?
            .replay(&history, self.ontology.as_ref(), None)?;
        Ok(request)
    }
    fn retention_at(&self, hash: ContentHash, version: u64) -> Result<StorageClass, StoreError> {
        let events = self.read_until(&self.object_stream(hash)?, 1, |record| {
            record.version == version
        })?;
        require(
            events.len() as u64 == version,
            "preparation-object-prefix-missing",
        )?;
        let (first, later) = events
            .split_first()
            .ok_or_else(|| StoreError::Document("preparation-object-prefix-missing".into()))?;
        let original: ObjectMetadata =
            serde_json::from_value(first.data.clone()).map_err(json_error)?;
        require(
            first.name == OBJECT_STORED
                && first.schema_version == 2
                && original.content_hash == hash,
            "preparation-object-prefix",
        )?;
        let mut class = original.storage_class;
        for event in later {
            let raised: RetentionRaised =
                serde_json::from_value(event.data.clone()).map_err(json_error)?;
            require(
                event.name == OBJECT_RETENTION_RAISED
                    && event.schema_version == 1
                    && raised.content_hash == hash
                    && raised.from == class
                    && raised.to.retention_rank() > class.retention_rank(),
                "preparation-object-prefix",
            )?;
            class = raised.to;
        }
        Ok(class)
    }
    pub(super) fn read_preparation(
        &self,
        key: &PublicationCommandKey,
    ) -> Result<Option<PublicationPreparationV1>, StoreError> {
        ensure_sync_context()?;
        let events = self.read_all(&self.preparation_stream(key)?, MAX_READ_LIMIT)?;
        let mut previous = None;
        let mut selected = None;
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
            let prepared: PublicationPreparationV1 =
                serde_json::from_slice(&bytes).map_err(json_error)?;
            require(
                prepared.command_key == *key
                    && prepared.attempt_number == selection.attempt_number
                    && prepared.previous_attempt_hash == previous,
                "preparation-record-chain",
            )?;
            if let Some(prior) = &selected {
                check_successor(prior, &prepared)?;
            }
            self.authorize_preparation(&prepared)?;
            previous = Some(selection.preparation_hash);
            selected = Some(prepared);
        }
        Ok(selected)
    }
    fn native_for(
        &self,
        decision: &Publication,
        attempt: u64,
    ) -> Result<BlobAppendGroup, StoreError> {
        let mut appends = vec![StreamAppend {
            stream: self.revision_stream()?,
            expected: if decision.expected_version == 0 {
                Expected::NoStream
            } else {
                Expected::Exact(decision.expected_version)
            },
            events: vec![NewEvent::new(
                decision.event.name(),
                2,
                serde_json::to_value(&decision.event).map_err(json_error)?,
            )?],
        }];
        for (hash, object) in &decision.objects {
            if let Some(append) = self.object_append(*hash, object)? {
                appends.push(append)
            }
        }
        Ok(BlobAppendGroup {
            group: AppendGroup {
                tenant: self.tenant.clone(),
                appends,
                meta: envelope(
                    &format!("ekr.occurrence.{}.{attempt}", decision.event.event_id),
                    ContentHash::of(&decision.event).to_hex(),
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
    pub(super) fn elect_preparation(
        &self,
        key: &PublicationCommandKey,
        input_hash: ContentHash,
        decision: &Publication,
        previous: Option<&PublicationPreparationV1>,
    ) -> Result<PublicationPreparationV1, StoreError> {
        ensure_sync_context()?;
        let held = self.read_preparation(key)?;
        if let Some(held) = held.as_ref() {
            if held.input_hash != input_hash {
                return Err(StoreError::PublicationInputConflict);
            }
            if previous.is_none() || previous.is_some_and(|prior| prior != held) {
                return Ok(held.clone());
            }
        }
        if let Some(prior) = previous {
            require(
                held.as_ref() == Some(prior),
                "preparation-predecessor-not-elected",
            )?;
            match self.resume_preparation(prior) {
                Err(StoreError::Conflict) => {}
                Err(error) => return Err(error),
                Ok(_) => return Ok(prior.clone()),
            }
        }
        let attempt_number = previous.map_or(Ok(0), |prior| {
            prior
                .attempt_number
                .checked_add(1)
                .ok_or_else(|| StoreError::Document("preparation-attempt-overflow".into()))
        })?;
        let request = self.native_for(decision, attempt_number)?;
        let prepared = PublicationPreparationV1 {
            format: PublicationPreparationV1::FORMAT.into(),
            command_key: key.clone(),
            input_hash,
            decision: decision.clone(),
            attempt_number,
            previous_attempt_hash: previous.map(PublicationPreparationV1::hash).transpose()?,
            native_fingerprint: request.fingerprint()?,
            native_request: NativePublicationRequest::capture(&request)?,
        };
        if let Some(prior) = previous {
            check_successor(prior, &prepared)?;
        }
        self.authorize_preparation(&prepared)?;
        let bytes = serde_json::to_vec(&prepared).map_err(json_error)?;
        let hash = ContentHash::of_bytes(&bytes);
        let selection = Selection {
            preparation_hash: hash,
            attempt_number,
            previous_attempt_hash: prepared.previous_attempt_hash,
        };
        let native = BlobAppendGroup {
            group: AppendGroup {
                tenant: self.tenant.clone(),
                appends: vec![StreamAppend {
                    stream: self.preparation_stream(key)?,
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
                match self.read_preparation(key)? {
                    Some(winner) if winner.input_hash == input_hash => Ok(winner),
                    Some(_) => Err(StoreError::PublicationInputConflict),
                    None => Err(error),
                }
            }
            Err(error) => Err(error),
        }
    }
    pub(super) fn resume_preparation(
        &self,
        prepared: &PublicationPreparationV1,
    ) -> Result<Appended, StoreError> {
        ensure_sync_context()?;
        let elected = self
            .read_preparation(&prepared.command_key)?
            .ok_or_else(|| StoreError::Document("preparation-not-elected".into()))?;
        if elected != *prepared {
            return Err(StoreError::Conflict);
        }
        let request = self.authorize_preparation(prepared)?;
        let result = self.atomic(&request)?;
        Ok(if result.deduplicated {
            Appended::AlreadyRecorded
        } else {
            Appended::Written
        })
    }
}
fn check_successor(
    prior: &PublicationPreparationV1,
    next: &PublicationPreparationV1,
) -> Result<(), StoreError> {
    require(
        prior.command_key == next.command_key
            && prior.input_hash == next.input_hash
            && next.attempt_number == prior.attempt_number + 1
            && next.previous_attempt_hash == Some(prior.hash()?),
        "preparation-successor-chain",
    )?;
    let a = &prior.decision;
    let b = &next.decision;
    if a.event == b.event && a.objects == b.objects {
        return Ok(());
    }
    let stale = matches!((&a.event.payload,&b.event.payload),(RevisionPayload::RevisionCommitted {transaction_id:first,..},RevisionPayload::TransactionStale {transaction_id:second,..}) if first==second)
        && a.event.event_id != b.event.event_id;
    let original_time = a.objects.values().next().map(|object| object.stored_at);
    require(
        stale
            && b.objects
                .values()
                .all(|object| Some(object.stored_at) == original_time),
        "preparation-decision-changed",
    )
}
