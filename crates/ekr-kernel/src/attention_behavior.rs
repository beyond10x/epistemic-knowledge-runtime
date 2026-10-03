//! Generated attention obligations bound to the actual retained-state projection.
use super::attention::error;
use crate::Commit;
use ekr_core::contract_data as w;
use ekr_core::contracts::kernel::obligations::{ListAttentionBehavior, ShowAttentionBehavior};
use ekr_core::contracts::obligation::UnmetObligation;
use ekr_core::contracts::{graph as g, integrate as i, kernel as m, primitives::Uuid};
use ekr_store::{IncubationRetention, ObjectStore, ObservationRetention, RevisionLog, StoreError};

fn optional<T, R>(value: &w::EssPresence<T>, f: impl FnOnce(&T) -> R) -> Option<R> {
    match value {
        w::EssPresence::Absent => None,
        w::EssPresence::Present(value) => Some(f(value)),
    }
}
fn subject(value: &w::EkrKernelAttentionSubject) -> m::AttentionSubject {
    m::AttentionSubject {
        kind: match *value.kind {
            w::EkrKernelAttentionKind::V0 => m::AttentionKind::BlockedIntegration,
            w::EkrKernelAttentionKind::V1 => m::AttentionKind::Dispute,
            w::EkrKernelAttentionKind::V2 => m::AttentionKind::SchemaProposal,
        },
        dispute_id: optional(&value.dispute_id, |id| m::DisputeId(Uuid(id.0.clone()))),
        blocker_id: optional(&value.blocker_id, |id| {
            i::IntegrationBlockerId(Uuid(id.0.clone()))
        }),
        proposal_id: optional(&value.proposal_id, |id| {
            i::SchemaProposalId(Uuid(id.0.clone()))
        }),
    }
}
fn item(value: &w::EkrKernelAttentionItem) -> Result<m::AttentionItem, StoreError> {
    Ok(m::AttentionItem {
        subject: subject(&value.subject),
        question: value.question.clone(),
        basis: m::ReviewBasis {
            observed_revision: m::RevisionNumber(
                value
                    .basis
                    .observed_revision
                    .0
                    .as_i64()
                    .ok_or_else(|| error("revision exceeds generated command range"))?,
            ),
            evidence_digest: m::ContentHash(value.basis.evidence_digest.0.clone()),
            options_digest: m::ContentHash(value.basis.options_digest.0.clone()),
            effects_digest: m::ContentHash(value.basis.effects_digest.0.clone()),
        },
        claims: value
            .claims
            .iter()
            .map(|id| g::AssertionId(Uuid(id.0.clone())))
            .collect(),
        evidence: value
            .evidence
            .iter()
            .map(|id| g::EvidenceId(Uuid(id.0.clone())))
            .collect(),
        observations: value
            .observations
            .iter()
            .map(|id| g::ObservationId(Uuid(id.0.clone())))
            .collect(),
    })
}
struct Behavior<'a, S: RevisionLog + ObjectStore> {
    commit: &'a Commit<S>,
    fault: Option<StoreError>,
    items: Vec<w::EkrKernelAttentionItem>,
    shown: Option<w::EkrKernelAttentionItem>,
}
impl<'a, S: RevisionLog + ObjectStore> Behavior<'a, S> {
    fn new(commit: &'a Commit<S>) -> Self {
        Self {
            commit,
            fault: None,
            items: Vec::new(),
            shown: None,
        }
    }
    fn failed(&mut self, fault: StoreError, source: &'static str) -> UnmetObligation {
        self.fault = Some(fault);
        UnmetObligation {
            capability: "available retained attention projection",
            source,
        }
    }
    fn finish<T>(&mut self, outcome: Result<T, UnmetObligation>) -> Result<T, StoreError> {
        if let Some(error) = self.fault.take() {
            return Err(error);
        }
        outcome.map_err(error)
    }
}
impl<S: RevisionLog + ObjectStore + ObservationRetention + IncubationRetention>
    ListAttentionBehavior for Behavior<'_, S>
{
    fn list_attention(
        &mut self,
        _: m::ListAttention,
    ) -> Result<m::ListAttentionOutcome, UnmetObligation> {
        match self.commit.project_attention().and_then(|items| {
            let projected = items.iter().map(item).collect::<Result<Vec<_>, _>>()?;
            self.items = items;
            Ok(projected)
        }) {
            Ok(items) => Ok(m::ListAttentionOutcome::Answered {
                list_attention_result: m::ListAttentionResult { items },
            }),
            Err(fault) => Err(self.failed(fault, "ekr.kernel.ListAttention")),
        }
    }
}
impl<S: RevisionLog + ObjectStore + ObservationRetention + IncubationRetention>
    ShowAttentionBehavior for Behavior<'_, S>
{
    fn show_attention(
        &mut self,
        input: m::ShowAttention,
    ) -> Result<m::ShowAttentionOutcome, UnmetObligation> {
        let result = self.commit.project_attention().and_then(|items| {
            let held = items
                .into_iter()
                .find(|item| subject(&item.subject) == input.subject)
                .ok_or_else(|| error("unknown or settled attention subject"))?;
            let projected = item(&held)?;
            self.shown = Some(held);
            Ok(projected)
        });
        match result {
            Ok(item) => Ok(m::ShowAttentionOutcome::Answered {
                show_attention_result: m::ShowAttentionResult { item },
            }),
            Err(fault @ StoreError::Document(_)) => {
                let refusal = m::KnowledgeRefused {
                    code: "attention-refused".into(),
                    reason: fault.to_string(),
                };
                self.fault = Some(fault);
                Ok(m::ShowAttentionOutcome::Refused { error: refusal })
            }
            Err(fault) => Err(self.failed(fault, "ekr.kernel.ShowAttention")),
        }
    }
}
pub(super) fn list<S: RevisionLog + ObjectStore + ObservationRetention + IncubationRetention>(
    commit: &Commit<S>,
) -> Result<Vec<w::EkrKernelAttentionItem>, StoreError> {
    let mut behavior = Behavior::new(commit);
    let outcome = behavior.list_attention(m::ListAttention {});
    behavior.finish(outcome)?;
    Ok(behavior.items)
}
pub(super) fn show<S: RevisionLog + ObjectStore + ObservationRetention + IncubationRetention>(
    commit: &Commit<S>,
    selected: &w::EkrKernelAttentionSubject,
) -> Result<w::EkrKernelAttentionItem, StoreError> {
    let mut behavior = Behavior::new(commit);
    let outcome = behavior.show_attention(m::ShowAttention {
        subject: subject(selected),
    });
    behavior.finish(outcome)?;
    behavior
        .shown
        .ok_or_else(|| error("missing generated attention result"))
}
