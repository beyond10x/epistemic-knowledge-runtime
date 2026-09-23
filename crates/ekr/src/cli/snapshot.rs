//! `ekr snapshot`: the kernel's projection of one verified read.

use ekr_core::{RevisionNumber, Timestamp};
use ekr_kernel::{Runtime, SnapshotResult};

use crate::exit::Failure;

/// Captures the requested (or newest) revision once and projects it; a missing revision is
/// the kernel's `RevisionNotFound`.
pub(super) fn run(
    runtime: Runtime,
    at: Option<u64>,
    valid_at: Option<Timestamp>,
) -> Result<SnapshotResult, Failure> {
    let read = runtime.read(at.map(RevisionNumber::new))?;
    Ok(read.snapshot(valid_at)?)
}
