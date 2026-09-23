//! `ekr explain`: the kernel's explanation chain over one verified read.

use ekr_core::AssertionId;
use ekr_kernel::{ExplanationResult, Runtime};

use crate::exit::Failure;

/// Captures the newest revision once and explains the assertion from that capture alone.
pub(super) fn run(runtime: Runtime, assertion: AssertionId) -> Result<ExplanationResult, Failure> {
    let read = runtime.read(None)?;
    Ok(read.explain(assertion)?)
}
