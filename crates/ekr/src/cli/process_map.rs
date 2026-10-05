//! `ekr process-map`: the `ekr.process-map/1` document of one revision
//! (`ekr.views.ProjectProcessMap`), through `ekr_views::export_process_map`.

use ekr_core::RevisionNumber;
use ekr_kernel::Runtime;
use ekr_views::OcelError;
use serde_json::Value;

use super::view::project_refusal;
use crate::exit::Failure;

/// The map of the `ekr.ocel/1` log `ekr ocel` prints for the same `revision`, `events` and
/// `event_time`. It is refused as that log is: an unseeded store, a revision beyond the head, a
/// name no node type holds and an event-time selector that selects nothing are the named
/// refusals `ekr.views.NotSeeded`, `ekr.views.RevisionNotFound`, `ekr.views.EventTypeNotFound`
/// and `ekr.views.EventTimeInvalid` (exit 2); a store that could not be read is a fault.
pub(super) fn run(
    runtime: &Runtime,
    revision: Option<u64>,
    events: &[String],
    event_time: &[String],
) -> Result<Value, Failure> {
    let answer = ekr_views::export_process_map(
        runtime,
        revision.map(RevisionNumber::new),
        events,
        event_time,
    )
    .map_err(|error| match error {
        OcelError::EventTypeNotFound { .. } => {
            Failure::refused("ekr.views.EventTypeNotFound", error)
        }
        OcelError::EventTimeInvalid { .. } => Failure::refused("ekr.views.EventTimeInvalid", error),
        OcelError::Project(error) => match project_refusal(&error) {
            Some(name) => Failure::refused(name, error),
            None => Failure::unread(error),
        },
    })?;
    serde_json::from_slice(&answer.bytes).map_err(Failure::fault)
}
