//! Provider wrapper for deterministic initialization interleavings and interruption.
use ekr_core::{ContentHash, RevisionNumber, Timestamp};
use ekr_store::{
    Appended, Initialize, Inventory, ObjectStore, Publication, PublicationCommandKey,
    PublicationPreparationV1, RetainedHistory, RevisionLog, StorageClass, StoreError,
    StoreInventory, StoredObject,
};

pub struct Controlled<S> {
    pub inner: S,
    pub before_initialize: Box<dyn Fn()>,
    pub interrupt_after_seed: bool,
    pub interrupt_completion: bool,
}

impl<S: RevisionLog> RevisionLog for Controlled<S> {
    fn preparation(
        &self,
        key: &PublicationCommandKey,
    ) -> Result<Option<PublicationPreparationV1>, StoreError> {
        self.inner.preparation(key)
    }
    fn prepare(
        &self,
        key: &PublicationCommandKey,
        input: ContentHash,
        decision: &Publication,
        previous: Option<&PublicationPreparationV1>,
    ) -> Result<PublicationPreparationV1, StoreError> {
        self.inner.prepare(key, input, decision, previous)
    }
    fn resume(&self, prepared: &PublicationPreparationV1) -> Result<Appended, StoreError> {
        self.inner.resume(prepared)
    }
    fn history(&self) -> Result<RetainedHistory, StoreError> {
        self.inner.history()
    }
    fn history_at(&self, revision: RevisionNumber) -> Result<RetainedHistory, StoreError> {
        self.inner.history_at(revision)
    }
    fn publish(&self, publication: &Publication) -> Result<Appended, StoreError> {
        if self.interrupt_after_seed {
            Err(StoreError::Backend(
                "synthetic publication interruption".into(),
            ))
        } else {
            self.inner.publish(publication)
        }
    }
    fn checkpoint_covered(&self) -> Result<Option<u64>, StoreError> {
        self.inner.checkpoint_covered()
    }
    fn write_checkpoint(
        &self,
        covered: u64,
        binding: ContentHash,
        bytes: Option<&[u8]>,
    ) -> Result<bool, StoreError> {
        self.inner.write_checkpoint(covered, binding, bytes)
    }
    fn seed_bytes(&self) -> Result<Option<Vec<u8>>, StoreError> {
        self.inner.seed_bytes()
    }
    fn fold(&self) -> Result<ekr_graph::CanonicalGraph, StoreError> {
        self.inner.fold()
    }
    fn head(&self) -> Result<Option<ekr_graph::Root>, StoreError> {
        self.inner.head()
    }
    fn replay(&self, revision: RevisionNumber) -> Result<ekr_graph::CanonicalGraph, StoreError> {
        self.inner.replay(revision)
    }
}
impl<S: Initialize> Initialize for Controlled<S> {
    fn initialize(&self, publication: &Publication) -> Result<Appended, StoreError> {
        (self.before_initialize)();
        self.inner.initialize(publication)
    }
}
impl<S: ObjectStore> ObjectStore for Controlled<S> {
    fn put(
        &self,
        class: StorageClass,
        bytes: &[u8],
        at: Timestamp,
    ) -> Result<StoredObject, StoreError> {
        if self.interrupt_completion
            && bytes.starts_with(br#"{"format":"ekr.migration-finished/2""#)
        {
            return Err(StoreError::Backend(
                "synthetic completion interruption".into(),
            ));
        }
        self.inner.put(class, bytes, at)
    }
    fn get(&self, hash: &ContentHash) -> Result<Option<Vec<u8>>, StoreError> {
        self.inner.get(hash)
    }
}
impl<S: Inventory> Inventory for Controlled<S> {
    fn inventory(&self) -> Result<StoreInventory, StoreError> {
        self.inner.inventory()
    }
    fn is_empty(&self) -> Result<bool, StoreError> {
        self.inner.is_empty()
    }
}
