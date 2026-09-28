//! `ekr propose`: the kernel's bounded document ingress, submitted as the host operator.
//!
//! The proposer is never taken from the document. The CLI submits as the trusted host operator
//! and the kernel refuses a document that attributes the proposal, or any assertion in it, to
//! anyone else (`task:the-proposer-field-is-unauthenticated`).

use std::io::Read;
use std::ops::Deref;
use std::path::Path;

use ekr_core::{AgentId, Timestamp};
use ekr_kernel::{ProposalRecordV1, Runtime};

use crate::exit::Failure;

/// Opens the document, then the store through `open` — a one-shot verb opens it here, a session
/// hands over the runtime it holds — and hands the unread reader to `Runtime::propose_reader`.
pub(super) fn run<R: Deref<Target = Runtime>>(
    document: &Path,
    stdin: &mut dyn Read,
    open: impl FnOnce() -> Result<R, Failure>,
    operator: AgentId,
    now: &dyn Fn() -> Timestamp,
) -> Result<ProposalRecordV1, Failure> {
    let reader = super::input::open(document, stdin)?;
    Ok(open()?.propose_reader(reader, operator, now)?)
}
