//! Candidate assertion lifecycle checks over the complete unordered operation set.
use super::issue;
use crate::{GraphOperation, GraphTransaction, ValidationIssue, ValidatorName};
use ekr_core::AssertionId;
use ekr_graph::{AssertionLifecycle, Assessment, GraphSnapshot, TemporalRange};
use std::collections::BTreeMap;
#[derive(Clone)]
struct Claim {
    accepted: bool,
    active: bool,
    range: TemporalRange,
    // By id: this validator follows supersession chains through canonical and candidate claims
    // alike, so it reads both sides in the one shape they share.
    lifecycle: AssertionLifecycle<AssertionId>,
}
pub(super) fn check(snapshot: &GraphSnapshot<'_>, tx: &GraphTransaction) -> Vec<ValidationIssue> {
    let mut claims: BTreeMap<AssertionId, Claim> = snapshot
        .graph()
        .assertions
        .iter()
        .map(|(id, a)| {
            (
                *id,
                Claim {
                    accepted: a.assessment.is_accepted(),
                    active: a.is_current(),
                    range: a.valid_time,
                    lifecycle: a.lifecycle.clone().map_assertions(|by| by.id()),
                },
            )
        })
        .collect();
    for op in &tx.operations {
        if let GraphOperation::AddAssertion(a) = op {
            claims.insert(
                a.id,
                Claim {
                    accepted: matches!(a.assessment, Assessment::Proposed),
                    active: matches!(a.lifecycle, AssertionLifecycle::Active),
                    range: a.valid_time,
                    lifecycle: a.lifecycle.clone(),
                },
            );
        }
    }
    let mut issues = Vec::new();
    let mut changes = BTreeMap::new();
    for op in &tx.operations {
        let id = match op {
            GraphOperation::RetractAssertion(r) => r.assertion,
            GraphOperation::SupersedeAssertion(s) => s.assertion,
            _ => continue,
        };
        if changes.insert(id, op).is_some() {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                "conflicting-assertion-lifecycle",
                format!(
                    "assertion {id} has multiple lifecycle changes in one unordered transaction"
                ),
            ));
        }
        if let Some(claim) = claims.get(&id) {
            if !claim.accepted || !claim.active {
                issues.push(issue(
                    tx,
                    ValidatorName::Structural,
                    "assertion-lifecycle-state",
                    format!("assertion {id} must be accepted and active before withdrawal"),
                ));
            }
        }
    }
    let original = claims.clone();
    for (id, op) in &changes {
        if let Some(claim) = claims.get_mut(id) {
            match op {
                GraphOperation::RetractAssertion(r) => {
                    claim.lifecycle = AssertionLifecycle::Retracted {
                        at_revision: snapshot.revision(),
                        reason: r.reason.clone(),
                    }
                }
                GraphOperation::SupersedeAssertion(s) => {
                    claim.range.to = Some(s.effective_from);
                    claim.lifecycle = AssertionLifecycle::Superseded {
                        by: s.by,
                        at_revision: snapshot.revision(),
                        effective_from: s.effective_from,
                    };
                }
                _ => unreachable!(),
            }
        }
    }
    for op in &tx.operations {
        let GraphOperation::SupersedeAssertion(change) = op else {
            continue;
        };
        let Some(old) = original.get(&change.assertion) else {
            continue;
        };
        let Some(replacement) = claims.get(&change.by) else {
            continue;
        };
        let valid = change.assertion != change.by
            && old
                .range
                .from
                .is_none_or(|from| from <= change.effective_from)
            && old.range.to.is_none_or(|to| change.effective_from <= to)
            && replacement.accepted
            && replacement.range.from == Some(change.effective_from)
            && replacement.range.contains(change.effective_from)
            && (matches!(replacement.lifecycle, AssertionLifecycle::Superseded { .. })
                || matches!(replacement.lifecycle, AssertionLifecycle::Active)
                    && replacement.active);
        if !valid {
            issues.push(issue(
                tx,
                ValidatorName::Structural,
                "invalid-supersession",
                format!(
                    "assertion {} has an ineligible replacement {} or valid-time boundary",
                    change.assertion, change.by
                ),
            ));
        }
        let mut seen = std::collections::BTreeSet::new();
        let mut next = change.assertion;
        while let Some(claim) = claims.get(&next) {
            if !seen.insert(next) {
                issues.push(issue(
                    tx,
                    ValidatorName::Structural,
                    "supersession-cycle",
                    format!("assertion {} enters a supersession cycle", change.assertion),
                ));
                break;
            }
            if let AssertionLifecycle::Superseded { by, .. } = claim.lifecycle {
                next = by;
            } else {
                break;
            }
        }
    }
    issues
}
