//! Kernel wrapper retaining test-only event accounting around the shared bounded loader.

pub(crate) use ekr_core::decode::yaml::{expand, load, Expansion, Past, Tally};
use serde_yaml_ng::observation::{Document, Documents};

/// The next document of `documents`. Under test, the events it buffered are tallied, so a case can
/// count how much of its input the loader read.
pub(crate) fn next<'input>(documents: &mut Documents<'input>) -> Option<Document<'input>> {
    let document = documents.next_document();
    #[cfg(test)]
    if let Some(document) = &document {
        loaded::add(document.event_count());
    }
    document
}

/// The events [`next`] has buffered on this thread, for cases that count the loader's work.
#[cfg(test)]
pub(crate) mod loaded {
    use std::cell::Cell;

    thread_local! {
        static EVENTS: Cell<usize> = const { Cell::new(0) };
    }

    pub(super) fn add(events: usize) {
        EVENTS.with(|count| count.set(count.get().saturating_add(events)));
    }

    /// The tally since the last call, which resets it.
    pub(crate) fn take() -> usize {
        EVENTS.with(|count| count.replace(0))
    }
}
