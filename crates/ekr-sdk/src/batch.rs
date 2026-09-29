//! Batches that respect the kernel's caps and bisect a rejection (`story:sdk-resolve-and-batch`).
//!
//! A [`Batcher`] commits a consumer's operations, given as atomic dependency groups: a group is
//! never split, and every transaction it proposes holds whole groups. Groups are packed in order
//! into batches under the `ekr.transaction-document/2` caps (10,000 operations, 8 MiB), a schema
//! change never sharing a batch with data. Each batch is proposed, validated and committed; a
//! `Stale` commit is proposed again under a newly minted transaction id; a batch that is rejected
//! is split in two and each half submitted again, down to the single group that is refused. The
//! [`BatchReport`] names every committed transaction and every rejected operation.

use ekr_core::{AgentId, TransactionId};
use serde::Deserialize;
use serde_json::Value as Json;

use crate::document::{
    to_yaml, DocumentError, Operation, TransactionBuilder, TransactionDocument, TRANSACTION_LIMITS,
};
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
#[error("the batch stopped after {} committed transactions: {cause}", report.committed.len())]
pub struct BatchError {
    /// What was committed and rejected before it stopped.
    pub report: BatchReport,
    /// Why it stopped.
    #[source]
    pub cause: Box<CallError>,
}

/// How one submission ended.
enum Attempt {
    Committed {
        transaction: TransactionId,
        revision: u64,
        stale: Vec<TransactionId>,
    },
    Refused(Rejection),
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

    /// Commit `groups`, each an atomic dependency group, in order. An empty group is skipped.
    ///
    /// # Errors
    /// [`BatchError`] when a request gets no answer the SDK can act on, carrying what was
    /// committed and rejected until then.
    pub fn commit<T: Transport + ?Sized>(
        &self,
        transport: &mut T,
        groups: &[Vec<Operation>],
    ) -> Result<BatchReport, BatchError> {
        let mut report = BatchReport::default();
        for (batch, members) in self.plan(groups).iter().enumerate() {
            if let Err(cause) = self.submit(transport, groups, members, batch, &mut report) {
                return Err(BatchError {
                    report,
                    cause: Box::new(cause),
                });
            }
        }
        Ok(report)
    }

    /// The groups packed into batches, in order, under the caps.
    fn plan(&self, groups: &[Vec<Operation>]) -> Vec<Vec<usize>> {
        let mut batches = Vec::new();
        let mut current: Vec<usize> = Vec::new();
        let (mut operations, mut bytes, mut schema) = (0, ENVELOPE_BYTES, false);
        for (index, group) in groups.iter().enumerate() {
            if group.is_empty() {
                continue;
            }
            let changes_schema = group.iter().any(Operation::is_schema_change);
            let size = estimate(group);
            let fits = operations + group.len() <= self.operations
                && bytes.saturating_add(size) <= self.bytes
                && changes_schema == schema;
            if !current.is_empty() && !fits {
                batches.push(std::mem::take(&mut current));
                (operations, bytes) = (0, ENVELOPE_BYTES);
            }
            current.push(index);
            operations += group.len();
            bytes = bytes.saturating_add(size);
            schema = changes_schema;
        }
        if !current.is_empty() {
            batches.push(current);
        }
        batches
    }

    /// Submit `members` as one transaction; if it is refused, each half again, down to one group.
    fn submit<T: Transport + ?Sized>(
        &self,
        transport: &mut T,
        groups: &[Vec<Operation>],
        members: &[usize],
        batch: usize,
        report: &mut BatchReport,
    ) -> Result<(), CallError> {
        let built = members
            .iter()
            .flat_map(|&group| groups[group].iter().cloned())
            .fold(
                TransactionBuilder::new(self.proposer),
                TransactionBuilder::push,
            )
            .build();
        let attempt = match built {
            Ok(document) if self.within(&document) => self.attempt(transport, document)?,
            Ok(_) => Attempt::Refused(Rejection::Document(format!(
                "past this batcher's limit of {} bytes",
                self.bytes
            ))),
            Err(error) => Attempt::Refused(Rejection::Document(error.to_string())),
        };
        match attempt {
            Attempt::Committed {
                transaction,
                revision,
                stale,
            } => report.committed.push(CommittedTransaction {
                transaction,
                revision,
                batch,
                groups: members.to_vec(),
                stale,
            }),
            Attempt::Refused(_) if members.len() > 1 => {
                let (left, right) = members.split_at(members.len() / 2);
                self.submit(transport, groups, left, batch, report)?;
                self.submit(transport, groups, right, batch, report)?;
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
        Ok(())
    }

    /// Whether `document` is within this batcher's byte limit.
    fn within(&self, document: &TransactionDocument) -> bool {
        document
            .to_yaml()
            .is_ok_and(|yaml| yaml.len() <= self.bytes)
    }

    /// Propose, validate and commit `document`, proposing it again under a new id while its
    /// commit finds the head moved.
    fn attempt<T: Transport + ?Sized>(
        &self,
        transport: &mut T,
        mut document: TransactionDocument,
    ) -> Result<Attempt, CallError> {
        let mut stale = Vec::new();
        loop {
            let transaction = document.transaction.id;
            let id = transaction.to_string();
            let propose = Request::new(["propose", "-"]).with_stdin(document.to_yaml()?);
            match call(transport, &propose)? {
                Answer::Outcome(_) => {}
                Answer::Refusal(refusal) if DOCUMENT_REFUSALS.contains(&refusal.code.as_str()) => {
                    return Ok(Attempt::Refused(Rejection::Refused(refusal)))
                }
                answer => return Err(unanswered(&propose, answer)),
            }
            let validate = Request::new(["validate", id.as_str()]);
            match call(transport, &validate)? {
                Answer::Outcome(Outcome::Validated(_)) => {}
                Answer::Outcome(Outcome::Rejected(record)) => {
                    let issues = record
                        .get("issues")
                        .cloned()
                        .and_then(|issues| serde_json::from_value(issues).ok())
                        .ok_or_else(|| CallError::Unexpected {
                            verb: "validate".to_owned(),
                            document: record.clone(),
                        })?;
                    return Ok(Attempt::Refused(Rejection::Rejected {
                        transaction,
                        issues,
                    }));
                }
                answer => return Err(unanswered(&validate, answer)),
            }
            let commit = Request::new(["commit", id.as_str()]);
            match call(transport, &commit)? {
                Answer::Outcome(Outcome::Committed(receipt)) => {
                    let revision = receipt["result"]["revision"].as_u64().ok_or_else(|| {
                        CallError::Unexpected {
                            verb: "commit".to_owned(),
                            document: receipt.clone(),
                        }
                    })?;
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
        let plan = Batcher::new(AgentId::mint()).plan(&groups);
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
        let plan = batcher.plan(&groups);
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
        let plan = Batcher::new(AgentId::mint()).plan(&groups);
        assert_eq!(plan, [vec![0], vec![1, 3], vec![4]]);
    }
}
