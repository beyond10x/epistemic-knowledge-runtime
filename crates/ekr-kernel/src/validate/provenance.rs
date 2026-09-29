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
//!
//! # Evidence a transaction adds is retained admissible evidence, or it is refused
//!
//! `AddEvidence` (`decision-blocker:evidence-entry-after-seed`, option 1) brings an evidence entry
//! and its payload together, and this validator holds them to the provenance profile
//! `retained-admissible-evidence/1` the seed is held to (design § 91.1), under the same rules:
//!
//! * the payload is the bytes the entry names — `ContentHash::of_bytes(payload)`, the
//!   `ekr.payload.v1` address `ekr hash` prints, equals `content_hash` — or the issue is
//!   `evidence-payload-mismatch`, naming both hashes, as the seed's
//!   `seed-evidence-payload-mismatch` does;
//! * the source is a `HumanStatement`, the one source kind the seed admits and the one
//!   `ekr.kernel.Explain` terminates at, or the issue is `evidence-unsupported-source`, as the
//!   seed's `seed-unsupported-source` is. An `Observation` source waits on a retained observation
//!   (`decision-blocker:observation-retention-path`); a `GraphAssertion` source would name an
//!   assertion by a canonical reference no validator has resolved.
//!
//! That the entry is attributed to the proposer is the proposal's question and is asked there, as
//! it is for an assertion's `proposed_by` (`ekr.kernel.ProposalAttribution`).

use ekr_core::ContentHash;
use ekr_graph::{Assessment, EvidenceSource, GraphSnapshot};

use super::{finish, issue, Validator};
use crate::issue::{ValidationIssue, ValidatorName};
use crate::transaction::{GraphOperation, GraphTransaction};

/// An assertion entering canonical state citing no evidence at all.
const ASSERTION_WITHOUT_EVIDENCE: &str = "assertion-without-evidence";

/// An assertion arriving with the verdict on itself already written in.
const ASSERTION_STATES_ITS_OWN_VERDICT: &str = "assertion-states-its-own-verdict";

/// Evidence added with a payload whose bytes do not hash to the entry's `content_hash`.
const EVIDENCE_PAYLOAD_MISMATCH: &str = "evidence-payload-mismatch";

/// Evidence added from a source other than a `HumanStatement`.
const EVIDENCE_UNSUPPORTED_SOURCE: &str = "evidence-unsupported-source";

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
            if let GraphOperation::AddEvidence(addition) = operation {
                let evidence = &addition.evidence;
                let found = ContentHash::of_bytes(&addition.payload);
                if found != evidence.content_hash {
                    issues.push(issue(
                        tx,
                        ValidatorName::Provenance,
                        EVIDENCE_PAYLOAD_MISMATCH,
                        format!(
                            "evidence {} names content_hash {}, and its payload's bytes hash to \
                             {found} (sha256(\"ekr.payload.v1\" || the payload's bytes), as \
                             `ekr hash` prints it); the entry's content_hash is the address of \
                             exactly its payload",
                            evidence.id, evidence.content_hash
                        ),
                    ));
                }
                if !matches!(evidence.source, EvidenceSource::HumanStatement { .. }) {
                    issues.push(issue(
                        tx,
                        ValidatorName::Provenance,
                        EVIDENCE_UNSUPPORTED_SOURCE,
                        format!(
                            "evidence {} has a {:?} source; evidence is added from a \
                             HumanStatement source only",
                            evidence.id,
                            evidence.source.kind()
                        ),
                    ));
                }
                continue;
            }
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
