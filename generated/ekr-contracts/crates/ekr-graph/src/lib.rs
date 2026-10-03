// generated from ekr v1
// model digest f91b8100f4d52e92766331c01c931d876f7ec64be4fe1d83c6351f9d3f48420b
// contract digest 873319d08748c75f4bb7d3d8f106756f1c677236ed294ead950d1eb8f1df8c48
// do not edit: regenerate with `ess synthesize`

//! ekr-graph — the `ekr-graph` component of `ekr` v1.
//!
//! Graph roots, nodes, edges, bitemporal assertions with their validation state, evidence and observations. Canonical graph changes arrive as committed transactions; independent observations are retained through ekr-store before interpretation (§ 105).
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

/// ekr-graph — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct EkrGraph<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> EkrGraph<B> {
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

impl<B> EkrGraph<B>
where
    B: ekr_types::graph::obligations::AssertionsQuery + ekr_types::graph::obligations::SettledAssertionsQuery,
{
    /// Serves `ekr.graph.Assertions` at `read_your_writes` consistency, from the owed projection.
    pub fn assertions(&self) -> Result<Vec<ekr_types::graph::Assertions>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.assertions()
    }

    /// Serves `ekr.graph.SettledAssertions` at `read_your_writes` consistency, from the owed projection.
    pub fn settled_assertions(&self) -> Result<Vec<ekr_types::graph::SettledAssertions>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.settled_assertions()
    }
}
