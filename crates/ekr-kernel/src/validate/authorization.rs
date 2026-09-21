//! Validator 7 of design § 20: a transaction cannot validate itself.
//!
//! Design § 6.10, entire:
//!
//! > The same actor or process that proposes a transaction cannot be the sole basis for its
//! > validation.
//!
//! `systems/ekr/domains/kernel.yaml` declares `ekr.kernel.Proposer` and `ekr.kernel.Validator` as
//! two actors "so that a conformance suite can hold an implementation to the separation". This is
//! that separation, checked: the actor running the pipeline is not the agent that proposed.
//!
//! # Only that, in P1
//!
//! Design amendment 83 adds an outward-write check to this validator — AGENTS.md invariant 6, that
//! a message sent or a page published requires an approval record the transaction cites. Effects
//! arrive in P4 with the `EffectAdapter`, and a check over a kind of operation that does not exist
//! yet would be an unreachable branch nothing can test. The story this file implements says so
//! explicitly.
//!
//! Nor is a capability model here: `ekr.kernel.Agent.capabilities` is a list of strings with no
//! policy reading it, and design § 48's trust profiles are P5.

use ekr_core::AgentId;
use ekr_graph::GraphSnapshot;

use super::{finish, issue, Validator};
use crate::issue::{ValidationIssue, ValidatorName};
use crate::transaction::GraphTransaction;

/// The actor validating is the agent that proposed.
const PROPOSER_IS_VALIDATOR: &str = "proposer-is-validator";

/// Validator 7: authorization.
///
/// The one validator with a field. The snapshot and the transaction between them cannot say who is
/// asking, and § 6.10 is a question about exactly that, so the actor is part of the pipeline rather
/// than part of the proposal — a proposer who could name their own validator would be answering
/// the question the check exists to ask.
/// # What this check is worth, and where the rest of it lives
///
/// It compares an actor the *pipeline* supplies with a `proposer` the *proposal* supplies, and
/// nothing authenticates the second. An agent that writes another agent's id into `tx.proposer`
/// validates its own transaction as itself and this validator says nothing — so what is checked
/// here is the separation of two named identities, not the separation of two actors.
///
/// That gap is not closable in this crate and should not be: the kernel receives no submitter
/// identity, and AGENTS.md invariant 7 wants these validators deterministic and rerunnable without
/// a model or a session. A validator that authenticates is neither. It belongs to whatever builds
/// the submission path — the thing that knows who connected — and is carried by
/// `task:the-proposer-field-is-unauthenticated`, named here so the story that grows that path
/// inherits the obligation instead of rediscovering it.
pub struct Authorization {
    /// The agent running the validation.
    pub actor: AgentId,
}

impl Validator for Authorization {
    fn name(&self) -> ValidatorName {
        ValidatorName::Authorization
    }

    fn validate(
        &self,
        _graph: &GraphSnapshot<'_>,
        tx: &GraphTransaction,
    ) -> Result<(), Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        if self.actor == tx.proposer {
            issues.push(issue(
                tx,
                ValidatorName::Authorization,
                PROPOSER_IS_VALIDATOR,
                format!(
                    "agent {} proposed this transaction and cannot be the basis of its own \
                     validation (design § 6.10)",
                    tx.proposer
                ),
            ));
        }
        finish(issues)
    }
}
