//! The validation pipeline: design § 20.
//!
//! > The validation pipeline should combine deterministic checks and policy checks. […] The
//! > deterministic subset should be rerunnable independently of AI agents.
//!
//! Seven validators, one module each, named as `ekr.kernel.ValidatorName` names them. Items 8 to
//! 10 of the design's list — contradiction, temporal consistency, policy — arrive with the
//! subsystems that need them, in P3 and P5.
//!
//! # No model runs here
//!
//! AGENTS.md invariant 7: identity, references, types, cardinality, transaction integrity,
//! revision lineage and retention rules are code. Every module below reads a [`GraphSnapshot`] and
//! an [`Ontology`](ekr_ontology::Ontology) and nothing else — no clock, no network, no model — so
//! the same proposal against the same snapshot produces the same issues and the same
//! [`ValidatedTransaction::validation_hash`] on every run, on every machine.
//!
//! # Each validator owns one refusal, and stays off the others
//!
//! The pipeline runs all seven and collects every issue, so a proposal with two defects is
//! reported once for each. That makes *which* validator refused a load-bearing statement, and the
//! cases in `tests/validation.rs` assert it: a transaction carrying exactly one of the five
//! defects of `story:transaction-and-validators` is refused by exactly one validator.
//!
//! Keeping that true costs one rule, applied throughout: **a validator is silent about what
//! another validator has already found unresolvable.** The type validator does not type-check a
//! value whose node does not exist, the cardinality validator does not count properties of a type
//! the ontology does not declare, and the ontology-constraint validator says nothing about an
//! `Invoke` on a node that is not there. The transaction is refused in every one of those cases —
//! by the validator whose question it actually is.

pub mod authorization;
mod candidate;
pub mod cardinality;
mod lifecycle;
pub mod ontology;
pub mod provenance;
pub mod reference;
pub mod structural;
pub mod types;

use std::collections::BTreeMap;

use ekr_core::{AgentId, NodeId, TypeId};
use ekr_graph::GraphSnapshot;

use crate::issue::{ValidationIssue, ValidatorName};
use crate::transaction::{GraphTransaction, ValidatedTransaction};

pub use authorization::Authorization;
pub use cardinality::Cardinality;
pub use ontology::OntologyConstraint;
pub use provenance::Provenance;
pub use reference::Reference;
pub use structural::Structural;
pub use types::Types;

/// One deterministic check over a proposal: design § 20.
///
/// The signature is the design's, with the name the domain gives each implementor added so that a
/// refusal can say which check made it. A validator holds no state that varies between runs; the
/// one that carries a field — [`Authorization`], which carries the actor doing the validating —
/// carries it because § 6.10 is a question about *who* is asking, and the snapshot cannot answer
/// that.
pub trait Validator {
    /// Which of `ekr.kernel.ValidatorName` this is.
    fn name(&self) -> ValidatorName;

    /// Whether the proposal passes this check against that snapshot.
    ///
    /// # Errors
    ///
    /// Every issue this validator found, not only the first: a proposer fixing one defect at a
    /// time and resubmitting pays a round trip per defect.
    fn validate(
        &self,
        graph: &GraphSnapshot<'_>,
        tx: &GraphTransaction,
    ) -> Result<(), Vec<ValidationIssue>>;
}

/// The deterministic pipeline of design § 20, items 1 to 7.
///
/// Built for one actor, because the last of the seven is a question about that actor. It is not
/// `Copy` and not free to build: it holds its validators as trait objects, so the order they run
/// in is data rather than a sequence of calls someone can reorder by accident.
pub struct Pipeline {
    validators: Vec<Box<dyn Validator>>,
}

impl Pipeline {
    /// Bootstrap checks share the deterministic rules without sealing a transaction against a
    /// fictitious predecessor. An empty seed skips only the transaction-specific nonempty rule.
    pub(crate) fn validate_bootstrap(
        &self,
        snapshot: &GraphSnapshot<'_>,
        proposal: &GraphTransaction,
    ) -> Result<(), Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        for validator in &self.validators {
            if proposal.operations.is_empty() && validator.name() == ValidatorName::Structural {
                continue;
            }
            if let Err(raised) = validator.validate(snapshot, proposal) {
                issues.extend(raised);
            }
        }
        finish(issues)
    }
    /// The seven deterministic validators, in the order design § 20 lists them, run by `actor`.
    #[must_use]
    pub fn deterministic(actor: AgentId) -> Self {
        Self {
            validators: vec![
                Box::new(Structural),
                Box::new(Reference),
                Box::new(Types),
                Box::new(Cardinality),
                Box::new(OntologyConstraint),
                Box::new(Provenance),
                Box::new(Authorization { actor }),
            ],
        }
    }

    /// The validators this pipeline runs, in order.
    #[must_use]
    pub fn validators(&self) -> Vec<ValidatorName> {
        self.validators
            .iter()
            .map(|validator| validator.name())
            .collect()
    }

    /// Runs every validator against `snapshot` and seals the proposal if all of them pass.
    ///
    /// Every validator runs, whatever the ones before it said, so the issues a proposer receives
    /// are all of them rather than the first.
    ///
    /// # Errors
    ///
    /// Every [`ValidationIssue`] raised, in the order the validators ran.
    pub fn validate(
        &self,
        snapshot: &GraphSnapshot<'_>,
        proposal: &GraphTransaction,
    ) -> Result<ValidatedTransaction, Vec<ValidationIssue>> {
        // The form that has a content address, built **before** and independently of the
        // validators rather than gated behind them. Gating it was what made its failure arm dead:
        // the type validator refuses exactly the values this conversion refuses, so a gated
        // conversion could only fail if the two walks of that one rule disagreed, and a guard
        // nothing can reach is a guard nobody can check.
        //
        // Ungated, the conversion is reached by every proposal, and a proposal carrying a float
        // reaches its refusal every time. It raises no issue of its own: the type validator has
        // already named the property and the path, which is the message a proposer can act on.
        // `the_two_walks_of_one_rule_agree` in `tests/validate_properties.rs` pins the two
        // answers — verdict and path — over generated values, so `canonical` being `Err` while
        // `issues` is empty is a state that case forbids, and the only thing that can come of it
        // is a refusal.
        let canonical = GraphTransaction::try_from(proposal.clone());

        let mut issues = Vec::new();
        for validator in &self.validators {
            if let Err(raised) = validator.validate(snapshot, proposal) {
                issues.extend(raised);
            }
        }

        match canonical {
            Ok(canonical) if issues.is_empty() => {
                Ok(ValidatedTransaction::seal(canonical, snapshot.revision()))
            }
            _ => Err(issues),
        }
    }
}

/// `Ok` when nothing was raised, and the issues otherwise: the shape every validator ends with.
pub(crate) fn finish(issues: Vec<ValidationIssue>) -> Result<(), Vec<ValidationIssue>> {
    if issues.is_empty() {
        Ok(())
    } else {
        Err(issues)
    }
}

/// A refusal against `proposal`, by `validator`, with `code` and a message a reader can act on.
pub(crate) fn issue(
    proposal: &GraphTransaction,
    validator: ValidatorName,
    code: &str,
    message: String,
) -> ValidationIssue {
    ValidationIssue {
        transaction_id: proposal.id,
        validator,
        code: code.to_owned(),
        message,
    }
}

/// The type of every node the proposal may name: canonical state's, plus the ones it creates.
///
/// A `BTreeMap<NodeId, TypeId>` because that is what `ekr_ontology::NodeTypes` is implemented for,
/// so the type checker can be handed it directly. A node the transaction creates is in it: a
/// transaction that creates a node and then refers to it is ordinary, and a reference validator
/// that only read canonical state would refuse every multi-operation proposal.
pub(crate) fn node_types(
    snapshot: &GraphSnapshot<'_>,
    proposal: &GraphTransaction,
) -> BTreeMap<NodeId, TypeId> {
    candidate::Candidate::of(snapshot, proposal).nodes
}
