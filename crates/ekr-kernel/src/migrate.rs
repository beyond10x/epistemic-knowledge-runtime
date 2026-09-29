//! The preserving store migration (design §§ 89, 90 and 100.3): a store's complete retained
//! history, re-published into a new, empty store under `ekr-seed-envelope/3`.
//!
//! The source is only read. Its inventory is replayed in full through this kernel's authority
//! before anything is written, so a source that does not verify refuses unchanged. The
//! destination receives the seed's exact input, context, authority, identities and time under
//! the `/3` envelope, then every later occurrence with its original event identity, revision
//! identity, actors and times: a proposal record byte for byte, and a record that names the seed
//! envelope, a prior root or a prior record derived again for the destination's lineage by the
//! same functions replay checks it with. Each is published through the destination's kernel
//! authority. Every other object the source holds is carried with its class, retention raises and
//! `stored_at`, a legacy `ObjectStored`/schema-1 one as schema-2 metadata and a blob. The
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
    /// The destination's `ekr-seed-envelope/3`.
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

impl<S: RevisionLog + ObjectStore + Inventory> Commit<S> {
    /// Migrates this store into `destination`, a store that holds nothing yet, opened under the
    /// same host anchor. This store is only read.
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
        let held = destination.store.inventory()?;
        if held.events != 0 {
            return Err(migration(
                "migrate-destination-not-empty",
                format!(
                    "the destination holds {} events; a migration writes only into a store \
                     that holds nothing",
                    held.events
                ),
            )
            .into());
        }
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
            return Err(migration(
                "migrate-unresolved-preparation",
                format!(
                    "occurrence {pending} was elected and never published; resolve it with the \
                     command that elected it before migrating"
                ),
            )
            .into());
        }
        let history = RetainedHistory {
            occurrences: inventory.occurrences.clone(),
            objects: inventory
                .objects
                .iter()
                .map(|(hash, held)| (*hash, held.object.clone()))
                .collect(),
        };
        let source = self
            .authority
            .reconstruct_in_full(&history)?
            .ok_or(CommitError::NotSeeded)?;

        let mut occurrences = Vec::with_capacity(history.occurrences.len());
        let mut replaced = BTreeMap::new();
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
                let (publication, envelope_hash) =
                    self.migrated_seed(&history, &inventory, occurrence, seed_hash, stored_at)?;
                if envelope_hash != seed_hash {
                    replaced.insert(seed_hash, envelope_hash);
                }
                publication
            } else {
                let state = destination
                    .authority
                    .reconstruct(&destination.store.history()?, None, None)?
                    .ok_or(CommitError::NotSeeded)?;
                let bytes = history.content(event.record_hash, StorageClass::Canonical)?;
                let (event, bytes) = destination.migrated_decision(&state, event, bytes)?;
                Publication {
                    objects: BTreeMap::from([(event.record_hash, record(bytes))]),
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
            let _published = if position == 0 {
                destination.store.initialize(&publication)?
            } else {
                destination.store.publish(&publication)?
            };
        }

        // Every other object: carried with its class, retention raises and stored_at.
        let present = destination.store.inventory()?.objects;
        let mut carried_objects = Vec::new();
        let mut legacy_objects = Vec::new();
        for (hash, held) in &inventory.objects {
            if held.legacy {
                legacy_objects.push(*hash);
            }
            if replaced.contains_key(hash) {
                continue;
            }
            let at = held.object.metadata.stored_at;
            let bytes = &held.object.bytes;
            match present.get(hash) {
                Some(there)
                    if there.object.metadata.storage_class.retention_rank()
                        >= held.object.metadata.storage_class.retention_rank() => {}
                Some(_) => {
                    destination
                        .store
                        .put(held.object.metadata.storage_class, bytes, at)?;
                }
                None => {
                    destination.store.put(held.stored_as, bytes, at)?;
                    for class in &held.raised_to {
                        destination.store.put(*class, bytes, at)?;
                    }
                    carried_objects.push(*hash);
                }
            }
        }

        // The destination, replayed in full from its seed, reaches the source's state.
        let migrated = destination
            .authority
            .reconstruct_in_full(&destination.store.history()?)?
            .ok_or(CommitError::NotSeeded)?;
        agrees(&source, &migrated)?;
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
    /// under `ekr-seed-envelope/3`, its payloads as Provenance objects with their source
    /// `stored_at`, and its result record for that envelope, with the source's identities.
    fn migrated_seed(
        &self,
        history: &RetainedHistory,
        inventory: &ekr_store::StoreInventory,
        occurrence: &ekr_store::RecordedOccurrence,
        seed_hash: ContentHash,
        stored_at: ekr_core::Timestamp,
    ) -> Result<(Publication, ContentHash), CommitError> {
        let source = self.authority.seed_envelope(history, seed_hash)?;
        let payloads = source.payload_bytes(history)?;
        let envelope = SeedEnvelope {
            format: seed::ENVELOPE_FORMAT.into(),
            input: RetainedSeedInput {
                format: source.input.format.clone(),
                ontology: source.input.ontology.clone(),
                graph: source.input.graph.clone(),
                payloads: SeedPayloads::Named(payloads.keys().copied().collect()),
            },
            context: source.context,
            authority: source.authority.clone(),
            committed_at: source.committed_at,
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
    fn migrated_decision(
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
