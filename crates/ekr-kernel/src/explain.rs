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
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    BootstrapContext, CommitReceiptV1, GraphOperation, GraphTransaction, ProposalRecordV1,
    SeedResultV1, TransactionDocument, TransactionRecord, ValidationProfileV1, ValidationReceiptV1,
    VerifiedRead,
};
use ekr_core::{
    AgentId, AssertionId, ContentHash, EventId, EvidenceId, RevisionId, RevisionNumber, Timestamp,
    TransactionId,
};
use ekr_graph::{
    Assertion, AssertionLifecycle, CanonicalRef, Evidence, EvidenceSource, GraphSnapshot, Root,
};
use ekr_store::{evidence_root, knowledge_root, GraphDocument};
use serde::Serialize;

mod reviewed;

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

/// `ekr.kernel.ExplainedProposal`: an ordinary origin's retained proposal, by reference.
///
/// The record itself is named by its payload address; of the document it retains, only the
/// operations about the explained assertion are carried.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExplainedProposal {
    /// The proposed transaction.
    pub transaction_id: TransactionId,
    /// Payload address of the retained proposal record.
    pub record_hash: ContentHash,
    /// The proposal's occurrence.
    pub event_id: EventId,
    /// The host-attributed submitter.
    pub submitter: AgentId,
    /// When it was submitted.
    pub submitted_at: Timestamp,
    /// Hash of the exact document bytes the record retains.
    pub document_hash: ContentHash,
    /// How many operations that document holds.
    pub operation_count: u64,
    /// The document's operations about the explained assertion, in document order: the
    /// `AddAssertion` that added it.
    pub operations: Vec<GraphOperation>,
}

/// `ekr.kernel.ExplainedCommit`: a retained commit receipt, by reference.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExplainedCommit {
    /// The committed transaction.
    pub transaction_id: TransactionId,
    /// Domain identity of the revision it committed.
    pub revision_id: RevisionId,
    /// The commit's occurrence.
    pub event_id: EventId,
    /// Who committed it.
    pub committer: AgentId,
    /// When it was committed.
    pub committed_at: Timestamp,
    /// The root of the revision it produced.
    pub result: Root,
    /// Hash of that root.
    pub result_hash: ContentHash,
    /// Payload address of the retained commit receipt.
    pub record_hash: ContentHash,
    /// Payload address of the proposal record the receipt commits.
    pub proposal_record_hash: ContentHash,
    /// Payload address of the validation receipt the receipt commits.
    pub validation_record_hash: ContentHash,
}

/// `ekr.kernel.ExplainedLifecycle`: one committed retraction or supersession of an assertion.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExplainedLifecycle {
    /// The assertion whose lifecycle changed.
    pub assertion_id: AssertionId,
    /// The lifecycle that commit gave it.
    pub lifecycle: AssertionLifecycle,
    /// The commit of the change, by reference.
    pub commit: ExplainedCommit,
}

/// `ekr.kernel.ExplainedAttachment`: evidence attached to an explained assertion after it was
/// added, with the commit that attached it (`story:evidence-attaches-to-a-held-assertion`).
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExplainedAttachment {
    /// The assertion the evidence is attached to.
    pub assertion_id: AssertionId,
    /// The evidence attached; it is one of the chain's Evidence links.
    pub evidence_id: EvidenceId,
    /// The revision the attaching commit produced.
    pub revision: RevisionNumber,
    /// The attaching commit, by reference.
    pub commit: ExplainedCommit,
}

/// `ekr.kernel.ExplanationLink`, tagged by `kind`.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind")]
pub enum ExplanationLink {
    /// A selected assertion as it stands at the captured revision.
    Assertion(Assertion),
    /// Its seed origin.
    Seed(ExplainedSeed),
    /// Its ordinary origin's retained proposal, by reference.
    Proposal(ExplainedProposal),
    /// Its ordinary origin's accepted validation, boxed: it carries the whole validation receipt.
    Validation(Box<ExplainedValidation>),
    /// Its ordinary origin's commit, by reference.
    Commit(ExplainedCommit),
    /// A later committed lifecycle change, through the captured revision.
    Lifecycle(ExplainedLifecycle),
    /// Evidence attached to it after it was added, through the captured revision.
    Attachment(ExplainedAttachment),
    /// An exact signed human correction and its immutable retained record address.
    HumanAnswer(Box<ekr_core::contract_data::EkrKernelExplainedAnswer>),
    /// Exact independently retained mapping bytes of a committed derivation.
    Mapping(Box<ekr_core::contract_data::EkrIntegrateRetainedMappingRecord>),
    /// A committed assertion's actual retained evidence and mapping correspondence.
    Derivation(Box<ekr_core::contract_data::EkrIntegrateCanonicalDerivationRecord>),
    /// Supporting evidence whose retained payload was verified at its content address.
    Evidence(Evidence),
}

/// `ekr.kernel.ExplanationResult`.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct ExplanationResult {
    /// [`ExplanationResult::FORMAT`].
    pub format: String,
    /// The requested assertion.
    pub assertion_id: AssertionId,
    /// The captured revision every link was read at.
    pub at: RevisionNumber,
    /// The chain; its length is the `links` count an `Explained` event reports.
    pub links: Vec<ExplanationLink>,
}
impl ExplanationResult {
    /// The answer format: `/2` references proposals, receipts and evidence by hash. The
    /// unversioned answer before it embedded each whole record.
    pub const FORMAT: &'static str = "ekr.explanation/2";
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

/// One committed transaction visible at the captured boundary, as the index reads it from its
/// retained commit receipt ([`indexed`]): no document is parsed until [`Committed::transaction`].
struct Committed<'a> {
    revision: RevisionNumber,
    committed_at: Timestamp,
    record: &'a TransactionRecord,
    receipt: &'a CommitReceiptV1,
    /// The proposal document the receipt commits, as retained.
    document: &'a [u8],
}

/// The index's one reading of a commit receipt: the revision it produced, when, and the proposal
/// document it committed. `None` for a transaction the record holds no commit of.
///
/// Which receipt field holds the committed document is read here and nowhere else in the index,
/// so a receipt that names its proposal by hash changes this function alone.
fn indexed(record: &TransactionRecord) -> Option<Committed<'_>> {
    let receipt = record.committed.as_ref()?;
    Some(Committed {
        revision: receipt.result.revision,
        committed_at: receipt.committed_at,
        record,
        receipt,
        document: &receipt.proposal.document_bytes,
    })
}

impl Committed<'_> {
    /// The committed transaction, parsed from its retained document.
    fn transaction(&self) -> Result<GraphTransaction, ProjectionError> {
        match TransactionDocument::parse(self.document) {
            Ok(document) => Ok(document.transaction().clone()),
            Err(_) => unverified("proposal-document"),
        }
    }
    /// The receipt by reference; `record_hash` is its verified payload address.
    fn explained(&self, record_hash: ContentHash) -> ExplainedCommit {
        let receipt = self.receipt;
        ExplainedCommit {
            transaction_id: self.record.proposal.transaction_id,
            revision_id: receipt.revision_id,
            event_id: receipt.event_id,
            committer: receipt.committer,
            committed_at: receipt.committed_at,
            result: receipt.result,
            result_hash: receipt.result_hash,
            record_hash,
            proposal_record_hash: self.record.proposal_record_hash,
            validation_record_hash: receipt.validation_record_hash,
        }
    }
}

/// The operations of `transaction` about assertion `id`: its addition, retraction or
/// supersession, in document order.
fn about(transaction: &GraphTransaction, id: AssertionId) -> Vec<GraphOperation> {
    transaction
        .operations
        .iter()
        .filter(|op| match op {
            GraphOperation::AddAssertion(added) => added.id == id,
            GraphOperation::RetractAssertion(r) => r.assertion == id,
            GraphOperation::SupersedeAssertion(s) => s.assertion == id,
            _ => false,
        })
        .cloned()
        .collect()
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
    /// Starts with the requested assertion, then related supersessions and reviewed temporal
    /// replacements (including their original assertions), in
    /// stable id order, each once. Per assertion: the assertion, its origin (Seed, or Proposal,
    /// Validation and Commit), then its lifecycle changes through the captured revision in
    /// revision order, then the evidence attached to it through the captured revision, in
    /// evidence-id order, each with its commit (design § 103.4). The chain ends in the union, by
    /// [`EvidenceId`], of each selected assertion's evidence, the evidence attached to it, and
    /// the complete evidence sets of the included ordinary origin and lifecycle transactions, each
    /// payload verified at its content address. HumanStatement
    /// evidence terminates; any other source refuses rather than fabricating a further step.
    ///
    /// Reviewed decisions appear as generated HumanAnswer links with their retained statement
    /// evidence, including uncertainty and a chosen claim whose own lifecycle stays active.
    /// Ordinary validations use the authority profile in force at their original basis.
    ///
    /// The chain is looked up, not found by reading every committed document. Replay records in
    /// each assertion of the verified graph the instant of the commit that added it (its
    /// `transaction_time.recorded_from`) and the revision of the commit that retracted or
    /// superseded it (its lifecycle's `at_revision`); the committed transactions of this boundary
    /// are indexed by revision and instant from their receipts alone. Only the documents of the
    /// commits so named are parsed, and each is held to what the graph says it did: an origin no
    /// such document adds is `origin-missing`, and a lifecycle no such document reproduces is
    /// `lifecycle-disagrees`, never a shorter chain. Two records claiming one revision are
    /// `origin-ambiguous`, before any document is read.
    ///
    /// # What explain trusts
    ///
    /// Explain trusts the verified graph and reads only the documents on the assertion's own
    /// chain (`ekr.kernel.ExplanationResult` in `systems/ekr/domains/kernel.yaml`, "What explain
    /// trusts"). A record of this capture that is on no chain — another commit whose document
    /// also adds the assertion, at another instant, or retracts an assertion the graph holds
    /// active — is not read, so it neither changes the answer nor refuses it: the answer is the
    /// chain of verified links the graph names. Such a record cannot come from a store: every
    /// retained record is replayed and verified when the store is opened, and a store altered on
    /// disk is refused there. Re-reading every committed document to look for one is the cost
    /// this lookup removes.
    ///
    /// Proposals and commits are answered by reference ([`ExplainedProposal`],
    /// [`ExplainedCommit`]); a proposal carries only its operations about the explained assertion.
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
        // One committed transaction per revision: two records claiming one revision leave the
        // origin of whatever that revision added ambiguous, decided before any document is read.
        let mut index: BTreeMap<RevisionNumber, Committed<'_>> = BTreeMap::new();
        for c in self.transactions.values().filter_map(indexed) {
            if c.revision <= self.root.revision
                && (self.answers.contains_key(&c.revision) || index.insert(c.revision, c).is_some())
            {
                return unverified("origin-ambiguous");
            }
        }
        let mut parsed: BTreeMap<RevisionNumber, GraphTransaction> = BTreeMap::new();
        let mut transaction = |c: &Committed<'_>| -> Result<GraphTransaction, ProjectionError> {
            if let Some(held) = parsed.get(&c.revision) {
                return Ok(held.clone());
            }
            let transaction = c.transaction()?;
            parsed.insert(c.revision, transaction.clone());
            Ok(transaction)
        };
        let mut links = Vec::new();
        let mut support: BTreeSet<EvidenceId> = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut linked_answers = BTreeSet::new();
        let mut linked_mappings = BTreeSet::new();
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
            for step in &self.application_steps {
                for derivation in step
                    .derivations
                    .iter()
                    .filter(|d| d.assertion_id.0 == id.to_string())
                {
                    let evidence_id: EvidenceId =
                        derivation.evidence_id.0.parse().map_err(|_| {
                            ProjectionError::Unverified {
                                code: "derivation-evidence-identity".into(),
                            }
                        })?;
                    let Some(evidence) = self.graph.evidence.get(&evidence_id) else {
                        return unverified("derivation-evidence-missing");
                    };
                    if !assertion.evidence.contains(&CanonicalRef::new(evidence_id)) {
                        return unverified("derivation-evidence-not-cited");
                    }
                    let observation = match &evidence.source {
                        EvidenceSource::Observation(id) => {
                            ekr_core::contract_data::EssPresence::Present(Box::new(
                                ekr_core::contract_data::EkrGraphObservationId(id.to_string()),
                            ))
                        }
                        _ => ekr_core::contract_data::EssPresence::Absent,
                    };
                    if derivation.observation_id != observation {
                        return unverified("derivation-observation-disagrees");
                    }
                    let Some(mapping) = step
                        .mappings
                        .iter()
                        .find(|m| m.mapping_id == derivation.mapping_id)
                    else {
                        return unverified("derivation-mapping-missing");
                    };
                    let mapping_hash: ContentHash =
                        mapping.mapping_digest.0.parse().map_err(|_| {
                            ProjectionError::Unverified {
                                code: "mapping-digest".into(),
                            }
                        })?;
                    let Some(bytes) = self.content(&mapping_hash) else {
                        return unverified("mapping-bytes-missing");
                    };
                    let payload = ekr_core::bytes::decode(&mapping.payload).map_err(|_| {
                        ProjectionError::Unverified {
                            code: "mapping-payload".into(),
                        }
                    })?;
                    let expected = serde_json::to_vec(&mapping.mapping).map_err(|_| {
                        ProjectionError::Unverified {
                            code: "mapping-value".into(),
                        }
                    })?;
                    let source: ContentHash =
                        mapping.source_document_digest.0.parse().map_err(|_| {
                            ProjectionError::Unverified {
                                code: "mapping-source-digest".into(),
                            }
                        })?;
                    if bytes != payload
                        || bytes != expected
                        || ContentHash::of_bytes(bytes) != mapping_hash
                        || self
                            .content(&source)
                            .is_none_or(|bytes| ContentHash::of_bytes(bytes) != source)
                        || !mapping.evidence.contains(&derivation.evidence_id)
                    {
                        return unverified("mapping-content-disagrees");
                    }
                    if linked_mappings.insert(mapping.mapping_id.0.clone()) {
                        links.push(ExplanationLink::Mapping(mapping.clone()));
                    }
                    links.push(ExplanationLink::Derivation(derivation.clone()));
                }
            }

            // The origin is matched by revision: each revision whose verified coordinate, or whose
            // record's receipt, was committed at the instant the graph records the assertion was
            // added. A receipt that disagrees with its coordinate is still read, and `verify`
            // refuses it as `commit-record-disagrees`.
            let recorded = assertion.transaction_time.recorded_from;
            let at_instant: BTreeSet<RevisionNumber> = self
                .revisions
                .iter()
                .filter(|(number, coordinate)| {
                    **number != RevisionNumber::SEED && coordinate.committed_at == recorded
                })
                .map(|(number, _)| *number)
                .chain(
                    index
                        .values()
                        .filter(|c| c.committed_at == recorded)
                        .map(|c| c.revision),
                )
                .collect();
            let mut origins = Vec::new();
            for candidate in at_instant.iter().filter_map(|number| index.get(number)) {
                let committed = transaction(candidate)?;
                if committed
                    .operations
                    .iter()
                    .any(|op| matches!(op, GraphOperation::AddAssertion(added) if added.id == id))
                {
                    origins.push((candidate, committed));
                }
            }
            let seeded = self.seed_input.graph.assertions.contains_key(&id);
            let mut reviewed_origins = Vec::new();
            for revision in at_instant.iter().filter(|n| self.answers.contains_key(n)) {
                let (_, committed) = self.explained_answer(*revision)?;
                if committed
                    .operations
                    .iter()
                    .any(|op| matches!(op, GraphOperation::AddAssertion(a) if a.id == id))
                {
                    reviewed_origins.push(*revision);
                }
            }
            match (seeded, origins.as_slice(), reviewed_origins.as_slice()) {
                (true, [], []) => links.push(ExplanationLink::Seed(self.explained_seed()?)),
                (false, [(origin, committed)], []) => {
                    let (validation, record_hash) = self.verify(origin)?;
                    let proposal = &origin.record.proposal;
                    links.push(ExplanationLink::Proposal(ExplainedProposal {
                        transaction_id: proposal.transaction_id,
                        record_hash: origin.record.proposal_record_hash,
                        event_id: proposal.event_id,
                        submitter: proposal.submitter,
                        submitted_at: proposal.submitted_at,
                        document_hash: proposal.document_hash,
                        operation_count: proposal.operation_count,
                        operations: about(committed, id),
                    }));
                    links.push(ExplanationLink::Validation(Box::new(validation)));
                    links.push(ExplanationLink::Commit(origin.explained(record_hash)));
                    support.extend(committed.evidence.iter().copied());
                }
                (false, [], [_]) => {} // The exact reviewed origin is linked below.
                (false, [], []) => return unverified("origin-missing"),
                _ => return unverified("origin-ambiguous"),
            }

            let mut current = AssertionLifecycle::Active;
            let changed_revision = match &assertion.lifecycle {
                AssertionLifecycle::Active => None,
                AssertionLifecycle::Retracted { at_revision, .. }
                | AssertionLifecycle::Superseded { at_revision, .. } => Some(*at_revision),
            };
            let changed = changed_revision.and_then(|revision| index.get(&revision));
            if let Some(change) = changed {
                let committed = transaction(change)?;
                for op in &committed.operations {
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
                        commit: change.explained(record_hash),
                    }));
                    support.extend(committed.evidence.iter().copied());
                    current = lifecycle;
                }
            } else if let Some(revision) =
                changed_revision.filter(|revision| self.answers.contains_key(revision))
            {
                let (_, committed) = self.explained_answer(revision)?;
                for operation in &committed.operations {
                    if let GraphOperation::RetractAssertion(change) = operation {
                        if change.assertion == id {
                            current = AssertionLifecycle::Retracted {
                                at_revision: revision,
                                reason: change.reason.clone(),
                            };
                        }
                    }
                }
            }
            require(assertion.lifecycle == current, "lifecycle-disagrees")?;

            for (revision, answer) in &self.answers {
                let identity = id.to_string();
                let relevant = changed_revision == Some(*revision)
                    || answer
                        .corrections
                        .iter()
                        .any(|c| c.assertion_id.0 == identity)
                    || answer
                        .replacements
                        .iter()
                        .any(|r| r.previous.0 == identity || r.replacement.0 == identity);
                if !relevant {
                    continue;
                }
                let (explained, committed) = self.explained_answer(*revision)?;
                for replacement in &answer.replacements {
                    if replacement.previous.0 == identity || replacement.replacement.0 == identity {
                        for member in [&replacement.previous.0, &replacement.replacement.0] {
                            pending.insert(member.parse().map_err(|_| {
                                ProjectionError::Unverified {
                                    code: "answer-replacement-invalid".into(),
                                }
                            })?);
                        }
                    }
                }
                support.extend(committed.evidence.iter().copied());
                support.insert(answer.statement_evidence.0.parse().map_err(|_| {
                    ProjectionError::Unverified {
                        code: "answer-evidence-invalid".into(),
                    }
                })?);
                if linked_answers.insert(*revision) {
                    links.push(ExplanationLink::HumanAnswer(Box::new(explained)));
                }
            }

            // Each attachment the captured graph holds for this assertion, by the revision that
            // made it: that commit's document must attach exactly this evidence to it. Only the
            // attached id joins the chain's evidence, not the attaching commit's whole set.
            for attached in self.graph.attached(id) {
                let evidence = attached.evidence.id();
                let Some(change) = index.get(&attached.revision) else {
                    return unverified("attachment-disagrees");
                };
                let committed = transaction(change)?;
                require(
                    committed.operations.iter().any(|op| {
                        matches!(op, GraphOperation::AttachEvidence(a)
                            if a.assertion == id && a.evidence == evidence)
                    }),
                    "attachment-disagrees",
                )?;
                let (_, record_hash) = self.verify(change)?;
                links.push(ExplanationLink::Attachment(ExplainedAttachment {
                    assertion_id: id,
                    evidence_id: evidence,
                    revision: change.revision,
                    commit: change.explained(record_hash),
                }));
                support.insert(evidence);
            }
        }
        for id in support {
            let Some(evidence) = self.graph.evidence.get(&id) else {
                return unverified("evidence-missing");
            };
            match evidence.source {
                EvidenceSource::HumanStatement { .. } => {}
                EvidenceSource::Observation(id) => {
                    let Some(observation) = self.observations.get(&id) else {
                        return unverified("evidence-observation-unavailable");
                    };
                    let payload = ekr_core::bytes::decode(&observation.payload).map_err(|_| {
                        ProjectionError::Unverified {
                            code: "evidence-observation-payload".into(),
                        }
                    })?;
                    require(
                        observation.observation.observation_id.0 == id.to_string()
                            && observation.observation.content_hash.0
                                == evidence.content_hash.to_string()
                            && self.content(&evidence.content_hash) == Some(payload.as_slice()),
                        "evidence-observation-disagrees",
                    )?;
                }
                _ => return unverified("evidence-source-unexplained"),
            }
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
            format: ExplanationResult::FORMAT.to_owned(),
            assertion_id: requested,
            at: self.root.revision,
            links,
        })
    }

    /// Binds every capture field a projection reads to retained bytes, before either reads it.
    ///
    /// The root is the highest revision coordinate, and its retained record (seed result or
    /// commit receipt, authority transition or answer) names exactly that root. The graph is at
    /// that revision, and its knowledge, evidence and ontology roots and the active authority's
    /// agent root are the root's. The original authority stays bound to the seed envelope.
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
                .or_else(|| record.and_then(reviewed::root))
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
            ContentHash::of(self.authority_at(self.root.revision)) == self.root.agent_root,
            "authority-root-disagrees",
        )?;
        let envelope = self
            .content(&self.seed.seed_hash)
            .filter(|bytes| ContentHash::of_bytes(bytes) == self.seed.seed_hash)
            .and_then(|bytes| crate::seed::envelope(bytes).ok());
        require(
            envelope.is_some_and(|envelope| {
                envelope.input.holds(&self.seed_input)
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
        let profile = &self
            .authority_at(receipt.validation.basis.previous_root.revision)
            .validation_profile;
        require(
            ContentHash::of(profile) == receipt.validation.basis.validation_profile_hash,
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
                validation_profile: profile.clone(),
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
