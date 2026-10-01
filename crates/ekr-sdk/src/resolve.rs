//! Resolve-or-create through a cache (`story:sdk-resolve-and-batch`).
//!
//! A [`Resolver`] answers a [`TypedReference`] from its cache when it can and asks `ekr resolve`
//! when it cannot. The cache is keyed by the reference's exact type id and its aliases, sorted,
//! deduplicated and without the empty alias, which identifies nothing: order and repeats do not
//! change the key, and a reference to a supertype is a different key from one to its subtype
//! (`decision-blocker:typed-reference-subtype-matching` is open).
//!
//! A `ProposeNew` answer mints a node id and queues a `CreateNode` carrying the answer's aliases,
//! and drops every other cached key of the same type id that shares one of those aliases: once
//! the node is committed, `ekr resolve` no longer answers such a key with the node it cached (it
//! may answer `Ambiguous`), the same rule [`Resolver::observe`] applies to a committed
//! `CreateNode`. [`Resolver::flush`] commits the queue through a [`Batcher`], one group per node.
//! A resolve the cache cannot answer that shares an alias with a queued node of the same type
//! flushes first, so `ekr resolve` sees that node instead of proposing a second. Each flush first
//! reads `ekr head`: a revision the SDK did not commit drops the cache, and every queued reference
//! is resolved again, so a node another process created meanwhile replaces the queued one instead
//! of colliding with it (`alias-already-exists`).

use std::collections::{BTreeMap, BTreeSet};

use ekr_core::{AgentId, GraphRootId, NodeId, TypeId};
use serde_json::Value as Json;

use crate::batch::{call, unanswered, BatchError, BatchReport, Batcher, CallError, Rejection};
use crate::document::{NodeDraft, Operation, TypedReference};
use crate::reply::{Answer, Outcome};
use crate::transport::{Request, Transport};

/// The exact type id and the sorted, distinct, non-empty aliases.
type Key = (TypeId, Vec<String>);

/// What a reference resolved to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolution {
    /// Exactly one node the store holds.
    Resolved(NodeId),
    /// A node minted for a `ProposeNew` answer, queued for the next flush.
    Queued(NodeId),
    /// More than one candidate, in id order; none is chosen and nothing is cached.
    Ambiguous(Vec<NodeId>),
}

/// What flushes did since the last [`Resolver::flush`] returned: the resolve that flushed
/// because it shared an alias with a queued node included.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Flushed {
    /// The batches that committed the queued nodes. Groups are numbered within the queue that
    /// was flushed, and each rejected operation is the `CreateNode` itself.
    pub report: BatchReport,
    /// Queued nodes that were not created because the store now answers their reference with
    /// another node, or with several, which another process committed meanwhile. The queued id
    /// never exists; use the resolution instead.
    pub replaced: BTreeMap<NodeId, Resolution>,
}

/// One node queued for creation.
#[derive(Clone, Debug)]
struct Queued {
    key: Key,
    draft: NodeDraft,
}

/// `ekr resolve`'s answer.
enum Found {
    Resolved(NodeId),
    ProposeNew(Vec<String>),
    Ambiguous(Vec<NodeId>),
}

/// Resolves typed references through a cache, and creates what a store does not hold.
#[derive(Debug)]
pub struct Resolver {
    root: GraphRootId,
    batcher: Batcher,
    cache: BTreeMap<Key, NodeId>,
    queued: Vec<Queued>,
    /// Queued nodes whose reference must be resolved again before they are proposed: refused as
    /// `alias-already-exists` by a flush that then failed, or not yet resolved again when a
    /// request failed. The next flush resolves them first.
    recheck: Vec<Queued>,
    /// The head the cache is known to agree with; unread until the first resolve is asked.
    known: Option<u64>,
    /// Revisions the SDK committed past `known`.
    own: BTreeSet<u64>,
    pending: Flushed,
}

impl Resolver {
    /// A resolver creating nodes under graph root `root`, proposed by `proposer`, the host
    /// operator.
    #[must_use]
    pub fn new(root: GraphRootId, proposer: AgentId) -> Self {
        Self {
            root,
            batcher: Batcher::new(proposer),
            cache: BTreeMap::new(),
            queued: Vec::new(),
            recheck: Vec::new(),
            known: None,
            own: BTreeSet::new(),
            pending: Flushed::default(),
        }
    }

    /// Resolve `reference`, queueing a node of its type named by its first non-empty alias when
    /// the store holds none.
    ///
    /// # Errors
    /// [`CallError`] when `ekr resolve`, `ekr head` or a flush gets no answer the SDK can act on,
    /// including a reference `ekr resolve` refuses (`reference-type-has-subtypes`, …).
    pub fn resolve<T: Transport + ?Sized>(
        &mut self,
        transport: &mut T,
        reference: &TypedReference,
    ) -> Result<Resolution, CallError> {
        let (root, type_id) = (self.root, reference.type_id);
        let name = reference
            .aliases
            .iter()
            .find(|alias| !alias.is_empty())
            .cloned()
            .unwrap_or_default();
        self.resolve_with(transport, reference, || NodeDraft::new(root, type_id, name))
    }

    /// Resolve `reference`, queueing the node `draft` returns when the store holds none. The
    /// resolver sets that node's id (freshly minted), its type (the reference's) and its aliases
    /// (the ones `ekr resolve` names); the draft supplies its root, name and values.
    ///
    /// # Errors
    /// As [`Resolver::resolve`].
    pub fn resolve_with<T: Transport + ?Sized, F: FnOnce() -> NodeDraft>(
        &mut self,
        transport: &mut T,
        reference: &TypedReference,
        draft: F,
    ) -> Result<Resolution, CallError> {
        let key = key_of(reference);
        if let Some(&node) = self.cache.get(&key) {
            return Ok(if self.is_queued(node) {
                Resolution::Queued(node)
            } else {
                Resolution::Resolved(node)
            });
        }
        let shares =
            self.queued.iter().chain(&self.recheck).any(|queued| {
                queued.key.0 == key.0 && queued.key.1.iter().any(|a| key.1.contains(a))
            });
        if shares {
            self.flush_queue(transport)?;
        }
        if self.known.is_none() {
            self.known = Some(head(transport)?);
        }
        Ok(match ask(transport, &key)? {
            Found::Resolved(node) => {
                self.cache.insert(key, node);
                Resolution::Resolved(node)
            }
            Found::ProposeNew(aliases) => {
                let mut node = draft();
                node.id = NodeId::mint();
                node.type_id = key.0;
                node.aliases = aliases;
                let id = node.id;
                // A cached answer sharing an alias with the queued node is one `ekr resolve`
                // stops giving once the node is committed.
                for alias in &node.aliases {
                    self.invalidate(key.0, alias);
                }
                self.cache.insert(key.clone(), id);
                self.queued.push(Queued { key, draft: node });
                Resolution::Queued(id)
            }
            Found::Ambiguous(candidates) => Resolution::Ambiguous(candidates),
        })
    }

    /// Drop every cached answer whose key holds `alias` for `type_id`. The next resolve of such a
    /// key asks `ekr resolve` again.
    pub fn invalidate(&mut self, type_id: TypeId, alias: &str) {
        self.cache
            .retain(|key, _| !(key.0 == type_id && key.1.iter().any(|held| held == alias)));
    }

    /// Record a consumer's own batches: their revisions are the SDK's, so the next flush does not
    /// drop the cache for them, and every alias a committed `CreateNode` or `AddAlias` gives is
    /// invalidated (an `AddAlias` for every type, since the resolver does not know the node's).
    pub fn observe(&mut self, report: &BatchReport, groups: &[Vec<Operation>]) {
        for committed in &report.committed {
            self.own.insert(committed.revision);
            let operations = committed
                .groups
                .iter()
                .filter_map(|&group| groups.get(group))
                .flatten();
            for operation in operations {
                match operation {
                    Operation::CreateNode(node) => {
                        for alias in &node.aliases {
                            self.invalidate(node.type_id, alias);
                        }
                    }
                    Operation::AddAlias(addition) => {
                        self.cache.retain(|key, _| !key.1.contains(&addition.alias))
                    }
                    _ => {}
                }
            }
        }
    }

    /// Commit every queued node, after checking the head for a commit the SDK did not make, and
    /// return what this and every flush since the last call did.
    ///
    /// # Errors
    /// [`CallError`] when a request gets no answer the SDK can act on. What was committed until
    /// then is kept for the next call, and nodes not yet committed stay queued.
    pub fn flush<T: Transport + ?Sized>(
        &mut self,
        transport: &mut T,
    ) -> Result<Flushed, CallError> {
        self.flush_queue(transport)?;
        Ok(std::mem::take(&mut self.pending))
    }

    /// What every flush since the last [`Resolver::flush`] returned did, taken: after a request
    /// failed, the commits it made before failing, which [`Resolver::flush`] would otherwise keep for
    /// the next call.
    pub(crate) fn take_pending(&mut self) -> Flushed {
        std::mem::take(&mut self.pending)
    }

    fn is_queued(&self, node: NodeId) -> bool {
        self.queued.iter().any(|queued| queued.draft.id == node)
    }

    /// The flush itself: the head check, then the queue through the batcher.
    fn flush_queue<T: Transport + ?Sized>(&mut self, transport: &mut T) -> Result<(), CallError> {
        let head = head(transport)?;
        let foreign = self.known.is_some_and(|known| {
            head < known || (known + 1..=head).any(|revision| !self.own.contains(&revision))
        });
        self.own.retain(|&revision| revision > head);
        self.known = Some(head);
        let mut again = std::mem::take(&mut self.recheck);
        if foreign {
            self.cache.clear();
            again.splice(0..0, std::mem::take(&mut self.queued));
        }
        self.reconcile(transport, again)?;
        if self.queued.is_empty() {
            return Ok(());
        }
        let groups: Vec<Vec<Operation>> = self
            .queued
            .iter()
            .map(|queued| vec![Operation::CreateNode(queued.draft.clone())])
            .collect();
        let (report, failure) = match self.batcher.commit(transport, &groups) {
            Ok(report) => (report, None),
            Err(error) => {
                let BatchError { report, cause, .. } = *error;
                (report, Some(*cause))
            }
        };
        self.own
            .extend(report.committed.iter().map(|committed| committed.revision));
        let committed: BTreeSet<usize> = report
            .committed
            .iter()
            .flat_map(|committed| committed.groups.iter().copied())
            .collect();
        let mut taken = Vec::new();
        let mut rejected = BTreeSet::new();
        for operation in &report.rejected {
            rejected.insert(operation.group);
            if let Rejection::Rejected { issues, .. } = &operation.rejection {
                if issues
                    .iter()
                    .any(|issue| issue.code == "alias-already-exists")
                {
                    taken.push(operation.group);
                }
            }
        }
        let queued = std::mem::take(&mut self.queued);
        let mut again = Vec::new();
        for (group, queued) in queued.into_iter().enumerate() {
            if committed.contains(&group) {
                continue;
            }
            if rejected.contains(&group) {
                self.cache.remove(&queued.key);
                if taken.contains(&group) {
                    again.push(queued);
                }
                continue;
            }
            self.queued.push(queued);
        }
        self.pending.report.committed.extend(report.committed);
        self.pending.report.rejected.extend(report.rejected);
        if report.refused.is_some() {
            self.pending.report.refused = report.refused;
        }
        if let Some(cause) = failure {
            // Each refused node is resolved again by the next flush, before anything is proposed.
            self.recheck.extend(again);
            return Err(cause);
        }
        // A node another process gave one of these aliases between the head check and the
        // commit: resolve again, and a node found replaces the one that was refused.
        self.reconcile(transport, again)
    }

    /// Resolve each of `queued` again: a node the store now holds replaces the queued one, and
    /// one it still does not hold stays queued. A queued node found under its own id was committed
    /// by the SDK, by a commit whose reply was lost: it is cached, and neither queued nor replaced.
    /// If a request fails, the nodes not yet resolved again are kept for the next flush.
    fn reconcile<T: Transport + ?Sized>(
        &mut self,
        transport: &mut T,
        queued: Vec<Queued>,
    ) -> Result<(), CallError> {
        let mut rest = queued.into_iter();
        while let Some(queued) = rest.next() {
            match ask(transport, &queued.key) {
                Ok(Found::ProposeNew(_)) => {
                    self.cache.insert(queued.key.clone(), queued.draft.id);
                    self.queued.push(queued);
                }
                Ok(Found::Resolved(node)) if node == queued.draft.id => {
                    self.cache.insert(queued.key, node);
                }
                Ok(Found::Resolved(node)) => {
                    self.cache.insert(queued.key, node);
                    self.pending
                        .replaced
                        .insert(queued.draft.id, Resolution::Resolved(node));
                }
                Ok(Found::Ambiguous(candidates)) => {
                    self.pending
                        .replaced
                        .insert(queued.draft.id, Resolution::Ambiguous(candidates));
                }
                Err(error) => {
                    self.recheck.push(queued);
                    self.recheck.extend(rest);
                    return Err(error);
                }
            }
        }
        Ok(())
    }
}

/// `reference`'s cache key.
fn key_of(reference: &TypedReference) -> Key {
    let aliases: BTreeSet<&String> = reference
        .aliases
        .iter()
        .filter(|alias| !alias.is_empty())
        .collect();
    (reference.type_id, aliases.into_iter().cloned().collect())
}

/// The head revision, from `ekr head`.
fn head<T: Transport + ?Sized>(transport: &mut T) -> Result<u64, CallError> {
    let request = Request::new(["head"]);
    match call(transport, &request)? {
        Answer::Outcome(Outcome::Other(document)) => {
            document["revision"]
                .as_u64()
                .ok_or_else(|| CallError::Unexpected {
                    verb: "head".to_owned(),
                    document,
                })
        }
        answer => Err(unanswered(&request, answer)),
    }
}

/// `ekr resolve -` of the reference `key` is.
fn ask<T: Transport + ?Sized>(transport: &mut T, key: &Key) -> Result<Found, CallError> {
    let reference = TypedReference::new(key.0, key.1.iter().cloned());
    let request = Request::new(["resolve", "-"]).with_stdin(reference.to_yaml()?);
    let document = match call(transport, &request)? {
        Answer::Outcome(Outcome::Other(document)) => document,
        answer => return Err(unanswered(&request, answer)),
    };
    let found = match document["kind"].as_str() {
        Some("Resolved") => field(&document, "node_id").map(Found::Resolved),
        Some("ProposeNew") => field(&document, "aliases").map(Found::ProposeNew),
        Some("Ambiguous") => field(&document, "candidates").map(Found::Ambiguous),
        _ => None,
    };
    found.ok_or_else(|| CallError::Unexpected {
        verb: "resolve".to_owned(),
        document,
    })
}

/// `document[name]`, read as a `V`.
fn field<V: serde::de::DeserializeOwned>(document: &Json, name: &str) -> Option<V> {
    serde_json::from_value(document[name].clone()).ok()
}
