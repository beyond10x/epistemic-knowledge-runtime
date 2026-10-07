//! What a preserving migration reads of a store (design §§ 90, 100.3): read only, and the one
//! reader of the frozen `ObjectStored`/schema-1 inline body.
use super::*;

/// Everything a store holds that a preserving migration carries, read without changing anything.
///
/// Plain values only, as [`PublishedEvent`] is: holding one grants nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoreInventory {
    /// The whole revision stream, each occurrence checked as a history read checks it.
    pub occurrences: Vec<RecordedOccurrence>,
    /// Every object the log stored, by address, each verified against its address and length.
    pub objects: BTreeMap<ContentHash, InventoriedObject>,
    /// The domain occurrence identity of each publication preparation's newest attempt.
    pub prepared: Vec<ekr_core::EventId>,
    /// How many events the provider log holds, of every stream.
    pub events: usize,
}

/// One stored object as its stream records it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InventoriedObject {
    /// The verified bytes, with the object's strongest class and original `stored_at`.
    pub object: RetainedObject,
    /// The class the object was first stored with.
    pub stored_as: StorageClass,
    /// The class of each retention raise, in stream order.
    pub raised_to: Vec<StorageClass>,
    /// Whether the object is a legacy `ObjectStored`/schema-1 record with its bytes inline, which
    /// no current read accepts.
    pub legacy: bool,
}

/// The frozen `ObjectStored`/schema-1 body: the metadata with the bytes inline, as a JSON number
/// array. Read here only, for the preserving migration; never written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InlineObjectStored {
    content_hash: ContentHash,
    storage_class: StorageClass,
    byte_len: u64,
    stored_at: Timestamp,
    bytes: Vec<u8>,
}

/// A store a preserving migration can read in full: [`EventlogStore::inventory`].
pub trait Inventory {
    /// Everything the store holds that a preserving migration carries, read without changing it.
    /// # Errors
    /// Provider failure or an object, occurrence or preparation that does not verify.
    fn inventory(&self) -> Result<StoreInventory, StoreError>;

    /// Whether the tenant holds no event or object. Providers whose change feeds lag committed
    /// history must override this with an authoritative observation. A concurrent publisher can
    /// still win after this check; initialization must claim its stream atomically.
    /// # Errors
    /// Provider failure or invalid retained material.
    fn is_empty(&self) -> Result<bool, StoreError> {
        let held = self.inventory()?;
        Ok(held.events == 0 && held.objects.is_empty() && held.occurrences.is_empty())
    }
}
impl<S: AtomicBlobEventStore> Inventory for EventlogStore<S> {
    fn inventory(&self) -> Result<StoreInventory, StoreError> {
        EventlogStore::inventory(self)
    }

    fn is_empty(&self) -> Result<bool, StoreError> {
        self.entered()?;
        if let Some(empty) = self.empty {
            if self.hosted_read_only {
                return Err(StoreError::ReadOnly(
                    "tenant emptiness requires capture metadata".into(),
                ));
            }
            return empty(&self.store, self.runtime(), &self.tenant);
        }
        let held = self.inventory()?;
        Ok(held.events == 0 && held.objects.is_empty() && held.occurrences.is_empty())
    }
}

impl<S: AtomicBlobEventStore> EventlogStore<S> {
    /// The store's inventory: every occurrence, every stored object and every preparation's
    /// newest decision. Writes nothing, and asks no authority: the kernel that migrates the store
    /// replays what this returns.
    ///
    /// A current object is read and checked as every read checks one. A legacy
    /// `ObjectStored`/schema-1 object is read under its frozen shape, its inline bytes checked
    /// against its `content_hash` and `byte_len`, and its later retention raises checked as a
    /// current object's are.
    ///
    /// A PostgreSQL store's inventory is one provider capture of the tenant (design § 107.2):
    /// its change feed can withhold committed events, and separate reads of the feed and of each
    /// stream would be several moments. Every check below is applied to the captured events and
    /// bound content as it is to what the feed and the streams return, and nothing else is read.
    ///
    /// # Errors
    /// Runtime-context refusal, provider failure, a log that disagrees with itself, and any object
    /// or preparation record that does not verify, by name. A PostgreSQL capture's own refusal is
    /// `postgres-capture: <reason>`; a reading handle whose tenant holds no capture metadata
    /// refuses as read-only.
    pub fn inventory(&self) -> Result<StoreInventory, StoreError> {
        self.entered()?;
        if let Some(capture) = self.capture {
            let captured = capture(
                &self.store,
                self.runtime(),
                &self.tenant,
                self.hosted_read_only,
            )?;
            return self.captured_inventory(captured);
        }
        let events = self.published_events()?;
        let occurrences = self.occurrences(MAX_READ_LIMIT, None)?;
        let mut objects = BTreeMap::new();
        let mut newest = BTreeMap::new();
        for event in &events {
            if event.stream_type == OBJECT_STREAM_TYPE && event.version == 1 {
                let hash: ContentHash = event.stream_id.parse().map_err(|_| {
                    StoreError::Document(format!("inventory-object-stream: {}", event.stream_id))
                })?;
                objects.insert(hash, self.inventoried(hash)?);
            }
            if event.name == "ekr.store.PublicationPrepared" {
                newest.insert(event.stream_id.clone(), event.data.clone());
            }
        }
        let mut prepared = Vec::new();
        for selection in newest.into_values() {
            prepared.push(self.prepared_occurrence(&selection)?);
        }
        Ok(StoreInventory {
            occurrences,
            objects,
            prepared,
            events: events.len(),
        })
    }

    /// The inventory of one capture of this store's tenant: its revision stream, every object its
    /// log stored and each preparation's newest decision, each checked as [`Self::inventory`]
    /// checks what it reads from the feed and the streams.
    fn captured_inventory(&self, captured: TenantCapture) -> Result<StoreInventory, StoreError> {
        let TenantCapture {
            tenant,
            events,
            blobs,
            ..
        } = captured;
        if tenant != self.tenant {
            return Err(StoreError::Document("capture-envelope-disagrees".into()));
        }
        let mut blobs: BTreeMap<String, Vec<u8>> = blobs
            .into_iter()
            .map(|blob| (blob.digest, blob.bytes))
            .collect();
        let total = events.len();
        let mut streams: BTreeMap<(String, String), Vec<RecordedEvent>> = BTreeMap::new();
        let mut stored = Vec::new();
        let mut newest = BTreeMap::new();
        for event in events {
            if event.is_redacted() || event.tenant != self.tenant {
                return Err(StoreError::Document("feed-envelope-disagrees".into()));
            }
            if event.stream_type == OBJECT_STREAM_TYPE && event.version == 1 {
                let hash: ContentHash = event.stream_id.parse().map_err(|_| {
                    StoreError::Document(format!("inventory-object-stream: {}", event.stream_id))
                })?;
                stored.push(hash);
            }
            if event.name == "ekr.store.PublicationPrepared" {
                newest.insert(event.stream_id.clone(), event.data.clone());
            }
            streams
                .entry((event.stream_type.clone(), event.stream_id.clone()))
                .or_default()
                .push(event);
        }
        // Each stream as one read of it would return it: every event the next version, this
        // tenant's and this stream's, with a provider identity not seen before.
        let mut stream = |id: StreamId| -> Result<Vec<RecordedEvent>, StoreError> {
            let events = streams
                .remove(&(id.stream_type().to_owned(), id.stream_id().to_owned()))
                .unwrap_or_default();
            let mut read = StreamRead::new(&id);
            read.absorb(
                &self.tenant,
                StreamSlice {
                    next_version: events.len() as u64 + 1,
                    events,
                    end_of_stream: true,
                },
                &|_| false,
            )?;
            Ok(read.events)
        };
        let mut held = HeldRevisions::default();
        for recorded in stream(self.revision_stream()?)? {
            held.accept(recorded)?;
        }
        let mut objects = BTreeMap::new();
        for hash in stored {
            let events = stream(self.object_stream(hash)?)?;
            let object = inventoried(hash, events, || Ok(blobs.remove(&hash.to_hex())))?;
            objects.insert(hash, object);
        }
        let mut prepared = Vec::new();
        for selection in newest.into_values() {
            prepared.push(prepared_occurrence(&selection, |key| {
                Ok(blobs.get(key).cloned())
            })?);
        }
        Ok(StoreInventory {
            occurrences: held.occurrences,
            objects,
            prepared,
            events: total,
        })
    }

    fn inventoried(&self, hash: ContentHash) -> Result<InventoriedObject, StoreError> {
        let events = self.read_all(&self.object_stream(hash)?, MAX_READ_LIMIT)?;
        inventoried(hash, events, || {
            Ok(self
                .runtime()
                .block_on(self.store.get_blob(&self.tenant, &hash.to_hex()))?)
        })
    }

    /// The domain occurrence a preparation's newest attempt elected, read from its private record
    /// without authorizing it: a migration only asks whether that decision was published.
    fn prepared_occurrence(
        &self,
        selection: &serde_json::Value,
    ) -> Result<ekr_core::EventId, StoreError> {
        prepared_occurrence(selection, |key| {
            Ok(self
                .runtime()
                .block_on(self.store.get_blob(&self.tenant, key))?)
        })
    }
}

/// One stored object from its whole stream, `events`, and `blob`, which reads the blob bound at
/// its address however the store was read: under its frozen inline shape for a legacy
/// `ObjectStored`/schema-1 record, which holds its bytes and reads no blob, and as every read
/// checks one otherwise.
fn inventoried(
    hash: ContentHash,
    events: Vec<RecordedEvent>,
    blob: impl FnOnce() -> Result<Option<Vec<u8>>, StoreError>,
) -> Result<InventoriedObject, StoreError> {
    let first = events
        .first()
        .ok_or_else(|| StoreError::Document(format!("inventory-object-missing: {hash}")))?;
    let (CheckedObject(object), legacy) = match (first.name.as_str(), first.schema_version) {
        (OBJECT_STORED, 1) => {
            let inline: InlineObjectStored =
                serde_json::from_value(first.data.clone()).map_err(|error| {
                    StoreError::Document(format!("inventory-legacy-object: {hash}: {error}"))
                })?;
            let meta = ObjectMetadata {
                content_hash: inline.content_hash,
                storage_class: inline.storage_class,
                byte_len: inline.byte_len,
                stored_at: inline.stored_at,
            };
            let (checked, _) = checked_object(hash, &events, meta, inline.bytes, |bytes| {
                ContentHash::of_bytes(bytes) == hash
            })?;
            (checked, true)
        }
        _ => {
            let meta = stored_metadata(&events)?
                .ok_or_else(|| StoreError::Document(format!("inventory-object-missing: {hash}")))?;
            let (checked, _) = retained_object(hash, &events, meta, blob()?)?;
            (checked, false)
        }
    };
    let stored_as =
        serde_json::from_value(first.data["storage_class"].clone()).map_err(json_error)?;
    let raised_to = events
        .iter()
        .skip(1)
        .map(|event| {
            serde_json::from_value::<RetentionRaised>(event.data.clone())
                .map(|raised| raised.to)
                .map_err(json_error)
        })
        .collect::<Result<_, _>>()?;
    Ok(InventoriedObject {
        object,
        stored_as,
        raised_to,
        legacy,
    })
}

/// The domain occurrence a preparation's newest attempt, `selection`, elected, read from its
/// private record through `blob`, which reads the blob bound under a key, without authorizing
/// it: a migration only asks whether that decision was published.
fn prepared_occurrence(
    selection: &serde_json::Value,
    blob: impl FnOnce(&str) -> Result<Option<Vec<u8>>, StoreError>,
) -> Result<ekr_core::EventId, StoreError> {
    let unreadable = || StoreError::Document("inventory-preparation-unreadable".into());
    let hash: ContentHash =
        serde_json::from_value(selection["preparation_hash"].clone()).map_err(|_| unreadable())?;
    let bytes = blob(&format!("ekr.private.preparation.{hash}"))?.ok_or_else(unreadable)?;
    if ContentHash::of_bytes(&bytes) != hash {
        return Err(unreadable());
    }
    let record: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| unreadable())?;
    serde_json::from_value(record["decision"]["event"]["event_id"].clone())
        .map_err(|_| unreadable())
}
