//! The structural checks of an `AttachEvidence` (`story:evidence-attaches-to-a-held-assertion`,
//! design § 102.2): the assertion stays current, and the evidence is new to it.
//!
//! Each attachment's assertion is looked up by id. These checks do not join the retraction and
//! supersession check of `lifecycle.rs`, which copies every claim of the graph, so a transaction
//! of a thousand attachments pays a thousand lookups and not the size of the graph. An assertion
//! canonical state does not hold, and evidence nothing retains or adds, are `Reference`'s
//! refusals and are not repeated here.

use std::collections::BTreeSet;

use ekr_core::{AssertionId, EvidenceId};
use ekr_graph::{CanonicalRef, GraphSnapshot};

use super::issue;
use crate::{GraphOperation, GraphTransaction, ValidationIssue, ValidatorName};

/// An attachment to an assertion that is not accepted and active, or that the same transaction
/// retracts or supersedes.
const ASSERTION_NOT_ACTIVE: &str = "assertion-not-active";

/// An attachment of evidence the assertion already cites, already has attached, or that the
/// transaction attaches to it twice.
const EVIDENCE_ALREADY_ATTACHED: &str = "evidence-already-attached";

/// The issues of `tx`'s attachments against `snapshot`, in operation order.
pub(super) fn check(snapshot: &GraphSnapshot<'_>, tx: &GraphTransaction) -> Vec<ValidationIssue> {
    let attachments: Vec<(AssertionId, EvidenceId)> = tx
        .operations
        .iter()
        .filter_map(|operation| match operation {
            GraphOperation::AttachEvidence(attachment) => {
                Some((attachment.assertion, attachment.evidence))
            }
            _ => None,
        })
        .collect();
    if attachments.is_empty() {
        return Vec::new();
    }
    let changed: BTreeSet<AssertionId> = tx
        .operations
        .iter()
        .filter_map(|operation| match operation {
            GraphOperation::RetractAssertion(retraction) => Some(retraction.assertion),
            GraphOperation::SupersedeAssertion(supersession) => Some(supersession.assertion),
            _ => None,
        })
        .collect();
    let state = snapshot.graph();
    let mut issues = Vec::new();
    let mut seen = BTreeSet::new();
    for (assertion, evidence) in attachments {
        let Some(held) = state.assertions.get(&assertion) else {
            continue;
        };
        if !held.is_current() {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                ASSERTION_NOT_ACTIVE,
                format!(
                    "assertion {assertion} is not accepted and active; evidence attaches only to \
                     a current assertion"
                ),
            ));
        } else if changed.contains(&assertion) {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                ASSERTION_NOT_ACTIVE,
                format!(
                    "assertion {assertion} is retracted or superseded by this transaction; \
                     evidence attaches only to an assertion that stays active"
                ),
            ));
        }
        if held.evidence.contains(&CanonicalRef::new(evidence)) {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                EVIDENCE_ALREADY_ATTACHED,
                format!(
                    "assertion {assertion} already cites evidence {evidence}; evidence attaches \
                     to an assertion once"
                ),
            ));
        } else if let Some(attached) = state
            .attached(assertion)
            .find(|attached| attached.evidence.id() == evidence)
        {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                EVIDENCE_ALREADY_ATTACHED,
                format!(
                    "assertion {assertion} already has evidence {evidence} attached, at revision \
                     {}; evidence attaches to an assertion once",
                    attached.revision
                ),
            ));
        } else if !seen.insert((assertion, evidence)) {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                EVIDENCE_ALREADY_ATTACHED,
                format!(
                    "evidence {evidence} is attached to assertion {assertion} twice by this \
                     transaction; evidence attaches to an assertion once"
                ),
            ));
        }
    }
    issues
}
