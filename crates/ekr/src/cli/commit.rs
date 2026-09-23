//! `ekr commit`: the kernel's commit handler, as the host operator.

use ekr_core::{AgentId, Timestamp, TransactionId};
use ekr_kernel::{CommitCommandResult, Runtime};

use crate::exit::Failure;

/// A recorded staleness is a declared outcome and returns `Ok` like a commit does. An exact
/// retry returns the original retained receipt without sampling `now`.
pub(super) fn run(
    runtime: Runtime,
    transaction: TransactionId,
    operator: AgentId,
    now: &dyn Fn() -> Timestamp,
) -> Result<CommitCommandResult, Failure> {
    Ok(runtime.commit(transaction, operator, now)?)
}
