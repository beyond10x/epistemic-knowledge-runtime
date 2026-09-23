//! `ekr validate`: the kernel's validation handler, run by the host profile's validator.

use ekr_core::{RevisionNumber, Timestamp, TransactionId};
use ekr_kernel::{Runtime, ValidationCommandResult};

use crate::exit::Failure;

/// A recorded rejection is a declared outcome and returns `Ok` like a validation does.
pub(super) fn run(
    runtime: Runtime,
    transaction: TransactionId,
    against: u64,
    now: &dyn Fn() -> Timestamp,
) -> Result<ValidationCommandResult, Failure> {
    Ok(runtime.validate(transaction, RevisionNumber::new(against), now)?)
}
