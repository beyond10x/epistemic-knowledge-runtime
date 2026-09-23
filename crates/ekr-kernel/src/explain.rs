//! Snapshot and Explain: read projections over one kernel-owned [`VerifiedRead`].
//!
//! Both projections are methods on an already captured read. Neither opens storage, reads the
//! head again or takes provider authority, so a read captured before a later commit keeps
//! answering exactly as of its own boundary (`.engineering/waves/p1-cli-explain-contract-r2.md`).
//!
//! Before either projection reads the capture, its graph, root, seed, seed input, context and
//! authority are bound to retained bytes: the root to its retained record, the graph to that
//! root's knowledge, evidence and ontology roots, the graph's own `GraphRoot` to the retained
//! seed input's, the authority to its agent root, and the seed input to the retained envelope at
//! `seed.seed_hash`. Every transaction record an explanation
//! names is compared with the retained bytes at its actual payload address inside the same
//! capture, and every evidence payload with its content address. A missing or disagreeing record
//! refuses the whole result with [`ProjectionError::Unverified`]; no partial chain is returned as
//! though it established provenance.
use std::collections::BTreeSet;

use crate::{
    BootstrapContext, CommitReceiptV1, GraphOperation, GraphTransaction, ProposalRecordV1,
    SeedResultV1, TransactionDocument, TransactionRecord, ValidationProfileV1, ValidationReceiptV1,
    VerifiedRead,
};
use ekr_core::{AssertionId, ContentHash, EvidenceId, RevisionId, RevisionNumber, Timestamp};
use ekr_graph::{
    Assertion, AssertionLifecycle, CanonicalRef, Evidence, EvidenceSource, GraphSnapshot, Root,
};
use ekr_store::{evidence_root, knowledge_root, GraphDocument};
use serde::Serialize;

/// `ekr.kernel.SnapshotResult`: the complete graph and root at one captured revision.
///
/// `valid_at` and `matching_assertions` are both absent without a selector. With one they carry
/// that instant and the ids [`GraphSnapshot::valid_at`] returns, in stable id order — possibly
/// empty. `root` and `graph` stay complete either way: a filtered view never claims a hash.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SnapshotResult {
    /// Domain identity of the captured revision.
    pub revision_id: RevisionId,
    /// Complete recomputed root of that revision.
    pub root: Root,
    /// Complete graph at that revision, as an `ekr.graph-document/2` document.
    pub graph: GraphDocument,
    /// The valid-time selector, when one was asked for.
    pub valid_at: Option<Timestamp>,
    /// Assertions believed at `valid_at`, in id order, when a selector was asked for.
    pub matching_assertions: Option<Vec<AssertionId>>,
}

/// `ekr.kernel.ExplainedSeed`: the actual seed admission an assertion originated in.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExplainedSeed {
    /// The original retained seed result.
    pub result: SeedResultV1,
    /// The actual bootstrap identities retained with the seed.
    pub context: BootstrapContext,
    /// The retained validation profile the seed was admitted under.
    pub validation_profile: ValidationProfileV1,
    /// Payload address of the seed result record.
    pub record_hash: ContentHash,
}

/// `ekr.kernel.ExplainedValidation`: the accepted validation of an ordinary origin.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExplainedValidation {
    /// The complete retained validation receipt, including its basis.
    pub receipt: ValidationReceiptV1,
    /// The profile whose hash that basis names.
    pub validation_profile: ValidationProfileV1,
    /// Payload address of the validation receipt record.
    pub record_hash: ContentHash,
}

/// `ekr.kernel.ExplainedLifecycle`: one committed retraction or supersession of an assertion.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExplainedLifecycle {
    /// The assertion whose lifecycle changed.
    pub assertion_id: AssertionId,
    /// The lifecycle that commit gave it.
    pub lifecycle: AssertionLifecycle,
    /// The complete retained commit receipt of the change.
    pub receipt: CommitReceiptV1,
    /// Payload address of that receipt record.
    pub record_hash: ContentHash,
}

/// `ekr.kernel.ExplanationLink`, tagged by `kind`.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum ExplanationLink {
    /// A selected assertion as it stands at the captured revision.
    Assertion(Assertion),
    /// Its seed origin.
    Seed(ExplainedSeed),
    /// Its ordinary origin's exact retained proposal.
    Proposal(ProposalRecordV1),
    /// Its ordinary origin's accepted validation.
    Validation(ExplainedValidation),
    /// Its ordinary origin's commit receipt.
    Commit(CommitReceiptV1),
    /// A later committed lifecycle change, through the captured revision.
    Lifecycle(ExplainedLifecycle),
    /// Supporting evidence whose retained payload was verified at its content address.
    Evidence(Evidence),
}

/// `ekr.kernel.ExplanationResult`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExplanationResult {
    /// The requested assertion.
    pub assertion_id: AssertionId,
    /// The captured revision every link was read at.
    pub at: RevisionNumber,
    /// The chain; its length is the `links` count an `Explained` event reports.
    pub links: Vec<ExplanationLink>,
}

/// Why a read projection refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ProjectionError {
    /// `ekr.kernel.AssertionNotFound`: the captured canonical core holds no such assertion.
    #[error("assertion {requested} does not exist")]
    AssertionNotFound {
        /// The requested identity.
        requested: AssertionId,
    },
    /// A required record or payload is missing or disagrees with its retained bytes.
    #[error("unverified explanation support: {code}")]
    Unverified {
        /// Which requirement failed.
        code: String,
    },
}

fn unverified<T>(code: &str) -> Result<T, ProjectionError> {
    Err(ProjectionError::Unverified { code: code.into() })
}
fn require(condition: bool, code: &str) -> Result<(), ProjectionError> {
    if condition {
        Ok(())
    } else {
        unverified(code)
    }
}

/// One committed transaction visible at the captured boundary.
struct Committed<'a> {
    revision: RevisionNumber,
    record: &'a TransactionRecord,
    receipt: &'a CommitReceiptV1,
    transaction: GraphTransaction,
}

impl VerifiedRead {
    /// `ekr.kernel.Snapshot` over this capture: complete graph and root, and, with `valid_at`,
    /// the ids [`GraphSnapshot::valid_at`] selects at that instant in stable order.
    ///
    /// # Errors
    /// [`ProjectionError::Unverified`] when the capture is not bound to its retained root and
    /// seed; see [`VerifiedRead::explain`] for the checks.
    pub fn snapshot(&self, valid_at: Option<Timestamp>) -> Result<SnapshotResult, ProjectionError> {
        let coordinate = self.bound()?;
        let matching_assertions = valid_at.map(|at| {
            let mut ids: Vec<AssertionId> = GraphSnapshot::of(&self.graph)
                .valid_at(at)
                .iter()
                .map(|assertion| assertion.id)
                .collect();
            ids.sort();
            ids
        });
        Ok(SnapshotResult {
            revision_id: coordinate.revision_id,
            root: self.root,
            graph: GraphDocument::of(&self.graph),
            valid_at,
            matching_assertions,
        })
    }

    /// `ekr.kernel.Explain` over this capture.
    ///
    /// Starts with the requested assertion, then every replacement a supersession selects, in
    /// stable id order, each once. Per assertion: the assertion, its origin (Seed, or Proposal,
    /// Validation and Commit), then its lifecycle changes through the captured revision in
    /// revision order. The chain ends in the union, by [`EvidenceId`], of each selected
    /// assertion's evidence and the complete evidence sets of the included ordinary origin and
    /// lifecycle transactions, each payload verified at its content address. HumanStatement
    /// evidence terminates; any other source refuses rather than fabricating a further step.
    ///
    /// # Errors
    /// [`ProjectionError::AssertionNotFound`] for an unknown id; [`ProjectionError::Unverified`]
    /// when the capture is not bound to its retained root and seed (see `bound`), or when any
    /// required record, replacement, acceptance or payload is missing or disagrees.
    pub fn explain(&self, requested: AssertionId) -> Result<ExplanationResult, ProjectionError> {
        self.bound()?;
        if !self.graph.assertions.contains_key(&requested) {
            return Err(ProjectionError::AssertionNotFound { requested });
        }
        let committed = self.committed()?;
        let mut links = Vec::new();
        let mut support: BTreeSet<EvidenceId> = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut pending = BTreeSet::from([requested]);
        while let Some(id) = pending.pop_first() {
            if !visited.insert(id) {
                continue;
            }
            let Some(assertion) = self.graph.assertions.get(&id) else {
                return unverified("replacement-assertion-missing");
            };
            links.push(ExplanationLink::Assertion(assertion.clone()));
            support.extend(assertion.evidence.iter().map(|cited| cited.id()));

            let origins: Vec<&Committed<'_>> = committed
                .iter()
                .filter(|c| {
                    c.transaction.operations.iter().any(
                        |op| matches!(op, GraphOperation::AddAssertion(added) if added.id == id),
                    )
                })
                .collect();
            let seeded = self.seed_input.graph.assertions.contains_key(&id);
            match (seeded, origins.as_slice()) {
                (true, []) => links.push(ExplanationLink::Seed(self.explained_seed()?)),
                (false, [origin]) => {
                    let (validation, _) = self.verify(origin)?;
                    links.push(ExplanationLink::Proposal(origin.receipt.proposal.clone()));
                    links.push(ExplanationLink::Validation(validation));
                    links.push(ExplanationLink::Commit(origin.receipt.clone()));
                    support.extend(origin.transaction.evidence.iter().copied());
                }
                (false, []) => return unverified("origin-missing"),
                _ => return unverified("origin-ambiguous"),
            }

            let mut current = AssertionLifecycle::Active;
            for change in &committed {
                for op in &change.transaction.operations {
                    let lifecycle = match op {
                        GraphOperation::RetractAssertion(r) if r.assertion == id => {
                            AssertionLifecycle::Retracted {
                                at_revision: change.revision,
                                reason: r.reason.clone(),
                            }
                        }
                        GraphOperation::SupersedeAssertion(s) if s.assertion == id => {
                            pending.insert(s.by);
                            AssertionLifecycle::Superseded {
                                by: CanonicalRef::new(s.by),
                                at_revision: change.revision,
                                effective_from: s.effective_from,
                            }
                        }
                        _ => continue,
                    };
                    let (_, record_hash) = self.verify(change)?;
                    links.push(ExplanationLink::Lifecycle(ExplainedLifecycle {
                        assertion_id: id,
                        lifecycle: lifecycle.clone(),
                        receipt: change.receipt.clone(),
                        record_hash,
                    }));
                    support.extend(change.transaction.evidence.iter().copied());
                    current = lifecycle;
                }
            }
            require(assertion.lifecycle == current, "lifecycle-disagrees")?;
        }
        for id in support {
            let Some(evidence) = self.graph.evidence.get(&id) else {
                return unverified("evidence-missing");
            };
            require(
                matches!(evidence.source, EvidenceSource::HumanStatement { .. }),
                "evidence-source-unexplained",
            )?;
            let Some(bytes) = self.content(&evidence.content_hash) else {
                return unverified("evidence-payload-missing");
            };
            require(
                ContentHash::of_bytes(bytes) == evidence.content_hash,
                "evidence-payload-mismatch",
            )?;
            links.push(ExplanationLink::Evidence(evidence.clone()));
        }
        Ok(ExplanationResult {
            assertion_id: requested,
            at: self.root.revision,
            links,
        })
    }

    /// Binds every capture field a projection reads to retained bytes, before either reads it.
    ///
    /// The root is the highest revision coordinate, and its retained record (seed result or
    /// commit receipt) names exactly that root. The graph is at that revision, and its
    /// knowledge, evidence and ontology roots and the authority's agent root are the root's.
    /// The retained seed envelope hashes to `seed.seed_hash` and holds exactly `seed_input`,
    /// `context` and `authority`; the graph root is that seed input's graph root; the retained
    /// seed result is `seed`.
    fn bound(&self) -> Result<&crate::VerifiedRevision, ProjectionError> {
        let Some((&last, coordinate)) = self.revisions.last_key_value() else {
            return unverified("revision-coordinate-missing");
        };
        require(
            last == self.root.revision && coordinate.root == self.root,
            "root-coordinate-disagrees",
        )?;
        let record = self.content(&coordinate.record_hash);
        let recorded = if last == RevisionNumber::SEED {
            record
                .and_then(|bytes| SeedResultV1::from_bytes(bytes).ok())
                .map(|seed| (seed.result, seed.revision_id))
        } else {
            record
                .and_then(|bytes| CommitReceiptV1::from_bytes(bytes).ok())
                .map(|receipt| (receipt.result, receipt.revision_id))
        };
        require(
            recorded == Some((self.root, coordinate.revision_id)),
            "root-record-disagrees",
        )?;
        require(
            self.graph.revision == self.root.revision,
            "graph-revision-disagrees",
        )?;
        require(
            knowledge_root(&self.graph) == self.root.knowledge_root,
            "knowledge-root-disagrees",
        )?;
        require(
            evidence_root(&self.graph) == self.root.evidence_root,
            "evidence-root-disagrees",
        )?;
        require(
            ContentHash::of(&self.graph.ontology) == self.root.ontology_root,
            "ontology-root-disagrees",
        )?;
        require(
            ContentHash::of(&self.authority) == self.root.agent_root,
            "authority-root-disagrees",
        )?;
        let envelope = self
            .content(&self.seed.seed_hash)
            .filter(|bytes| ContentHash::of_bytes(bytes) == self.seed.seed_hash)
            .and_then(|bytes| crate::seed::envelope(bytes).ok());
        require(
            envelope.is_some_and(|envelope| {
                envelope.input == self.seed_input
                    && envelope.context == self.context
                    && envelope.authority == self.authority
            }),
            "seed-envelope-disagrees",
        )?;
        // No sub-root covers the graph root, and no operation moves it: it is the seed's.
        require(
            self.graph.root == self.seed_input.graph.root,
            "graph-root-disagrees",
        )?;
        let Some(origin) = self.revisions.get(&RevisionNumber::SEED) else {
            return unverified("revision-coordinate-missing");
        };
        let held = self
            .content(&origin.record_hash)
            .and_then(|bytes| SeedResultV1::from_bytes(bytes).ok());
        require(held.as_ref() == Some(&self.seed), "seed-record-disagrees")?;
        Ok(coordinate)
    }

    /// Committed transactions through this boundary, in revision order, with parsed documents.
    fn committed(&self) -> Result<Vec<Committed<'_>>, ProjectionError> {
        let mut committed = Vec::new();
        for record in self.transactions.values() {
            let Some(receipt) = record.committed.as_ref() else {
                continue;
            };
            let revision = receipt.result.revision;
            if revision > self.root.revision {
                continue;
            }
            let Ok(document) = TransactionDocument::parse(&receipt.proposal.document_bytes) else {
                return unverified("proposal-document");
            };
            committed.push(Committed {
                revision,
                record,
                receipt,
                transaction: document.transaction().clone(),
            });
        }
        committed.sort_by_key(|c| c.revision);
        Ok(committed)
    }

    /// Checks a commit's proposal, validation and receipt against their retained bytes.
    fn verify(
        &self,
        c: &Committed<'_>,
    ) -> Result<(ExplainedValidation, ContentHash), ProjectionError> {
        let receipt = c.receipt;
        require(c.record.proposal == receipt.proposal, "proposal-disagrees")?;
        let proposal = self
            .content(&c.record.proposal_record_hash)
            .and_then(|bytes| ProposalRecordV1::from_bytes(bytes).ok());
        require(
            proposal.as_ref() == Some(&receipt.proposal),
            "proposal-record-disagrees",
        )?;
        require(
            c.record.validation.as_ref() == Some(&receipt.validation)
                && c.record.validation_record_hash == Some(receipt.validation_record_hash),
            "validation-disagrees",
        )?;
        let validation = self
            .content(&receipt.validation_record_hash)
            .and_then(|bytes| ValidationReceiptV1::from_bytes(bytes).ok());
        require(
            validation.as_ref() == Some(&receipt.validation),
            "validation-record-disagrees",
        )?;
        require(
            ContentHash::of(&self.authority.validation_profile)
                == receipt.validation.basis.validation_profile_hash,
            "validation-profile-disagrees",
        )?;
        let Some(coordinate) = self.revisions.get(&c.revision) else {
            return unverified("revision-coordinate-missing");
        };
        let held = self
            .content(&coordinate.record_hash)
            .and_then(|bytes| CommitReceiptV1::from_bytes(bytes).ok());
        require(
            held.as_ref() == Some(receipt)
                && coordinate.revision_id == receipt.revision_id
                && coordinate.root == receipt.result,
            "commit-record-disagrees",
        )?;
        Ok((
            ExplainedValidation {
                receipt: receipt.validation.clone(),
                validation_profile: self.authority.validation_profile.clone(),
                record_hash: receipt.validation_record_hash,
            },
            coordinate.record_hash,
        ))
    }

    /// The seed origin, checked against the retained seed result and authority anchor.
    fn explained_seed(&self) -> Result<ExplainedSeed, ProjectionError> {
        let Some(coordinate) = self.revisions.get(&RevisionNumber::SEED) else {
            return unverified("revision-coordinate-missing");
        };
        let held = self
            .content(&coordinate.record_hash)
            .and_then(|bytes| SeedResultV1::from_bytes(bytes).ok());
        require(
            held.as_ref() == Some(&self.seed) && coordinate.revision_id == self.seed.revision_id,
            "seed-record-disagrees",
        )?;
        require(
            ContentHash::of(&self.authority) == self.seed.authority_root,
            "seed-authority-disagrees",
        )?;
        Ok(ExplainedSeed {
            result: self.seed.clone(),
            context: self.context,
            validation_profile: self.authority.validation_profile.clone(),
            record_hash: coordinate.record_hash,
        })
    }
}
