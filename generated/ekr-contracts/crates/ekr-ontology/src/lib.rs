// generated from ekr v1
// model digest f356a296c15cb806b72a218ce953f41b3edb6194bcefd41f350d20df79d80073
// contract digest 8d0aad69aca07fb6f5ae41a0ddb7cb31ca9eeb74bcd0c90f225a038d335271f5
// do not edit: regenerate with `ess synthesize`

//! ekr-ontology — the `ekr-ontology` component of `ekr` v1.
//!
//! The type system of a graph as data: schema versions, node and edge types with their properties, per-type lifecycles and named operations. Evolves only through schema transactions the kernel commits.
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

/// ekr-ontology — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct EkrOntology<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> EkrOntology<B> {
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

impl<B> EkrOntology<B>
where
    B: ekr_types::ontology::obligations::NodeTypesByVersionQuery,
{
    /// Serves `ekr.ontology.NodeTypesByVersion` at `read_your_writes` consistency, from the owed projection.
    pub fn node_types_by_version(&self) -> Result<Vec<ekr_types::ontology::NodeTypesByVersion>, ekr_types::obligation::UnmetObligation> {
        self.behaviors.node_types_by_version()
    }
}
