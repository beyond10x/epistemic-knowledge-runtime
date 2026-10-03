// generated from ekr v1
// model digest 7edcefdeed5694edaa4ba44768d7a681da14fe2bb22dac6ba7c2b980c4d7f5eb
// contract digest 1156942f269e6e4ef491bc151de186bdd43b08754f822f8d88caa825382dcd7b
// do not edit: regenerate with `ess synthesize`

//! ekr-store — the `ekr-store` component of `ekr` v1.
//!
//! Persistence through eventlog: content-addressed objects by storage class, and snapshots of committed revisions.
//!
//! The component's outer surface exactly as the specification declares it: accepted commands as
//! handlers, declared views as queries, published events as a typed outbox. The behaviour behind
//! every handler is an implementation obligation — see the `PLAN.md` beside this workspace — and
//! until one is satisfied, its stub answers with a typed refusal naming what is owed.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event this component declares it publishes, on its way to the system's transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishedEvent {
}

/// ekr-store — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct EkrStore<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> EkrStore<B> {
    /// A new port over the given obligation implementations.
    pub fn new(behaviors: B) -> Self {
        Self {
            behaviors,
            outbox: Vec::new(),
        }
    }

    /// Hands over everything published since the last drain, in publication order.
    ///
    /// The system's transport calls this; anything else reading it is taking events the transport
    /// will then never deliver.
    pub fn drain_outbox(&mut self) -> Vec<PublishedEvent> {
        core::mem::take(&mut self.outbox)
    }
}
