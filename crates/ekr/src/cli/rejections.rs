//! `ekr rejections`: each rejected transaction whose rejection was validated against a revision in
//! a range, with the issues its retained rejection record holds (`ekr.kernel.RejectionsV1`,
//! `systems/ekr/domains/kernel.yaml`). A read of retained records through
//! `Runtime::transactions`; it adds no record format.

use std::collections::BTreeMap;

use ekr_core::{AgentId, Timestamp, TransactionId};
use ekr_kernel::{RecordedValidationIssue, Runtime, TransactionRecord, TransactionState};
use serde::Serialize;

use crate::exit::Failure;

/// The document's format name.
const FORMAT: &str = "ekr.rejections/1";

/// The `ekr.rejections/1` document: the range as requested, and what it selects.
#[derive(Serialize)]
pub(crate) struct Rejections {
    format: &'static str,
    /// The lowest basis revision selected, as requested; omitted when it was not.
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<u64>,
    /// The highest basis revision selected, as requested; omitted when it was not.
    #[serde(skip_serializing_if = "Option::is_none")]
    to: Option<u64>,
    /// By `against`, then transaction id.
    pub(crate) rejections: Vec<Rejected>,
}

/// One rejected transaction and the issues its rejection record retains, in their order.
#[derive(Serialize)]
pub(crate) struct Rejected {
    pub(crate) transaction_id: TransactionId,
    /// The revision the rejection was validated against: its requested basis.
    pub(crate) against: u64,
    /// The host-attributed submitter, as `ekr transactions` prints it.
    proposer: AgentId,
    rejected_at: Timestamp,
    pub(crate) issues: Vec<RecordedValidationIssue>,
}

/// The rejected transactions of `transactions` whose basis revision lies in `from..=to`, an absent
/// bound unbounded. A transaction in any other state — committed, stale, validated, proposed —
/// has no entry. `transactions` ascends by id, and the sort by `against` is stable, so ties keep
/// that order.
pub(crate) fn select(
    transactions: &BTreeMap<TransactionId, TransactionRecord>,
    from: Option<u64>,
    to: Option<u64>,
) -> Rejections {
    let mut rejections: Vec<Rejected> = transactions
        .iter()
        .filter_map(|(transaction_id, record)| {
            let rejection = match (record.state(), &record.rejection) {
                (TransactionState::Rejected, Some(rejection)) => rejection,
                _ => return None,
            };
            let against = rejection.requested_basis.previous_root.revision.get();
            let inside =
                from.is_none_or(|from| against >= from) && to.is_none_or(|to| against <= to);
            inside.then(|| Rejected {
                transaction_id: *transaction_id,
                against,
                proposer: record.proposal.submitter,
                rejected_at: rejection.rejected_at,
                issues: rejection.issues.clone(),
            })
        })
        .collect();
    rejections.sort_by_key(|rejected| rejected.against);
    Rejections {
        format: FORMAT,
        from,
        to,
        rejections,
    }
}

pub(super) fn run(
    runtime: &Runtime,
    from: Option<u64>,
    to: Option<u64>,
) -> Result<Rejections, Failure> {
    Ok(select(&*runtime.transactions()?, from, to))
}
