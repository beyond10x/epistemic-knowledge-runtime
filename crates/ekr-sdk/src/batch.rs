//! Batches that respect the kernel's caps and bisect a rejection (`story:sdk-resolve-and-batch`).
//!
//! A [`Batcher`] commits a consumer's operations, given as atomic dependency groups: a group is
//! never split, and every transaction it proposes holds whole groups. Groups are packed in order
//! into batches under the `ekr.transaction-document/2` caps (10,000 operations, 8 MiB), a schema
//! change never sharing a batch with data. Each batch is proposed, validated and committed; a
//! `Stale` commit is proposed again under a newly minted transaction id; a batch that is rejected
//! is split in two and each half submitted again, down to the single group that is refused. The
//! [`BatchReport`] names every committed transaction and every rejected operation.
//!
//! [`Batcher::commit_with_evidence`] adds the entries of an [`EvidenceSet`] the groups cite, each
//! in the group of its first citing assertion within every transaction proposed, until one
//! commits it (`story:sdk-evidence-attachment`).

use std::collections::BTreeSet;

use ekr_core::{AgentId, TransactionId};
use serde::Deserialize;
use serde_json::Value as Json;

use crate::document::{
    to_yaml, DocumentError, Operation, TransactionBuilder, TransactionDocument, TRANSACTION_LIMITS,
};
use crate::evidence::EvidenceSet;
use crate::reply::{Answer, Outcome, Refusal};
use crate::transport::{Request, Transport, TransportError};

/// How many `Stale` commits in a row one batch is proposed again after.
const STALE_RETRIES: usize = 8;

/// Bytes a document holds besides its operations: the format, the transaction's id, proposer
/// and schema version, and the keys, with room to spare.
const ENVELOPE_BYTES: usize = 512;

/// Bytes one evidence id adds to the transaction's `evidence` list, with its indentation.
const EVIDENCE_ID_BYTES: usize = 48;

/// The `ekr propose` refusals that are about the document, and so are bisected like a rejection.
const DOCUMENT_REFUSALS: [&str; 2] = [
    "ekr.kernel.StructurallyInvalid",
    "ekr.kernel.ProposalAttribution",
];

/// Commits a consumer's groups of operations in batches, bisecting a rejection.
#[derive(Clone, Debug)]
pub struct Batcher {
    proposer: AgentId,
    operations: usize,
    bytes: usize,
}

/// What a [`Batcher`] did: every committed transaction and every rejected operation.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BatchReport {
    /// Every transaction committed, in the order it was committed.
    pub committed: Vec<CommittedTransaction>,
    /// Every operation of every group that was refused, in group order within its batch.
    pub rejected: Vec<RejectedOperation>,
    /// A refusal of the transaction as a whole, which stopped the run; `None` when the run went
    /// to its end.
    pub refused: Option<RefusedBatch>,
}

/// One committed transaction.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommittedTransaction {
    /// Its id, as `ekr transactions --state Committed` lists it.
    pub transaction: TransactionId,
    /// The revision it published.
    pub revision: u64,
    /// The batch it came from, counted from 0 in the order the groups were packed.
    pub batch: usize,
    /// The groups it carried, by their index in the input, in order.
    pub groups: Vec<usize>,
    /// The ids it was proposed under before, each of whose commits found the head moved.
    pub stale: Vec<TransactionId>,
}

/// One operation of a group that was refused. Groups are atomic, so every operation of a refused
/// group is listed, each with the group's rejection; the kernel's issues name the operation they
/// are about in their message.
#[derive(Clone, Debug, PartialEq)]
pub struct RejectedOperation {
    /// The batch its group was packed into.
    pub batch: usize,
    /// Its group's index in the input.
    pub group: usize,
    /// Its index within the group.
    pub index: usize,
    /// The operation.
    pub operation: Operation,
    /// Why the group, alone, was refused.
    pub rejection: Rejection,
}

/// Why a single group was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejection {
    /// `ekr validate` rejected the transaction holding the group alone, with these issues.
    Rejected {
        /// The transaction that was rejected.
        transaction: TransactionId,
        /// The validators' issues.
        issues: Vec<Issue>,
    },
    /// `ekr propose` refused the document holding the group alone
    /// (`ekr.kernel.StructurallyInvalid`, `ekr.kernel.ProposalAttribution`).
    Refused(Refusal),
    /// The SDK could not build a document of the group alone: it is past a document limit, mixes a
    /// schema change with data, or holds a value the YAML writer refuses.
    Document(String),
}

/// One validator issue of a rejection, as `ekr validate` prints it.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Issue {
    /// The validator that raised it, such as `Type` or `Structural`.
    pub validator: String,
    /// Its stable code, such as `unknown-type` or `alias-already-exists`.
    pub code: String,
    /// What was wrong, naming the operation and the value.
    pub message: String,
}

/// A request that got no answer the SDK can act on.
#[derive(Debug, thiserror::Error)]
pub enum CallError {
    /// The transport returned no reply.
    #[error(transparent)]
    Transport(#[from] TransportError),
    /// The verb answered with something other than the outcome it declares: a refusal that is
    /// not about the document, a usage message or a fault.
    #[error("`{verb}` did not answer with an outcome: {answer:?}")]
    Unanswered {
        /// The verb.
        verb: String,
        /// What it answered.
        answer: Answer,
    },
    /// The verb answered a document this SDK does not read.
    #[error("`{verb}` answered a document this SDK does not read: {document}")]
    Unexpected {
        /// The verb.
        verb: String,
        /// The document.
        document: Json,
    },
    /// A document could not be written.
    #[error(transparent)]
    Document(#[from] DocumentError),
    /// The head moved under every commit of one batch: nine in a row, the first and eight
    /// retries.
    #[error("the head moved under {} commits of one batch in a row: {transactions:?}", transactions.len())]
    StaleRetries {
        /// Every id the batch was proposed under.
        transactions: Vec<TransactionId>,
    },
}

/// A batch that stopped before its end: what was done, and why it stopped.
#[derive(Debug, thiserror::Error)]
#[error(
    "the batch stopped after {} committed transactions{}: {cause}",
    report.committed.len(),
    unknown(outcome_unknown.as_ref())
)]
pub struct BatchError {
    /// What was committed and rejected before it stopped.
    pub report: BatchReport,
    /// The transaction whose `commit` was sent and got no reply, or a fault: it may have been
    /// committed. It is not in `report.committed`; settle it by reading
    /// `ekr transactions --state Committed` for its id.
    pub outcome_unknown: Option<UnknownOutcome>,
    /// Why it stopped.
    #[source]
    pub cause: Box<CallError>,
}

/// A transaction whose `commit` was sent and not answered with an outcome: committed or not, the
/// SDK cannot tell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownOutcome {
    /// Its id.
    pub transaction: TransactionId,
    /// The batch it came from.
    pub batch: usize,
    /// The groups it carried, by their index in the input.
    pub groups: Vec<usize>,
}

/// A refusal of the transaction as a whole rather than of an operation in it: the proposer is not
/// the host operator (`ekr.kernel.ProposalAttribution` naming another submitter), or proposed and
/// validates (`proposer-is-validator`). Every transaction of the run would be refused the same
/// way, so the run stops, and the refusal is named once for every group not committed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefusedBatch {
    /// The batch that was refused.
    pub batch: usize,
    /// Every group of that batch and of every later batch, none of them committed or rejected.
    pub groups: Vec<usize>,
    /// The refusal.
    pub rejection: Rejection,
}

/// The validator issues that concern the transaction as a whole.
const TRANSACTION_ISSUES: [&str; 1] = ["proposer-is-validator"];

/// ` (outcome unknown: <id>, batch …, groups …)`, or nothing.
fn unknown(outcome: Option<&UnknownOutcome>) -> String {
    outcome.map_or_else(String::new, |outcome| {
        format!(
            " (outcome unknown: transaction {}, batch {}, groups {:?})",
            outcome.transaction, outcome.batch, outcome.groups
        )
    })
}

/// How one submission ended.
enum Attempt {
    Committed {
        transaction: TransactionId,
        revision: u64,
        stale: Vec<TransactionId>,
    },
    Refused(Rejection),
    /// Refused as a whole: stop the run.
    Stop(Rejection),
}

/// Where a run is: what it did, the commit it has sent without an answer yet, and the evidence
/// its groups introduce.
struct Run<'a> {
    report: BatchReport,
    in_flight: Option<UnknownOutcome>,
    evidence: &'a mut EvidenceSet,
}

impl Batcher {
    /// A batcher proposing as `proposer`, the host operator, under the kernel's caps.
    #[must_use]
    pub fn new(proposer: AgentId) -> Self {
        Self {
            proposer,
            operations: TRANSACTION_LIMITS.operations,
            bytes: TRANSACTION_LIMITS.input_bytes,
        }
    }

    /// This batcher packing at most `operations` operations and `bytes` bytes of YAML into one
    /// batch. Each is held between 1 and the kernel's cap.
    #[must_use]
    pub fn with_limits(mut self, operations: usize, bytes: usize) -> Self {
        self.operations = operations.clamp(1, TRANSACTION_LIMITS.operations);
        self.bytes = bytes.clamp(1, TRANSACTION_LIMITS.input_bytes);
        self
    }

    /// Commit `groups`, each an atomic dependency group, in order. An empty group is skipped. A
    /// refusal of the transaction as a whole stops the run: [`BatchReport::refused`] names it.
    ///
    /// # Errors
    /// [`BatchError`], boxed, when a request gets no answer the SDK can act on, carrying what was
    /// committed and rejected until then, and the transaction whose commit went unanswered.
    pub fn commit<T: Transport + ?Sized>(
        &self,
        transport: &mut T,
        groups: &[Vec<Operation>],
    ) -> Result<BatchReport, Box<BatchError>> {
        self.commit_with_evidence(transport, groups, &mut EvidenceSet::new(self.proposer))
    }

    /// [`Self::commit`], adding the entries of `evidence` that the groups' assertions cite. Each
    /// transaction proposed carries an entry's `!AddEvidence` in the group of its first assertion
    /// citing it, ahead of that group's operations, unless an earlier transaction committed it:
    /// a group is never submitted without the evidence it introduces, and a group whose entry was
    /// committed before cites the existing id. An entry is marked committed
    /// ([`EvidenceSet::is_committed`]) with the transaction that added it. The report lists each
    /// group's own operations, never the `!AddEvidence` added to it.
    ///
    /// # Errors
    /// As [`Self::commit`]. The entries of [`BatchError::outcome_unknown`]'s transaction are not
    /// marked committed; if it is committed, mark them with [`EvidenceSet::mark_committed`].
    pub fn commit_with_evidence<T: Transport + ?Sized>(
        &self,
        transport: &mut T,
        groups: &[Vec<Operation>],
        evidence: &mut EvidenceSet,
    ) -> Result<BatchReport, Box<BatchError>> {
        let plan = self.plan(groups, evidence);
        let mut run = Run {
            report: BatchReport::default(),
            in_flight: None,
            evidence,
        };
        for (batch, members) in plan.iter().enumerate() {
            match self.submit(transport, groups, members, batch, &mut run) {
                Ok(None) => {}
                Ok(Some(rejection)) => {
                    let done: std::collections::BTreeSet<usize> = run
                        .report
                        .committed
                        .iter()
                        .flat_map(|committed| committed.groups.iter().copied())
                        .chain(run.report.rejected.iter().map(|rejected| rejected.group))
                        .collect();
                    let remaining = plan[batch..]
                        .iter()
                        .flatten()
                        .copied()
                        .filter(|group| !done.contains(group))
                        .collect();
                    run.report.refused = Some(RefusedBatch {
                        batch,
                        groups: remaining,
                        rejection,
                    });
                    break;
                }
                Err(cause) => {
                    return Err(Box::new(BatchError {
                        report: run.report,
                        outcome_unknown: run.in_flight,
                        cause: Box::new(cause),
                    }));
                }
            }
        }
        Ok(run.report)
    }

    /// The groups packed into batches, in order, under the caps. Each group's size counts the
    /// `!AddEvidence` of every entry of `evidence` it is the first to cite.
    fn plan(&self, groups: &[Vec<Operation>], evidence: &EvidenceSet) -> Vec<Vec<usize>> {
        let mut batches = Vec::new();
        let mut current: Vec<usize> = Vec::new();
        let (mut operations, mut bytes, mut schema) = (0, ENVELOPE_BYTES, false);
        let mut introduced = BTreeSet::new();
        for (index, group) in groups.iter().enumerate() {
            if group.is_empty() {
                continue;
            }
            let changes_schema = group.iter().any(Operation::is_schema_change);
            let entries = evidence.introduce(group, &mut introduced);
            let size = estimate(group).saturating_add(if entries.is_empty() {
                0
            } else {
                estimate(&entries)
            });
            let length = group.len() + entries.len();
            let fits = operations + length <= self.operations
                && bytes.saturating_add(size) <= self.bytes
                && changes_schema == schema;
            if !current.is_empty() && !fits {
                batches.push(std::mem::take(&mut current));
                (operations, bytes) = (0, ENVELOPE_BYTES);
            }
            current.push(index);
            operations += length;
            bytes = bytes.saturating_add(size);
            schema = changes_schema;
        }
        if !current.is_empty() {
            batches.push(current);
        }
        batches
    }

    /// Submit `members` as one transaction; if it is refused, each half again, down to one group.
    /// `Some` is a refusal of the transaction as a whole, which stops the run.
    fn submit<T: Transport + ?Sized>(
        &self,
        transport: &mut T,
        groups: &[Vec<Operation>],
        members: &[usize],
        batch: usize,
        run: &mut Run,
    ) -> Result<Option<Rejection>, CallError> {
        let mut attached = BTreeSet::new();
        let built = members
            .iter()
            .flat_map(|&group| {
                let entries = run.evidence.introduce(&groups[group], &mut attached);
                entries.into_iter().chain(groups[group].iter().cloned())
            })
            .fold(
                TransactionBuilder::new(self.proposer),
                TransactionBuilder::push,
            )
            .build();
        let attempt = match built {
            Ok(document) if self.within(&document) => {
                let mut sent = None;
                let attempt = self.attempt(transport, document, &mut sent);
                if let Some(transaction) = sent {
                    run.in_flight = Some(UnknownOutcome {
                        transaction,
                        batch,
                        groups: members.to_vec(),
                    });
                }
                attempt?
            }
            Ok(_) => Attempt::Refused(Rejection::Document(format!(
                "past this batcher's limit of {} bytes",
                self.bytes
            ))),
            Err(error) => Attempt::Refused(Rejection::Document(error.to_string())),
        };
        let report = &mut run.report;
        match attempt {
            Attempt::Committed {
                transaction,
                revision,
                stale,
            } => {
                report.committed.push(CommittedTransaction {
                    transaction,
                    revision,
                    batch,
                    groups: members.to_vec(),
                    stale,
                });
                run.evidence.extend_committed(attached);
            }
            Attempt::Stop(rejection) => return Ok(Some(rejection)),
            Attempt::Refused(_) if members.len() > 1 => {
                let (left, right) = members.split_at(members.len() / 2);
                for half in [left, right] {
                    if let Some(stop) = self.submit(transport, groups, half, batch, run)? {
                        return Ok(Some(stop));
                    }
                }
            }
            Attempt::Refused(rejection) => {
                for &group in members {
                    for (index, operation) in groups[group].iter().enumerate() {
                        report.rejected.push(RejectedOperation {
                            batch,
                            group,
                            index,
                            operation: operation.clone(),
                            rejection: rejection.clone(),
                        });
                    }
                }
            }
        }
        Ok(None)
    }

    /// Whether `document` is within this batcher's byte limit.
    fn within(&self, document: &TransactionDocument) -> bool {
        document
            .to_yaml()
            .is_ok_and(|yaml| yaml.len() <= self.bytes)
    }

    /// Whether `refusal` refuses this batcher's proposer rather than one operation's attribution.
    /// `ekr.kernel.ProposalAttribution` reads "… does not match registered submitter <id>", the
    /// host operator: a proposer other than that id refuses every transaction. When the proposer
    /// is that id, the refusal is an assertion's `proposed_by` or an evidence entry's
    /// `extracted_by`, and is bisected. A reason naming no id is taken as the proposer's.
    fn refuses_proposer(&self, refusal: &Refusal) -> bool {
        refusal.code == "ekr.kernel.ProposalAttribution"
            && refusal
                .reason
                .rsplit(' ')
                .next()
                .and_then(|submitter| submitter.parse::<AgentId>().ok())
                != Some(self.proposer)
    }

    /// Propose, validate and commit `document`, proposing it again under a new id while its
    /// commit finds the head moved. `sent` holds the id of a commit sent and not answered with an
    /// outcome, a refusal or a usage message, which may have been applied.
    fn attempt<T: Transport + ?Sized>(
        &self,
        transport: &mut T,
        mut document: TransactionDocument,
        sent: &mut Option<TransactionId>,
    ) -> Result<Attempt, CallError> {
        let mut stale = Vec::new();
        loop {
            let transaction = document.transaction.id;
            let id = transaction.to_string();
            let propose = Request::new(["propose", "-"]).with_stdin(document.to_yaml()?);
            match call(transport, &propose)? {
                Answer::Outcome(_) => {}
                Answer::Refusal(refusal) if self.refuses_proposer(&refusal) => {
                    return Ok(Attempt::Stop(Rejection::Refused(refusal)))
                }
                Answer::Refusal(refusal) if DOCUMENT_REFUSALS.contains(&refusal.code.as_str()) => {
                    return Ok(Attempt::Refused(Rejection::Refused(refusal)))
                }
                answer => return Err(unanswered(&propose, answer)),
            }
            let validate = Request::new(["validate", id.as_str()]);
            match call(transport, &validate)? {
                Answer::Outcome(Outcome::Validated(_)) => {}
                Answer::Outcome(Outcome::Rejected(record)) => {
                    let issues: Vec<Issue> = record
                        .get("issues")
                        .cloned()
                        .and_then(|issues| serde_json::from_value(issues).ok())
                        .ok_or_else(|| CallError::Unexpected {
                            verb: "validate".to_owned(),
                            document: record.clone(),
                        })?;
                    let whole = issues
                        .iter()
                        .any(|issue| TRANSACTION_ISSUES.contains(&issue.code.as_str()));
                    let rejection = Rejection::Rejected {
                        transaction,
                        issues,
                    };
                    return Ok(if whole {
                        Attempt::Stop(rejection)
                    } else {
                        Attempt::Refused(rejection)
                    });
                }
                answer => return Err(unanswered(&validate, answer)),
            }
            let commit = Request::new(["commit", id.as_str()]);
            *sent = Some(transaction);
            let answer = call(transport, &commit)?;
            if matches!(
                answer,
                Answer::Outcome(Outcome::Stale(_)) | Answer::Refusal(_) | Answer::Usage(_)
            ) {
                // Nothing was applied.
                *sent = None;
            }
            match answer {
                Answer::Outcome(Outcome::Committed(receipt)) => {
                    let revision = receipt["result"]["revision"].as_u64().ok_or_else(|| {
                        CallError::Unexpected {
                            verb: "commit".to_owned(),
                            document: receipt.clone(),
                        }
                    })?;
                    *sent = None;
                    return Ok(Attempt::Committed {
                        transaction,
                        revision,
                        stale,
                    });
                }
                Answer::Outcome(Outcome::Stale(_)) => {
                    stale.push(transaction);
                    if stale.len() > STALE_RETRIES {
                        return Err(CallError::StaleRetries {
                            transactions: stale,
                        });
                    }
                    document.transaction.id = TransactionId::mint();
                }
                answer => return Err(unanswered(&commit, answer)),
            }
        }
    }
}

/// `request`'s reply, typed.
pub(crate) fn call<T: Transport + ?Sized>(
    transport: &mut T,
    request: &Request,
) -> Result<Answer, CallError> {
    Ok(transport.request(request)?.answer())
}

/// The error for `request` answered with `answer`.
pub(crate) fn unanswered(request: &Request, answer: Answer) -> CallError {
    CallError::Unanswered {
        verb: request.verb().to_owned(),
        answer,
    }
}

/// The bytes `group` adds to a transaction document, over-estimated: its operations as YAML with
/// four more bytes of indentation a line, and the evidence ids its assertions add to the
/// manifest. A group the writer refuses is estimated at the whole cap, so it travels alone.
fn estimate(group: &[Operation]) -> usize {
    let Ok(yaml) = to_yaml(group) else {
        return TRANSACTION_LIMITS.input_bytes;
    };
    let evidence: usize = group
        .iter()
        .map(|operation| match operation {
            Operation::AddAssertion(assertion) => assertion.evidence.len(),
            _ => 0,
        })
        .sum();
    yaml.len() + 4 * yaml.lines().count() + EVIDENCE_ID_BYTES * evidence
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{GraphRootId, NodeDraft, TypeId};

    fn node(name: &str) -> Operation {
        NodeDraft::new(GraphRootId::mint(), TypeId::mint(), name).into()
    }

    #[test]
    fn the_kernel_caps_pack_10_000_operations_and_no_more_into_one_batch() {
        let groups: Vec<Vec<Operation>> = (0..10_001).map(|n| vec![node(&n.to_string())]).collect();
        let plan = Batcher::new(AgentId::mint()).plan(&groups, &EvidenceSet::new(AgentId::mint()));
        let sizes: Vec<usize> = plan.iter().map(Vec::len).collect();
        assert_eq!(sizes, [10_000, 1]);
    }

    #[test]
    fn the_kernel_caps_hold_every_batch_under_8_mib() {
        // 24 groups of 20 nodes whose names are 60,000 bytes each: about 28 MiB in all.
        let name = "n".repeat(60_000);
        let groups: Vec<Vec<Operation>> = (0..24)
            .map(|_| (0..20).map(|_| node(&name)).collect())
            .collect();
        let batcher = Batcher::new(AgentId::mint());
        let plan = batcher.plan(&groups, &EvidenceSet::new(AgentId::mint()));
        assert!(plan.len() >= 4, "{} batches", plan.len());
        for members in &plan {
            let document = members
                .iter()
                .flat_map(|&group| groups[group].iter().cloned())
                .fold(
                    TransactionBuilder::new(AgentId::mint()),
                    TransactionBuilder::push,
                )
                .build()
                .expect("every planned batch is within the kernel's limits");
            assert!(document.to_yaml().unwrap().len() <= TRANSACTION_LIMITS.input_bytes);
        }
        let planned: Vec<usize> = plan.concat();
        assert_eq!(planned, (0..24).collect::<Vec<_>>(), "in order, each once");
    }

    #[test]
    fn a_schema_change_never_shares_a_batch_with_data_and_empty_groups_are_skipped() {
        use crate::document::{EdgeWidening, Operation as Op};
        let widening: Op = EdgeWidening::new(TypeId::mint(), [], []).into();
        let groups = vec![
            vec![node("a")],
            vec![widening.clone()],
            vec![],
            vec![widening],
            vec![node("b")],
        ];
        let plan = Batcher::new(AgentId::mint()).plan(&groups, &EvidenceSet::new(AgentId::mint()));
        assert_eq!(plan, [vec![0], vec![1, 3], vec![4]]);
    }

    /// The `!AddEvidence` a group is the first to cite counts toward the caps: 12 groups of one
    /// assertion, each citing its own 4,000-byte item, and every third one citing the previous
    /// group's item too. Without it, the operation cap alone would pack them into three batches.
    #[test]
    fn the_evidence_a_group_introduces_counts_toward_the_caps() {
        use crate::document::{
            Assertion, NodeId, Object, Predicate, PropertyId, Subject, Timestamp, Value,
        };
        use crate::evidence::EvidenceItem;
        let operator = AgentId::mint();
        let mut evidence = EvidenceSet::new(operator);
        let mut previous = None;
        let groups: Vec<Vec<Operation>> = (0..12_u8)
            .map(|n| {
                let cites = evidence.cite(EvidenceItem::new(
                    "a reader",
                    Timestamp::EPOCH,
                    vec![n; 4_000],
                ));
                let mut claim = Assertion::new(
                    GraphRootId::mint(),
                    Subject::Node(NodeId::mint()),
                    Predicate::Property(PropertyId::mint()),
                    Object::Value(Value::String(n.to_string())),
                    operator,
                )
                .citing(cites);
                if n % 3 == 2 {
                    claim = claim.citing(previous.expect("an earlier item"));
                }
                previous = Some(cites);
                vec![claim.into()]
            })
            .collect();
        let (cap_operations, cap_bytes) = (5, 128 * 1024);
        let batcher = Batcher::new(operator).with_limits(cap_operations, cap_bytes);

        let plan = batcher.plan(&groups, &evidence);

        assert!(plan.len() >= 6, "{plan:?}");
        assert_eq!(plan.concat(), (0..12).collect::<Vec<_>>());
        let mut run = evidence.clone();
        for members in &plan {
            let mut attached = BTreeSet::new();
            let document = members
                .iter()
                .flat_map(|&group| {
                    let entries = run.introduce(&groups[group], &mut attached);
                    entries.into_iter().chain(groups[group].iter().cloned())
                })
                .fold(TransactionBuilder::new(operator), TransactionBuilder::push)
                .build()
                .expect("every planned batch is within the kernel's limits");
            assert!(
                document.transaction.operations.len() <= cap_operations,
                "{members:?}"
            );
            assert!(
                document.to_yaml().unwrap().len() <= cap_bytes,
                "{members:?}"
            );
            run.extend_committed(attached);
        }
    }
}
