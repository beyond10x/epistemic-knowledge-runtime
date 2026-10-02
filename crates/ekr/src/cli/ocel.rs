//! `ekr ocel`: the `ekr.ocel/1` document of one revision (`ekr.views.ExportOcel`), through
//! `ekr_views::export_ocel`.

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use ekr_views::OcelError;
use serde_json::Value;

use super::view::project_refusal;
use crate::exit::Failure;

/// The document of `revision`, the head when absent, with the event types `events` names, or the
/// viewer's rule's when it names none. An unseeded store, a revision beyond the head and a name
/// no node type holds are the named refusals `ekr.views.NotSeeded`, `ekr.views.RevisionNotFound`
/// and `ekr.views.EventTypeNotFound` (exit 2); a store that could not be read is a fault.
pub(super) fn run(
    runtime: &Runtime,
    revision: Option<u64>,
    events: &[String],
) -> Result<Value, Failure> {
    let answer = ekr_views::export_ocel(runtime, revision.map(RevisionNumber::new), events)
        .map_err(|error| match error {
            OcelError::EventTypeNotFound { .. } => {
                Failure::refused("ekr.views.EventTypeNotFound", error)
            }
            OcelError::Project(error) => match project_refusal(&error) {
                Some(name) => Failure::refused(name, error),
                None => Failure::unread(error),
            },
        })?;
    serde_json::from_slice(&answer.bytes).map_err(Failure::fault)
}
