//! `ekr quality`: the `ekr.store-quality/1` document of one revision
//! (`ekr.views.ReportStoreQuality`), through `ekr_views::report_quality`.

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use serde_json::Value;

use super::view::project_refusal;
use crate::exit::Failure;

/// The document of `revision`, the head when absent. An unseeded store and a revision beyond the
/// head are the named refusals `ekr.views.NotSeeded` and `ekr.views.RevisionNotFound` (exit 2),
/// as every `ekr.views` read refuses them; a store that could not be read is a fault.
pub(super) fn run(runtime: &Runtime, revision: Option<u64>) -> Result<Value, Failure> {
    let answer =
        ekr_views::report_quality(runtime, revision.map(RevisionNumber::new)).map_err(|error| {
            match project_refusal(&error) {
                Some(name) => Failure::refused(name, error),
                None => Failure::unread(error),
            }
        })?;
    serde_json::from_slice(&answer.bytes).map_err(Failure::fault)
}
