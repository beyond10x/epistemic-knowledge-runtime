//! Content-addressed objects and the class that decides how long one is kept.
//!
//! Design § 37 and § 57, projected from `ekr.store.StorageClass` and `ekr.store.StoredObject` of
//! `systems/ekr/domains/store.yaml`.

use ekr_core::{ContentHash, Timestamp};
use serde::{Deserialize, Serialize};

use crate::StoreError;

/// How long a payload is worth keeping: `ekr.store.StorageClass`, design § 37.
///
/// Different information deserves different durability, and the class is what a reclamation sweep
/// reads. Nothing here reclaims anything — design § 38–39's collector arrives in P6 with the
/// commands that move an object toward deletion — so the class is recorded and not yet acted on.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum StorageClass {
    /// Durable, revisioned, strongly governed; semantically retracted rather than deleted.
    Canonical,
    /// Retained while it is required to verify, audit or reproduce a canonical assertion.
    Provenance,
    /// Retained while it has meaningful integration or schema-discovery potential.
    Incubating,
    /// Reproducible, and freely deletable.
    Cache,
    /// Short-lived agent and runtime working state.
    Ephemeral,
}

impl StorageClass {
    /// Every class the domain declares, in the order it declares them.
    ///
    /// Held so that a case can walk the five rather than name them, and checked against
    /// `systems/ekr/domains/store.yaml` by `tests/domain_projection.rs`: a sixth class added to the
    /// domain and not to this array turns that case red, which is the only way a hand-kept list
    /// stays honest.
    pub const ALL: [Self; 5] = [
        Self::Canonical,
        Self::Provenance,
        Self::Incubating,
        Self::Cache,
        Self::Ephemeral,
    ];

    /// The variant's own name, as `systems/ekr/domains/store.yaml` spells it.
    ///
    /// The stored form of a class, and therefore part of the contract: a payload written under one
    /// spelling and read under another is a payload with no retention answer.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Canonical => "Canonical",
            Self::Provenance => "Provenance",
            Self::Incubating => "Incubating",
            Self::Cache => "Cache",
            Self::Ephemeral => "Ephemeral",
        }
    }

    /// How strongly this class requires the bytes to be kept. Higher keeps longer.
    ///
    /// Design § 37's five, read as a ladder: `Canonical` is "durable, revisioned, strongly
    /// governed"; `Provenance` is kept while it is needed to verify a canonical assertion;
    /// `Incubating` while it still might be integrated; `Cache` is "reproducible, and freely
    /// deletable"; `Ephemeral` is working state with the shortest life of the five.
    ///
    /// **Not the derived [`Ord`].** That orders by declaration position, which runs the other way —
    /// `Canonical < Ephemeral` — so `max()` over two classes picks the one that may be deleted
    /// first. The derived ordering exists to make [`StoredObject`] sortable and means nothing about
    /// retention; this is the retention order, and it is the only thing [`Self::strongest`] reads.
    #[must_use]
    pub const fn retention_rank(&self) -> u8 {
        match self {
            Self::Canonical => 4,
            Self::Provenance => 3,
            Self::Incubating => 2,
            Self::Cache => 1,
            Self::Ephemeral => 0,
        }
    }

    /// The stronger of two retention requirements.
    ///
    /// What [`ObjectStore::put`] folds a repeated write with: content-addressed bytes are one
    /// object and a class is a requirement about them, so the strongest requirement anyone has
    /// stated wins. See that method for why it is neither the first nor the last.
    #[must_use]
    pub const fn strongest(self, other: Self) -> Self {
        if self.retention_rank() >= other.retention_rank() {
            self
        } else {
            other
        }
    }
}

/// One immutable payload, addressed by its content: `ekr.store.StoredObject`, design § 57.
///
/// Its identity is `content_hash`, so two writes of the same bytes are one object however many
/// times they arrive and whoever writes them. The address is [`ContentHash::of_bytes`] — the
/// payload domain — and never [`ContentHash::of`]: these are bytes the runtime did not choose, and
/// the two address spaces are deliberately disjoint.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct StoredObject {
    /// Its address, which is its identity.
    pub content_hash: ContentHash,
    /// What it is kept for.
    pub storage_class: StorageClass,
    /// How many bytes it is. `byte_len >= 0` is the domain's invariant and this type's shape.
    pub byte_len: u64,
    /// When it was first stored — the *first* write's instant, because a second write of the same
    /// bytes is the same object and does not move it.
    pub stored_at: Timestamp,
}

/// The content-addressed object store: bytes in, address out, bytes back.
///
/// Synchronous, for the reason [`RevisionLog`](crate::RevisionLog) is: the port underneath is
/// async and the bridge lives in this crate, so nothing above it is coloured
/// (`architecture-decision-record:0006-ekr-store-bridges-the-async-port`).
pub trait ObjectStore {
    /// Stores `bytes` under `storage_class`, or answers with the object that already holds them.
    ///
    /// `stored_at` is the caller's, because the runtime has no clock, and it is the **first**
    /// write's instant: the object is the bytes, the bytes did not change, and a later write did
    /// not create it.
    ///
    /// # The class is the strongest ever requested, never the first and never the last
    ///
    /// Content addressing makes two writes of one payload one object, so two callers who wanted
    /// different things of it have to be answered with one retention class. First-write-wins is
    /// wrong and was the first implementation's defect: bytes written as [`StorageClass::Cache`] —
    /// "reproducible, and freely deletable" — stayed freely deletable after a later write named
    /// them a revision lineage's seed, and design § 38's collector reads that class. Last-write-wins
    /// is wrong the same way in the other direction, and is worse for being order-dependent: a
    /// cache write arriving after a canonical one would release bytes canonical state depends on.
    ///
    /// A class is a **requirement** rather than a description, and requirements over one object
    /// compose by taking the strongest — [`StorageClass::strongest`]. So nothing durable is ever
    /// released because some caller wanted less, the answer does not depend on the order the
    /// writes arrived in, and the cost is that bytes may be kept longer than any single caller
    /// asked. That is the safe direction for a collector that deletes.
    ///
    /// Raising a class is recorded rather than overwritten: an object's history is append-only
    /// like everything else this crate holds, so the record a reader gets is a fold over every
    /// write of those bytes.
    ///
    /// # Errors
    ///
    /// [`StoreError::Backend`] when the provider is unavailable, and
    /// [`StoreError::Document`] when the record it holds cannot be read.
    fn put(
        &self,
        storage_class: StorageClass,
        bytes: &[u8],
        stored_at: Timestamp,
    ) -> Result<StoredObject, StoreError>;

    /// The bytes at `content_hash`, or `None` when this store does not hold them.
    ///
    /// # Errors
    ///
    /// [`StoreError::Backend`] when the provider is unavailable, and
    /// [`StoreError::Document`] when the record it holds cannot be read.
    fn get(&self, content_hash: &ContentHash) -> Result<Option<Vec<u8>>, StoreError>;
}
