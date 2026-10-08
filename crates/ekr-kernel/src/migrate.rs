//! The preserving store migration (design §§ 89, 90 and 100.3): a store's complete retained
//! history, re-published into a new, empty store under `ekr-seed-envelope/4`.
//!
//! The source is only read. Its inventory is replayed in full through this kernel's authority
//! before anything is written, so a source that does not verify refuses unchanged. The
//! destination receives the seed's exact input, context, authority, identities and time under
//! the `/4` envelope, then every later occurrence with its original event identity, revision
//! identity, actors and times: a proposal record byte for byte, and a record that names the seed
//! envelope, a prior root or a prior record derived again for the destination's lineage by the
//! same functions replay checks it with. Each is published through the destination's kernel
//! authority; a commit goes out with the payload of each `AddEvidence` it holds, at the source's
//! `stored_at` and first class, so those payloads are not carried objects. Every other object the
//! source holds is carried with its class, retention raises and `stored_at`, a legacy
//! `ObjectStored`/schema-1 one as schema-2 metadata and a blob. The
//! destination is then replayed in full and compared with the source; the report says which
//! record replaced which, and is retained in the destination.
use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{ContentHash, EventId};
use ekr_graph::{RevisionEvent, RevisionPayload};
use ekr_store::{
    Initialize, Inventory, ObjectStore, Publication, PublicationObject, RetainedHistory,
    RevisionLog, StorageClass, StoreError,
};
use serde::Serialize;

use crate::replay::{basis, refuse, validate, validation_record, ReplayState};
use crate::seed::{self, RetainedSeedInput, SeedEnvelope, SeedPayloads};
use crate::{
    Commit, CommitError, CommitReceiptV1, ProposalRecordV1, RejectionRecordV1, SeedResultV1,
    StaleRecordV1, ValidationReceiptV1,
};

/// One occurrence as the migration re-published it: its identity and event name, and the record
/// it names in the source and in the destination — the same address where the record was carried
/// byte for byte.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct MigratedOccurrence {
    /// The occurrence identity, the same in both stores.
    pub event_id: EventId,
    /// The event name, e.g. `ekr.kernel.RevisionCommitted`.
    pub event: String,
    /// The record the source occurrence names.
    pub source_record_hash: ContentHash,
    /// The record the destination occurrence names.
    pub destination_record_hash: ContentHash,
}

/// What a preserving migration did, `ekr.store-migration/1` (design § 100.3).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct StoreMigrationV1 {
    /// Exactly `ekr.store-migration/1`.
    pub format: String,
    /// The source's seed envelope.
    pub source_seed_hash: ContentHash,
    /// The destination's `ekr-seed-envelope/4`.
    pub destination_seed_hash: ContentHash,
    /// Every occurrence, in stream order.
    pub occurrences: Vec<MigratedOccurrence>,
    /// Every object carried beside the history: held by the source, named by no re-published
    /// occurrence, and not replaced by one.
    pub carried_objects: Vec<ContentHash>,
    /// The objects the source held as legacy `ObjectStored`/schema-1 inline records, which the
    /// destination holds as schema-2 metadata and a blob.
    pub legacy_objects: Vec<ContentHash>,
    /// The address of this report, less this field, retained in the destination as a Canonical
    /// object.
    pub map_hash: ContentHash,
}

/// The retained form of [`StoreMigrationV1`]: every field but its own address.
#[derive(Serialize)]
struct RetainedMap<'a> {
    format: &'a str,
    source_seed_hash: ContentHash,
    destination_seed_hash: ContentHash,
    occurrences: &'a [MigratedOccurrence],
    carried_objects: &'a [ContentHash],
    legacy_objects: &'a [ContentHash],
}

impl StoreMigrationV1 {
    /// The report format.
    pub const FORMAT: &'static str = "ekr.store-migration/1";
}

fn migration(code: &str, detail: impl std::fmt::Display) -> StoreError {
    StoreError::Document(format!("{code}: {detail}"))
}

/// The fixed legacy markers, retained for `/2` and `/3` reader compatibility only.
const MARKERS: [&[u8]; 2] = [
    br#"{"format":"ekr.migration-started/1"}"#,
    br#"{"format":"ekr.migration-finished/1"}"#,
];

/// The old fixed markers remain readable only for legacy envelopes. New migrations bind their
/// claim in the seed itself, so ordinary content can never impersonate start or completion.
fn legacy_markers() -> [ContentHash; 2] {
    MARKERS.map(ContentHash::of_bytes)
}

pub(crate) fn completion(claim: EventId, seed_hash: ContentHash) -> Result<Vec<u8>, StoreError> {
    #[derive(Serialize)]
    struct Completed {
        format: &'static str,
        migration: EventId,
        seed_hash: ContentHash,
    }
    serde_json::to_vec(&Completed {
        format: "ekr.migration-finished/2",
        migration: claim,
        seed_hash,
    })
    .map_err(|error| StoreError::Document(error.to_string()))
}

pub(crate) fn required_markers(
    authority: &crate::KernelAuthority,
    history: &RetainedHistory,
) -> Result<BTreeSet<ContentHash>, StoreError> {
    let Some(first) = history.occurrences.first() else {
        return Ok(BTreeSet::new());
    };
    let RevisionPayload::Seeded { seed_hash, .. } = first.event.payload else {
        return Err(StoreError::NotSeeded);
    };
    let envelope = authority.seed_envelope(history, seed_hash)?;
    if let Some(claim) = envelope.migration {
        return Ok(BTreeSet::from([ContentHash::of_bytes(&completion(
            claim, seed_hash,
        )?)]));
    }
    // Legacy control markers were Canonical objects. An evidence payload is never control,
    // even when a separate object writer has raised its retention to Canonical.
    let mut evidence = envelope.input.payload_keys();
    evidence.extend(authority.added_evidence_required(history, false)?);
    Ok(legacy_markers()
        .into_iter()
        .filter(|hash| !evidence.contains(hash))
        .collect())
}

/// Refuses an unfinished copy before any replay, including a cached or checkpointed replay.
pub(crate) fn finished(
    authority: &crate::KernelAuthority,
    history: &RetainedHistory,
) -> Result<(), StoreError> {
    if history.occurrences.is_empty() || authority.cache()?.migration_settled {
        return Ok(());
    }
    let wanted = required_markers(authority, history)?;
    let first = &history.occurrences[0];
    let RevisionPayload::Seeded { seed_hash, .. } = first.event.payload else {
        return Err(StoreError::NotSeeded);
    };
    let envelope = authority.seed_envelope(history, seed_hash)?;
    let canonical = |hash: &ContentHash| {
        history
            .objects
            .get(hash)
            .is_some_and(|held| held.metadata.storage_class == StorageClass::Canonical)
    };
    let unfinished = if envelope.migration.is_some() {
        wanted.iter().any(|hash| !canonical(hash))
    } else {
        let [started, finished] = legacy_markers();
        wanted.contains(&started)
            && canonical(&started)
            && !(wanted.contains(&finished) && canonical(&finished))
    };
    let mut cache = authority.cache()?;
    if unfinished {
        if cache.migrating {
            return Ok(());
        }
        return Err(migration(
            "migrate-incomplete",
            "a migration into this store began and did not finish; it is not the migrated store. \
             Remove it and migrate again",
        ));
    }
    cache.migration_settled = true;
    Ok(())
}

/// A store read once for a preserving copy (design §§ 105.2, 107.2): its inventory, taken in one
/// read of the provider — the image a read-only SQLite store holds, or one PostgreSQL capture —
/// and replayed in full through the kernel's authority.
///
/// Everything a copy publishes comes from this value; [`CapturedStore::copy_into`] reads nothing
/// more of the source. A commit to the source after the capture is therefore in no copy of it, and
/// the capture's [`head`](CapturedStore::head) is the head every copy of it holds: a stage's base.
/// Holding one grants nothing but copying.
pub struct CapturedStore<'a, S: RevisionLog + ObjectStore> {
    /// The store it was read from, whose authority decodes its seed envelope.
    source: &'a Commit<S>,
    inventory: ekr_store::StoreInventory,
    history: RetainedHistory,
    /// The inventory replayed in full: what every copy is compared with.
    state: std::sync::Arc<ReplayState>,
}

impl<S: RevisionLog + ObjectStore + Inventory> Commit<S> {
    /// Reads this store once for a preserving copy: its inventory ([`Inventory::inventory`], one
    /// provider capture on PostgreSQL), checked for an elected decision never published, and
    /// replayed in full. Writes nothing.
    ///
    /// # Errors
    /// `migrate-unresolved-preparation` for a preparation whose decision was never published; any
    /// refusal of the inventory or of the full replay, `migrate-incomplete` included;
    /// [`CommitError::NotSeeded`] for a store with no seed.
    pub fn capture(&self) -> Result<CapturedStore<'_, S>, CommitError> {
        self.capture_with(|pending| {
            migration(
                "migrate-unresolved-preparation",
                format!(
                    "occurrence {pending} was elected and never published; resolve it with the \
                     command that elected it before migrating"
                ),
            )
            .into()
        })
    }

    /// [`Self::capture`], refusing a preparation whose decision was never published with
    /// `unresolved`: a stage's begin, seal and publication name it
    /// [`StoreError::UnresolvedPreparation`].
    pub(crate) fn capture_with(
        &self,
        unresolved: impl FnOnce(EventId) -> CommitError,
    ) -> Result<CapturedStore<'_, S>, CommitError> {
        let inventory = self.store.inventory()?;
        let published: BTreeSet<EventId> = inventory
            .occurrences
            .iter()
            .map(|occurrence| occurrence.event.event_id)
            .collect();
        if let Some(pending) = inventory
            .prepared
            .iter()
            .find(|event_id| !published.contains(event_id))
        {
            return Err(unresolved(*pending));
        }
        let history = RetainedHistory {
            occurrences: inventory.occurrences.clone(),
            objects: inventory
                .objects
                .iter()
                .map(|(hash, held)| (*hash, held.object.clone()))
                .collect(),
        };
        let state = self
            .authority
            .reconstruct_in_full(&history)?
            .ok_or(CommitError::NotSeeded)?;
        Ok(CapturedStore {
            source: self,
            inventory,
            history,
            state,
        })
    }

    /// Migrates this store into `destination`, a store that holds nothing yet, opened under the
    /// same host anchor: [`Commit::capture`], then [`CapturedStore::copy_into`]. This store is
    /// only read.
    ///
    /// # Errors
    /// `migrate-destination-not-empty` for a destination that holds any event;
    /// `migrate-unresolved-preparation` for a source preparation whose decision was never
    /// published; any refusal of the source's full replay; `migrate-verification-disagrees` where
    /// the destination's full replay does not reach the source's state.
    pub fn migrate_into<D: RevisionLog + ObjectStore + Initialize + Inventory>(
        &self,
        destination: &Commit<D>,
    ) -> Result<StoreMigrationV1, CommitError> {
        empty(destination)?;
        self.capture()?.copy_unchecked(destination)
    }
}

/// Refuses a destination that holds anything.
fn empty<D: RevisionLog + ObjectStore + Inventory>(
    destination: &Commit<D>,
) -> Result<(), CommitError> {
    if destination.store.is_empty()? {
        Ok(())
    } else {
        Err(migration(
            "migrate-destination-not-empty",
            "a migration writes only into a store that holds nothing",
        )
        .into())
    }
}

impl<S: RevisionLog + ObjectStore + Inventory> CapturedStore<'_, S> {
    /// The captured head: the head every copy of this capture holds, whatever the source has
    /// committed since.
    #[must_use]
    pub fn head(&self) -> ekr_graph::Root {
        self.state.head().root
    }

    /// The identity of the captured head revision.
    #[must_use]
    pub fn head_revision(&self) -> ekr_core::RevisionId {
        self.state.head().revision_id
    }

    /// The captured inventory.
    pub(crate) const fn inventory(&self) -> &ekr_store::StoreInventory {
        &self.inventory
    }

    /// The captured history, every object of the inventory held.
    pub(crate) const fn history(&self) -> &RetainedHistory {
        &self.history
    }

    /// The inventory replayed in full.
    pub(crate) fn state(&self) -> &ReplayState {
        &self.state
    }

    /// Copies the captured store into `destination`, a store that holds nothing yet, opened under
    /// the same host anchor — a new store, or a stage's own tenant of the same store (design
    /// § 107.2) — and verifies the copy there. Reads nothing more of the source.
    ///
    /// The destination's seed binds a fresh claim under `ekr-seed-envelope/4` and its completion
    /// receipt is written last, so a copy interrupted at any point leaves a destination every
    /// other reader refuses as `migrate-incomplete`. A capture can be copied more than once, each
    /// copy under its own claim.
    ///
    /// # Errors
    /// `migrate-destination-not-empty`; any refusal of the destination's writes or replay;
    /// `migrate-verification-disagrees` where the destination's full replay does not reach the
    /// captured state.
    pub fn copy_into<D: RevisionLog + ObjectStore + Initialize + Inventory>(
        &self,
        destination: &Commit<D>,
    ) -> Result<StoreMigrationV1, CommitError> {
        empty(destination)?;
        self.copy_unchecked(destination)
    }

    /// [`Self::copy_into`] after its destination was found empty.
    fn copy_unchecked<D: RevisionLog + ObjectStore + Initialize + Inventory>(
        &self,
        destination: &Commit<D>,
    ) -> Result<StoreMigrationV1, CommitError> {
        // The seed atomically binds a fresh claim to this copy. Until the
        // completion receipt is written last, other readers refuse `migrate-incomplete`, so a
        // migration interrupted at any point leaves no store that answers as the migrated one.
        {
            let mut cache = destination.authority.cache()?;
            cache.migration_settled = false;
            cache.migrating = true;
        }
        let outcome = (|| {
            let claim = EventId::mint();
            let report = self.source.publish_into(
                destination,
                &self.history,
                &self.inventory,
                &self.state,
                claim,
            )?;
            let finished = completion(claim, report.destination_seed_hash)?;
            let _ = destination.store.put(
                StorageClass::Canonical,
                &finished,
                self.state.head().committed_at,
            )?;
            Ok(report)
        })();
        destination.authority.cache()?.migrating = false;
        outcome
    }
}

impl<S: RevisionLog + ObjectStore + Inventory> Commit<S> {
    /// Publishes the verified `source` history, `inventory` and `history` of this store into
    /// `destination`, verifies it there and retains the report.
    fn publish_into<D: RevisionLog + ObjectStore + Initialize + Inventory>(
        &self,
        destination: &Commit<D>,
        history: &RetainedHistory,
        inventory: &ekr_store::StoreInventory,
        source: &ReplayState,
        claim: EventId,
    ) -> Result<StoreMigrationV1, CommitError> {
        let mut occurrences = Vec::with_capacity(history.occurrences.len());
        let mut replaced = BTreeMap::new();
        let mut published_objects = BTreeSet::new();
        for (position, occurrence) in history.occurrences.iter().enumerate() {
            let event = &occurrence.event;
            let stored_at = history
                .objects
                .get(&event.record_hash)
                .map(|record| record.metadata.stored_at)
                .ok_or_else(|| migration("migrate-record-missing", event.record_hash))?;
            let record = |bytes: Vec<u8>| PublicationObject {
                storage_class: StorageClass::Canonical,
                stored_at,
                bytes,
            };
            let publication = if let RevisionPayload::Seeded { seed_hash, .. } = event.payload {
                let (publication, envelope_hash) = self
                    .migrated_seed(history, inventory, occurrence, seed_hash, stored_at, claim)?;
                if envelope_hash != seed_hash {
                    replaced.insert(seed_hash, envelope_hash);
                }
                publication
            } else {
                let held = destination.store.history()?;
                let state = destination
                    .authority
                    .reconstruct(&held, None, None)?
                    .ok_or(CommitError::NotSeeded)?;
                // A validation against an earlier revision is derived again against that
                // revision's graph, which the destination's state may no longer hold.
                let state = match event.payload {
                    RevisionPayload::TransactionValidated { against, .. } => {
                        destination.authority.holding(&held, state, against)?
                    }
                    _ => state,
                };
                let bytes = history.content(event.record_hash, StorageClass::Canonical)?;
                let (event, bytes) = destination.migrated_decision(&state, event, bytes)?;
                let mut objects = BTreeMap::from([(event.record_hash, record(bytes))]);
                // A commit published the payload of each `AddEvidence` it brought with its
                // receipt, at Provenance, and replay of the commit reads it there: it is published
                // with it here too, with the `stored_at` the source first stored it with. A payload
                // the source held below Provenance before the commit is stored first at that
                // class, so the commit raises it as it did in the source.
                if let RevisionPayload::RevisionCommitted { transaction_id, .. } = event.payload {
                    let tx = state
                        .transactions
                        .get(&transaction_id)
                        .ok_or(StoreError::ProposalMissing { transaction_id })?;
                    let document = state.document(&tx.proposal)?;
                    for hash in crate::commands::added_payloads(document.transaction()).into_keys()
                    {
                        let source = inventory
                            .objects
                            .get(&hash)
                            .ok_or_else(|| migration("migrate-payload-missing", hash))?;
                        let stored_at = source.object.metadata.stored_at;
                        let mut class = source.stored_as;
                        if class.retention_rank() < StorageClass::Provenance.retention_rank() {
                            if !held.objects.contains_key(&hash) {
                                let _ = destination.store.put(
                                    class,
                                    &source.object.bytes,
                                    stored_at,
                                )?;
                            }
                            class = StorageClass::Provenance;
                        }
                        objects.entry(hash).or_insert_with(|| PublicationObject {
                            storage_class: class,
                            stored_at,
                            bytes: source.object.bytes.to_vec(),
                        });
                    }
                }
                Publication {
                    objects,
                    event,
                    expected_version: position as u64,
                }
            };
            if publication.event.record_hash != event.record_hash {
                replaced.insert(event.record_hash, publication.event.record_hash);
            }
            occurrences.push(MigratedOccurrence {
                event_id: event.event_id,
                event: event.name().to_owned(),
                source_record_hash: event.record_hash,
                destination_record_hash: publication.event.record_hash,
            });
            published_objects.extend(publication.objects.keys().copied());
            let _published = if position == 0 {
                // The migration claim is part of the atomic seed.
                // A competing initializer cannot observe a seed without its claim, and a
                // losing migration cannot poison a seed that won first. Identical seed retries
                // are not migration ownership: only the handle that writes continues the copy.
                let initialized = destination.store.initialize(&publication);
                match initialized {
                    Ok(ekr_store::Appended::Written) => ekr_store::Appended::Written,
                    Err(error) if destination.store.is_empty()? => return Err(error.into()),
                    Ok(ekr_store::Appended::AlreadyRecorded) | Err(_) => {
                        return Err(migration(
                            "migrate-destination-not-empty",
                            "another initializer claimed the destination",
                        )
                        .into())
                    }
                }
            } else {
                destination.store.publish(&publication)?
            };
        }

        // Every other object: carried with its class, retention raises and stored_at.
        let mut carried_objects = Vec::new();
        let mut legacy_objects = Vec::new();
        for (hash, held) in &inventory.objects {
            if held.legacy {
                legacy_objects.push(*hash);
            }
            // Prior migration receipts and fixed legacy marker bytes are ordinary carried
            // objects: only the fresh claim bound in this destination seed controls admission.
            if replaced.contains_key(hash) {
                continue;
            }
            let at = held.object.metadata.stored_at;
            let bytes = &held.object.bytes;
            // Replaying each original retention level is idempotent when publication already
            // stored this object. It avoids a feed inventory whose watermark could hide it.
            destination.store.put(held.stored_as, bytes, at)?;
            for class in &held.raised_to {
                destination.store.put(*class, bytes, at)?;
            }
            if !published_objects.contains(hash) {
                carried_objects.push(*hash);
            }
        }

        // The destination, replayed in full from its seed, reaches the source's state.
        let migrated = destination
            .authority
            .reconstruct_in_full(&destination.store.history()?)?
            .ok_or(CommitError::NotSeeded)?;
        agrees(source, &migrated)?;
        destination.retain_checkpoint();

        let destination_seed_hash = migrated.seed.seed_hash;
        let map = RetainedMap {
            format: StoreMigrationV1::FORMAT,
            source_seed_hash: source.seed.seed_hash,
            destination_seed_hash,
            occurrences: &occurrences,
            carried_objects: &carried_objects,
            legacy_objects: &legacy_objects,
        };
        let map_bytes =
            serde_json::to_vec(&map).map_err(|error| StoreError::Document(error.to_string()))?;
        let map_hash = destination
            .store
            .put(
                StorageClass::Canonical,
                &map_bytes,
                migrated.head().committed_at,
            )?
            .content_hash;
        Ok(StoreMigrationV1 {
            format: StoreMigrationV1::FORMAT.into(),
            source_seed_hash: source.seed.seed_hash,
            destination_seed_hash,
            occurrences,
            carried_objects,
            legacy_objects,
            map_hash,
        })
    }

    /// The destination's seed publication: the source seed's input, context, authority and time
    /// under `ekr-seed-envelope/4`, its payloads as Provenance objects with their source
    /// `stored_at`, and its result record for that envelope, with the source's identities.
    fn migrated_seed(
        &self,
        history: &RetainedHistory,
        inventory: &ekr_store::StoreInventory,
        occurrence: &ekr_store::RecordedOccurrence,
        seed_hash: ContentHash,
        stored_at: ekr_core::Timestamp,
        claim: EventId,
    ) -> Result<(Publication, ContentHash), CommitError> {
        let source = self.authority.seed_envelope(history, seed_hash)?;
        let payloads = source.payload_bytes(history)?;
        let envelope = SeedEnvelope {
            format: seed::MIGRATION_ENVELOPE_FORMAT.into(),
            input: RetainedSeedInput {
                format: source.input.format.clone(),
                ontology: source.input.ontology.clone(),
                graph: source.input.graph.clone(),
                payloads: SeedPayloads::Named(payloads.keys().copied().collect()),
            },
            context: source.context,
            authority: source.authority.clone(),
            committed_at: source.committed_at,
            migration: Some(claim),
        };
        let envelope_bytes = envelope
            .to_bytes()
            .map_err(|error| StoreError::InvalidSeed(error.to_string()))?;
        let envelope_hash = ContentHash::of_bytes(&envelope_bytes);
        let original = SeedResultV1::from_bytes(
            history.content(occurrence.event.record_hash, StorageClass::Canonical)?,
        )?;
        let mut result = original.result;
        result.transaction = envelope_hash;
        let record = SeedResultV1 {
            seed_hash: envelope_hash,
            result,
            result_hash: ContentHash::of(&result),
            ..original
        };
        let record_bytes = record.to_bytes()?;
        let record_hash = ContentHash::of_bytes(&record_bytes);
        let envelope_at = inventory
            .objects
            .get(&seed_hash)
            .map_or(stored_at, |held| held.object.metadata.stored_at);
        let mut objects = BTreeMap::from([
            (
                envelope_hash,
                PublicationObject {
                    storage_class: StorageClass::Canonical,
                    stored_at: envelope_at,
                    bytes: envelope_bytes,
                },
            ),
            (
                record_hash,
                PublicationObject {
                    storage_class: StorageClass::Canonical,
                    stored_at,
                    bytes: record_bytes,
                },
            ),
        ]);
        for (hash, bytes) in payloads {
            let at = inventory
                .objects
                .get(&hash)
                .map_or(source.committed_at, |held| held.object.metadata.stored_at);
            objects.entry(hash).or_insert_with(|| PublicationObject {
                storage_class: StorageClass::Provenance,
                stored_at: at,
                bytes: bytes.to_vec(),
            });
        }
        let publication = Publication {
            event: RevisionEvent {
                format: RevisionEvent::FORMAT.into(),
                event_id: occurrence.event.event_id,
                record_hash,
                payload: RevisionPayload::Seeded {
                    revision_id: record.revision_id,
                    seed_hash: envelope_hash,
                },
            },
            objects,
            expected_version: 0,
        };
        Ok((publication, envelope_hash))
    }
}

impl<D: RevisionLog + ObjectStore> Commit<D> {
    /// The destination's form of one retained decision after `state`, the destination's replay of
    /// what precedes it: the same identities, actors and times; a proposal record byte for byte;
    /// and a record naming the lineage derived again against `state` by the functions replay
    /// checks it with.
    pub(crate) fn migrated_decision(
        &self,
        state: &ReplayState,
        event: &RevisionEvent,
        bytes: &[u8],
    ) -> Result<(RevisionEvent, Vec<u8>), CommitError> {
        let anchor = &self.authority.anchor;
        let validator = self.authority.context.validator;
        let seed_hash = state.seed.seed_hash;
        let record = |bytes: Vec<u8>, payload: RevisionPayload| {
            (
                RevisionEvent {
                    format: event.format.clone(),
                    event_id: event.event_id,
                    record_hash: ContentHash::of_bytes(&bytes),
                    payload,
                },
                bytes,
            )
        };
        let proposed = |transaction_id| {
            state
                .transactions
                .get(&transaction_id)
                .ok_or(StoreError::ProposalMissing { transaction_id })
        };
        Ok(match event.payload {
            RevisionPayload::Seeded { .. } => return Err(StoreError::SeedIsNotFirst.into()),
            RevisionPayload::TransactionProposed { .. } => {
                ProposalRecordV1::from_bytes(bytes)?;
                (event.clone(), bytes.to_vec())
            }
            RevisionPayload::TransactionValidated {
                transaction_id,
                against,
                ..
            } => {
                let original = ValidationReceiptV1::from_bytes(bytes)?;
                let tx = proposed(transaction_id)?;
                let prior = state
                    .revisions
                    .get(&against)
                    .ok_or_else(|| refuse("validation-basis-absent"))?;
                let validated = validate(
                    &*state.document(&tx.proposal)?,
                    &state.revisions,
                    &state.held,
                    prior,
                    anchor,
                    validator,
                )?
                .map_err(|_| refuse("retained-validation-refused"))?;
                let receipt = validation_record(
                    &tx.proposal,
                    tx.proposal_record_hash,
                    &validated,
                    basis(prior, seed_hash, anchor),
                    validator,
                    event.event_id,
                    original.validated_at,
                );
                let validation_hash = receipt.validation_hash;
                record(
                    receipt.to_bytes()?,
                    RevisionPayload::TransactionValidated {
                        transaction_id,
                        against,
                        validation_hash,
                    },
                )
            }
            RevisionPayload::TransactionRejected { .. } => {
                let original = RejectionRecordV1::from_bytes(bytes)?;
                let prior = state
                    .revisions
                    .get(&original.requested_basis.previous_root.revision)
                    .ok_or_else(|| refuse("rejection-basis-absent"))?;
                let rejection = RejectionRecordV1 {
                    requested_basis: basis(prior, seed_hash, anchor),
                    ..original
                };
                record(rejection.to_bytes()?, event.payload.clone())
            }
            RevisionPayload::RevisionCommitted { transaction_id, .. } => {
                let original = CommitReceiptV1::from_bytes(bytes)?;
                let tx = proposed(transaction_id)?;
                let validation = tx
                    .validation
                    .clone()
                    .ok_or(StoreError::ValidationMissing { transaction_id })?;
                let prior = state.head();
                let validated = match state.validated.get(&transaction_id) {
                    Some(validated) => std::sync::Arc::clone(validated),
                    None => std::sync::Arc::new(
                        validate(
                            &*state.document(&tx.proposal)?,
                            &state.revisions,
                            &state.held,
                            prior,
                            anchor,
                            validator,
                        )?
                        .map_err(|_| refuse("retained-commit-validation-refused"))?,
                    ),
                };
                let (_, root) = crate::apply::apply(
                    prior,
                    &validated,
                    &validation.validators,
                    original.committed_at,
                )?;
                let receipt = CommitReceiptV1 {
                    proposal: tx.proposal.clone(),
                    validation,
                    validation_record_hash: tx
                        .validation_record_hash
                        .ok_or(StoreError::ValidationMissing { transaction_id })?,
                    result: root,
                    result_hash: ContentHash::of(&root),
                    ..original
                };
                record(receipt.to_bytes()?, event.payload.clone())
            }
            RevisionPayload::TransactionStale { transaction_id, .. } => {
                let original = StaleRecordV1::from_bytes(bytes)?;
                let tx = proposed(transaction_id)?;
                let validation = tx
                    .validation
                    .as_ref()
                    .ok_or(StoreError::ValidationMissing { transaction_id })?;
                let observed = state
                    .revisions
                    .get(&original.observed_root.revision)
                    .ok_or_else(|| refuse("stale-observed-revision-absent"))?;
                let stale = StaleRecordV1 {
                    validation_record_hash: tx
                        .validation_record_hash
                        .ok_or(StoreError::ValidationMissing { transaction_id })?,
                    expected_basis: validation.basis.clone(),
                    observed_revision_id: observed.revision_id,
                    observed_event_id: observed.event_id,
                    observed_record_hash: observed.record_hash,
                    observed_root: observed.root,
                    observed_root_hash: ContentHash::of(&observed.root),
                    ..original
                };
                record(stale.to_bytes()?, event.payload.clone())
            }
        })
    }
}

/// Whether the destination's full replay reached the source's state: the same revisions with the
/// same identities, times and knowledge, evidence, ontology and authority roots, the same head
/// graph, and the same transactions in the same states.
fn agrees(source: &ReplayState, migrated: &ReplayState) -> Result<(), StoreError> {
    let disagrees = |what: &str| migration("migrate-verification-disagrees", what);
    if source.revisions.len() != migrated.revisions.len() {
        return Err(disagrees("revision count"));
    }
    for ((number, a), (other, b)) in source.revisions.iter().zip(&migrated.revisions) {
        if number != other
            || a.revision_id != b.revision_id
            || a.event_id != b.event_id
            || a.committed_at != b.committed_at
            || a.root.knowledge_root != b.root.knowledge_root
            || a.root.evidence_root != b.root.evidence_root
            || a.root.ontology_root != b.root.ontology_root
            || a.root.agent_root != b.root.agent_root
        {
            return Err(disagrees(&format!("revision {number}")));
        }
    }
    let (a, b) = (source.head().graph()?, migrated.head().graph()?);
    if a.nodes != b.nodes || a.edges != b.edges || a.assertions != b.assertions {
        return Err(disagrees("head graph"));
    }
    if a.evidence != b.evidence || source.seed_payloads != migrated.seed_payloads {
        return Err(disagrees("evidence"));
    }
    let states = |state: &ReplayState| -> Vec<_> {
        state
            .transactions
            .iter()
            .map(|(id, record)| (*id, record.state()))
            .collect()
    };
    if states(source) != states(migrated) {
        return Err(disagrees("transactions"));
    }
    Ok(())
}
