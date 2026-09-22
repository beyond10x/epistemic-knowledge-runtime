//! Validator 6 of design § 20: a canonical assertion has sufficient provenance.
//!
//! Design § 6.5, entire:
//!
//! > Every canonical assertion must have sufficient provenance according to policy. "An agent said
//! > so" is not sufficient provenance.
//!
//! P1's policy is the minimum that sentence states and nothing beyond it: **an assertion entering
//! canonical state cites at least one piece of evidence.** `Assertion::proposed_by` is not
//! evidence — it is the agent saying so — and an assertion whose evidence set is empty carries
//! only that.
//!
//! # Deliberately not here
//!
//! * Whether the cited evidence *exists*. That is reference existence, and the reference validator
//!   refuses it; an assertion citing an evidence id nothing holds is refused there, once.
//!
//!   This sentence was written before it was true. The reference validator resolved evidence
//!   against the ids the transaction listed as well as against retained evidence, so between the
//!   two validators an invented id satisfied both — non-empty here, listed there. The division of
//!   labour is real now and `tests/adversary_membrane.rs` is what keeps it: this validator counts,
//!   `Reference` resolves, and neither does the other's half.
//! * Whether the evidence is *good enough* — a confidence floor, a source class, a count that
//!   varies by assertion kind. Design § 48's trust profiles and the policy validator (item 10 of
//!   § 20) are P5, and a threshold invented here would be a policy nothing published.
//!
//! # And one thing that is here, because the reasoning above used to stop halfway
//!
//! This module used to say of the assertion's own
//! [`Assessment`] that reading it "would let a proposer opt out of
//! § 6.5 by labelling the claim `Proposed`" — so the field was ignored, and the evidence rule was
//! applied to every `AddAssertion` whatever state it carried. That much is right and it stays.
//!
//! What it does not address is the other direction, which is the one that matters. `Accepted` is
//! documented by `ekr-graph` as "it crossed the integrity boundary. The agents whose validation it
//! rests on are named", and design § 21 makes that crossing what the canonical core *is*. **This
//! kernel is that boundary.** An assertion arriving already marked `Accepted`, naming validators
//! that never ran, is an agent writing down the answer to the question this pipeline exists to
//! ask — AGENTS.md invariant 4, defeated the way the transaction's evidence list defeated it
//! before: by the proposer saying so.
//!
//! So a proposal states a claim and does not state the verdict on it: an `AddAssertion` carries
//! `Proposed` and nothing else. `Rejected`, `Disputed`, `Superseded` and `Retracted` are refused
//! with `Accepted` rather than separately — each is mintable by the same proposer in the same
//! field, and a rule written against the one state an adversary demonstrated would be a rule with
//! four ways round it. `Retracted` and `Superseded` are also *moves of a committed record*, which
//! design § 36 makes their own operations rather than a field a create can set.

use ekr_graph::{Assessment, GraphSnapshot};

use super::{finish, issue, Validator};
use crate::issue::{ValidationIssue, ValidatorName};
use crate::transaction::{GraphOperation, GraphTransaction};

/// An assertion entering canonical state citing no evidence at all.
const ASSERTION_WITHOUT_EVIDENCE: &str = "assertion-without-evidence";

/// An assertion arriving with the verdict on itself already written in.
const ASSERTION_STATES_ITS_OWN_VERDICT: &str = "assertion-states-its-own-verdict";

/// Validator 6: provenance.
pub struct Provenance;

impl Validator for Provenance {
    fn name(&self) -> ValidatorName {
        ValidatorName::Provenance
    }

    fn validate(
        &self,
        _graph: &GraphSnapshot<'_>,
        tx: &GraphTransaction,
    ) -> Result<(), Vec<ValidationIssue>> {
        let mut issues = Vec::new();
        for operation in &tx.operations {
            let GraphOperation::AddAssertion(assertion) = operation else {
                continue;
            };
            if !matches!(assertion.assessment, Assessment::Proposed)
                || !matches!(assertion.lifecycle, ekr_graph::AssertionLifecycle::Active)
            {
                issues.push(issue(
                    tx,
                    ValidatorName::Provenance,
                    ASSERTION_STATES_ITS_OWN_VERDICT,
                    format!(
                        "assertion {} is proposed as {}, and a proposal states a claim rather than \
                         the verdict on it; only this pipeline moves an assertion out of Proposed",
                        assertion.id,
                        assertion.assessment.name()
                    ),
                ));
            }
            if assertion.evidence.is_empty() {
                issues.push(issue(
                    tx,
                    ValidatorName::Provenance,
                    ASSERTION_WITHOUT_EVIDENCE,
                    format!(
                        "assertion {} cites no evidence; agent {} having proposed it is not \
                         provenance (design § 6.5)",
                        assertion.id, assertion.proposed_by
                    ),
                ));
            }
        }
        finish(issues)
    }
}
