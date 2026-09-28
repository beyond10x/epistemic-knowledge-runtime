//! Bytes this process has already hashed to their content address, and a per-thread count of the
//! work history reads do.
//!
//! # Hashing a retained object at most once
//!
//! A blob is checked against its address when a store handle first loads it, and again by every
//! [`RetainedHistory::content`](crate::RetainedHistory::content) a replay makes. The second check
//! cannot simply be skipped: a `RetainedHistory` is plain public data, so its bytes may have been
//! changed since they were loaded. What it can do instead is compare them in full with bytes this
//! process has already hashed to the same address. SHA-256 is a function, so bytes equal to those
//! hash to the same address, and the comparison gives the answer the hash would have given; only
//! the SHA-256 is saved, and a comparison costs a small fraction of it. Bytes that differ in any
//! position or in length are hashed in full, exactly as before.
//!
//! The registry holds no bytes of its own. Its entries are weak references to the verified copies
//! store handles keep ([`register`]), so it holds bytes only while a handle does, and nothing
//! outlives the process.
use ekr_core::ContentHash;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, Weak};

/// Verified bytes by address, weakly: each entry is a handle's verified copy while it lives.
static REGISTRY: Mutex<Registry> = Mutex::new(Registry {
    by_hash: BTreeMap::new(),
    prune_at: PRUNE_FLOOR,
});

/// How many entries the registry holds before it first drops the ones no handle keeps.
const PRUNE_FLOOR: usize = 256;

struct Registry {
    by_hash: BTreeMap<ContentHash, Weak<Vec<u8>>>,
    /// The size at which dead entries are next dropped; doubles with the live entries.
    prune_at: usize,
}

/// Whether `bytes` are the payload `hash` addresses: compared with bytes this process already
/// hashed to `hash` when it holds some, and hashed otherwise.
pub(crate) fn addresses(hash: ContentHash, bytes: &[u8]) -> bool {
    let known = REGISTRY
        .lock()
        .ok()
        .and_then(|registry| registry.by_hash.get(&hash).and_then(Weak::upgrade));
    if known.is_some_and(|known| known.as_slice() == bytes) {
        return true;
    }
    count(|work| work.blobs_hashed += 1);
    ContentHash::of_bytes(bytes) == hash
}

/// The shared verified copy of the payload `hash` addresses, for a handle to keep: the copy another
/// handle already keeps, if one does, and otherwise `bytes`, registered.
///
/// The caller must have established that `bytes` are what `hash` addresses, with [`addresses`];
/// `crate::eventlog` does so for every object it passes here, through the one function that checks
/// a loaded blob. A live copy is verified for the same address, so the caller keeps it in place of
/// its own: one copy per process, and each handle keeps the registered one alive for as long as it
/// holds the object, whichever handle registered it.
pub(crate) fn register(hash: ContentHash, bytes: &[u8]) -> Arc<Vec<u8>> {
    let Ok(mut registry) = REGISTRY.lock() else {
        return Arc::new(bytes.to_vec());
    };
    if let Some(live) = registry.by_hash.get(&hash).and_then(Weak::upgrade) {
        return live;
    }
    let held = Arc::new(bytes.to_vec());
    registry.by_hash.insert(hash, Arc::downgrade(&held));
    if registry.by_hash.len() >= registry.prune_at {
        registry.by_hash.retain(|_, held| held.strong_count() > 0);
        registry.prune_at = PRUNE_FLOOR.max(registry.by_hash.len() * 2);
    }
    held
}

/// Work the reads on the calling thread have done since [`read_work`] last reported.
///
/// Test instrumentation, not part of the store's contract: it lets a test show that a second read
/// on one handle fetched and hashed no blob the first read verified. Counts are per thread because
/// every store call runs on its caller's thread, and tests in one binary run on several.
#[doc(hidden)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReadWork {
    /// Retained-object blobs fetched from the provider.
    pub blobs_read: u64,
    /// Blobs whose content address this crate computed over their bytes.
    pub blobs_hashed: u64,
    /// Revision-stream occurrences fetched from the provider and checked for the first time. The
    /// one held occurrence each read fetches again, to confirm the provider still has it, is not
    /// counted.
    pub occurrences_read: u64,
}

thread_local! {
    static WORK: Cell<ReadWork> = const { Cell::new(ReadWork {
        blobs_read: 0,
        blobs_hashed: 0,
        occurrences_read: 0,
    }) };
}

/// The work counted on this thread since the last call, which starts the count again.
#[doc(hidden)]
#[must_use]
pub fn read_work() -> ReadWork {
    WORK.with(|work| work.replace(ReadWork::default()))
}

pub(crate) fn count(add: impl FnOnce(&mut ReadWork)) {
    WORK.with(|work| {
        let mut now = work.get();
        add(&mut now);
        work.set(now);
    });
}

#[cfg(test)]
mod registry {
    use super::{addresses, read_work, register};
    use ekr_core::ContentHash;
    use std::sync::Arc;

    /// A registered copy saves the hash for equal bytes only, and only while a handle keeps it.
    #[test]
    fn equal_bytes_skip_the_hash_and_any_other_bytes_are_hashed() {
        let bytes = b"verified-registry unit payload".to_vec();
        let hash = ContentHash::of_bytes(&bytes);

        let _ = read_work();
        assert!(addresses(hash, &bytes));
        assert_eq!(read_work().blobs_hashed, 1, "unregistered bytes are hashed");

        let held = register(hash, &bytes);
        let again = register(hash, &bytes);
        assert!(
            Arc::ptr_eq(&held, &again),
            "a live copy is shared, not duplicated"
        );
        drop(again);
        assert!(addresses(hash, &bytes));
        assert_eq!(
            read_work().blobs_hashed,
            0,
            "equal registered bytes are compared"
        );

        let mut changed = bytes.clone();
        changed[4] ^= 1;
        assert!(!addresses(hash, &changed));
        assert!(!addresses(hash, &bytes[..bytes.len() - 1]));
        assert!(!addresses(hash, &[bytes.as_slice(), b"!"].concat()));
        assert_eq!(
            read_work().blobs_hashed,
            3,
            "different bytes are hashed in full"
        );

        drop(held);
        assert!(addresses(hash, &bytes));
        assert_eq!(
            read_work().blobs_hashed,
            1,
            "bytes no handle keeps any more are hashed again"
        );
    }
}
