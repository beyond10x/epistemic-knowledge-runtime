//! `ekr transactions`: every retained transaction, through `Runtime::transactions`.

use clap::ValueEnum;
use ekr_core::{AgentId, Timestamp, TransactionId};
use ekr_kernel::{Runtime, TransactionState};
use serde::Serialize;

use crate::exit::Failure;

/// A lifecycle state to select, by the name `ekr transactions` prints.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
#[value(rename_all = "verbatim")]
pub enum StateFilter {
    /// Awaiting validation.
    Proposed,
    /// Accepted at a retained basis.
    Validated,
    /// Applied to an immutable revision.
    Committed,
    /// Refused with retained deterministic issues.
    Rejected,
    /// The canonical head moved before publication.
    Stale,
}

impl StateFilter {
    /// The filter that selects `state`. No `_` arm: a new kernel state does not compile until
    /// it can be selected.
    const fn of(state: TransactionState) -> Self {
        match state {
            TransactionState::Proposed => Self::Proposed,
            TransactionState::Validated => Self::Validated,
            TransactionState::Committed => Self::Committed,
            TransactionState::Rejected => Self::Rejected,
            TransactionState::Stale => Self::Stale,
        }
    }
}

/// One line of `ekr transactions`.
#[derive(Serialize)]
pub(super) struct Listed {
    transaction_id: TransactionId,
    state: TransactionState,
    /// The host-attributed submitter, which the kernel binds the document's proposer to.
    proposer: AgentId,
    submitted_at: Timestamp,
    operation_count: u64,
}

pub(super) fn run(runtime: &Runtime, state: Option<StateFilter>) -> Result<Vec<Listed>, Failure> {
    Ok(runtime
        .transactions()?
        .into_iter()
        .filter(|(_, record)| state.is_none_or(|wanted| StateFilter::of(record.state()) == wanted))
        .map(|(transaction_id, record)| Listed {
            transaction_id,
            state: record.state(),
            proposer: record.proposal.submitter,
            submitted_at: record.proposal.submitted_at,
            operation_count: record.proposal.operation_count,
        })
        .collect())
}
