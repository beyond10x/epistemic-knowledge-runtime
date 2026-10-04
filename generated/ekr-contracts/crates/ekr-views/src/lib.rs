// generated from ekr v1
// model digest ec4199535b14ebf6616330cfd8db6941a1d3c73db52f88162c43c5733c15bf46
// contract digest 080455dbbeca5725f70ef1015fb02efb2db14c0623719d7773754d1965b5cfc7
// do not edit: regenerate with `ess synthesize`

//! ekr-views — the `ekr-views` component of `ekr` v1.
//!
//! Read-only projections of one committed revision: the graph projection ekr.graph-projection/1 a viewer renders from. Writes no canonical state; what it renders is Cache.
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
    /// `ekr.views.GraphProjected`.
    GraphProjected(ekr_types::views::GraphProjected),
}

/// ekr-views — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct EkrViews<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> EkrViews<B> {
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

impl<B> EkrViews<B>
where
    B: ekr_types::views::obligations::ProjectGraphBehavior,
{
    /// Accepts `ekr.views.ProjectGraph`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn project_graph(&mut self, input: ekr_types::views::ProjectGraph) -> Result<ekr_types::views::ProjectGraphOutcome, ekr_types::obligation::UnmetObligation> {
        let outcome = self.behaviors.project_graph(input)?;
        match &outcome {
            ekr_types::views::ProjectGraphOutcome::Projected { graph_projected, .. } => {
                self.outbox.push(PublishedEvent::GraphProjected(graph_projected.clone()));
            }
            ekr_types::views::ProjectGraphOutcome::NotFound { .. } => {}
            ekr_types::views::ProjectGraphOutcome::NotSeeded { .. } => {}
        }
        Ok(outcome)
    }
}
