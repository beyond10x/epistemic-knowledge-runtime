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
}
impl<S: AtomicBlobEventStore> Inventory for EventlogStore<S> {
    fn inventory(&self) -> Result<StoreInventory, StoreError> {
        EventlogStore::inventory(self)
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
    /// # Errors
    /// Runtime-context refusal, provider failure, a log that disagrees with itself, and any object
    /// or preparation record that does not verify, by name.
    pub fn inventory(&self) -> Result<StoreInventory, StoreError> {
        ensure_sync_context()?;
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

    fn inventoried(&self, hash: ContentHash) -> Result<InventoriedObject, StoreError> {
        let events = self.read_all(&self.object_stream(hash)?, MAX_READ_LIMIT)?;
        let first = events
            .first()
            .ok_or_else(|| StoreError::Document(format!("inventory-object-missing: {hash}")))?;
        let (CheckedObject(object), legacy) = match (first.name.as_str(), first.schema_version) {
            (OBJECT_STORED, 1) => {
                let inline: InlineObjectStored = serde_json::from_value(first.data.clone())
                    .map_err(|error| {
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
                let meta = stored_metadata(&events)?.ok_or_else(|| {
                    StoreError::Document(format!("inventory-object-missing: {hash}"))
                })?;
                let blob = self
                    .runtime()
                    .block_on(self.store.get_blob(&self.tenant, &hash.to_hex()))?;
                let (checked, _) = retained_object(hash, &events, meta, blob)?;
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

    /// The domain occurrence a preparation's newest attempt elected, read from its private record
    /// without authorizing it: a migration only asks whether that decision was published.
    fn prepared_occurrence(
        &self,
        selection: &serde_json::Value,
    ) -> Result<ekr_core::EventId, StoreError> {
        let unreadable = || StoreError::Document("inventory-preparation-unreadable".into());
        let hash: ContentHash = serde_json::from_value(selection["preparation_hash"].clone())
            .map_err(|_| unreadable())?;
        let bytes = self
            .runtime()
            .block_on(
                self.store
                    .get_blob(&self.tenant, &format!("ekr.private.preparation.{hash}")),
            )?
            .ok_or_else(unreadable)?;
        if ContentHash::of_bytes(&bytes) != hash {
            return Err(unreadable());
        }
        let record: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| unreadable())?;
        serde_json::from_value(record["decision"]["event"]["event_id"].clone())
            .map_err(|_| unreadable())
    }
}
