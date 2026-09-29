//! Evidence that travels with the assertions citing it (`story:sdk-evidence-attachment`).
//!
//! A consumer holds its evidence as [`EvidenceItem`]s — a source identity, the time it was
//! observed and the exact bytes — and cites each through an [`EvidenceSet`], which hashes the
//! bytes and mints the evidence id here, without a request: [`payload_hash`] is the hash
//! `ekr hash` prints and [`EvidenceId::mint`] the function `ekr mint evidence` runs. The same item
//! cited again gets the same id.
//!
//! [`Batcher::commit_with_evidence`](crate::batch::Batcher::commit_with_evidence) turns each
//! entry into an `!AddEvidence` and puts it into the group of the first assertion that cites it
//! in each transaction it proposes, so bisection never separates an assertion from evidence it
//! introduces. A group whose entry was committed earlier cites the existing id.

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AgentId, ContentHash, EvidenceId, Timestamp};

use crate::document::{payload_hash, Confidence, EvidenceAddition, EvidenceSource, Operation};

/// One piece of a consumer's evidence: who or what said it, when it was observed, and its exact
/// bytes, trailing newline included.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceItem {
    /// The source's identity, written as `!HumanStatement {identity}`.
    pub source: String,
    /// When it was observed.
    pub observed_at: Timestamp,
    /// The exact bytes. An `!AddEvidence` carries at most 16,384 of them
    /// (`sequence_elements`); a group citing a larger item is refused as a document.
    pub bytes: Vec<u8>,
}

impl EvidenceItem {
    /// The item `source` said at `observed_at`, of exactly `bytes`.
    #[must_use]
    pub fn new(source: impl Into<String>, observed_at: Timestamp, bytes: Vec<u8>) -> Self {
        Self {
            source: source.into(),
            observed_at,
            bytes,
        }
    }

    /// [`payload_hash`] of its bytes: what `ekr hash` prints as `content_hash` for them.
    #[must_use]
    pub fn content_hash(&self) -> ContentHash {
        payload_hash(&self.bytes)
    }
}

/// What makes two items the same: source, observed-at time and the hash of the bytes.
type Key = (String, Timestamp, ContentHash);

/// The evidence entries a consumer's assertions cite, each minted and hashed here, and which of
/// them a store holds.
#[derive(Clone, Debug)]
pub struct EvidenceSet {
    extracted_by: AgentId,
    entries: BTreeMap<EvidenceId, EvidenceAddition>,
    ids: BTreeMap<Key, EvidenceId>,
    committed: BTreeSet<EvidenceId>,
}

impl EvidenceSet {
    /// An empty set whose entries `extracted_by`, the host operator, extracted.
    #[must_use]
    pub fn new(extracted_by: AgentId) -> Self {
        Self {
            extracted_by,
            entries: BTreeMap::new(),
            ids: BTreeMap::new(),
            committed: BTreeSet::new(),
        }
    }

    /// The evidence id to cite for `item`. The first time, its bytes are hashed and an id is
    /// minted for an entry of confidence [`Confidence::CERTAIN`]; an item with the same source,
    /// observed-at time and bytes gets the same id.
    pub fn cite(&mut self, item: EvidenceItem) -> EvidenceId {
        let key = (item.source.clone(), item.observed_at, item.content_hash());
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        let addition = EvidenceAddition::new(
            EvidenceSource::human(item.source),
            self.extracted_by,
            item.observed_at,
            Confidence::CERTAIN,
            item.bytes,
        );
        let id = addition.evidence.id;
        self.entries.insert(id, addition);
        self.ids.insert(key, id);
        id
    }

    /// The entry and bytes cited as `id`, when this set minted it.
    #[must_use]
    pub fn entry(&self, id: EvidenceId) -> Option<&EvidenceAddition> {
        self.entries.get(&id)
    }

    /// How many entries the set holds.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the set holds no entry.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether a transaction a batcher committed with this set added the entry `id`.
    #[must_use]
    pub fn is_committed(&self, id: EvidenceId) -> bool {
        self.committed.contains(&id)
    }

    /// Record that the store holds the entry `id`, so no later group adds it again: for a
    /// transaction whose commit outcome was unknown and which `ekr transactions --state Committed`
    /// lists.
    pub fn mark_committed(&mut self, id: EvidenceId) {
        self.committed.insert(id);
    }

    /// The `!AddEvidence` operations `operations` introduces into a transaction that already
    /// holds `attached`: one per id its assertions cite that this set minted, that is not
    /// committed and not in `attached`, in the order first cited. Each is added to `attached`.
    pub(crate) fn introduce(
        &self,
        operations: &[Operation],
        attached: &mut BTreeSet<EvidenceId>,
    ) -> Vec<Operation> {
        operations
            .iter()
            .filter_map(|operation| match operation {
                Operation::AddAssertion(assertion) => Some(assertion.evidence.iter()),
                _ => None,
            })
            .flatten()
            .filter_map(|id| {
                let entry = self.entries.get(id)?;
                (!self.committed.contains(id) && attached.insert(*id)).then(|| entry.clone().into())
            })
            .collect()
    }

    /// Record every id in `attached` as committed.
    pub(crate) fn extend_committed(&mut self, attached: BTreeSet<EvidenceId>) {
        self.committed.extend(attached);
    }
}
