//! Read-only inbox queries over the kernel's typed attention projection.
use super::{render, Printed};
use crate::exit::Failure;
use clap::{Subcommand, ValueEnum};
use ekr_core::contract_data::*;
use ekr_kernel::{PersistenceError, Runtime};

/// The source of an inbox question, matching the generated attention subject kind.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum AttentionKind {
    /// Competing canonical assertions.
    Dispute,
    /// A retained interpretation whose integration is blocked.
    BlockedIntegration,
    /// A proposed schema refinement awaiting a decision or application.
    SchemaProposal,
}
/// Read questions and their evidence identities. These verbs never answer a question.
#[derive(Debug, Subcommand)]
pub enum AttentionCommand {
    /// List unresolved questions with evidence and the exact review basis.
    List,
    /// Show a question using the kind and identity returned by list.
    Show {
        /// The kind of retained state from which the question is projected.
        #[arg(value_enum)]
        kind: AttentionKind,
        /// The dispute, blocker or proposal UUID returned by list.
        id: String,
    },
}
fn refusal(error: PersistenceError) -> Failure {
    match error {
        PersistenceError::Document(ref message) if message.starts_with("attention-") => {
            Failure::refused("ekr.kernel.KnowledgeRefused", message)
        }
        other => Failure::store(other),
    }
}
pub(super) fn run(command: AttentionCommand, runtime: &Runtime) -> Result<Printed, Failure> {
    match command {
        AttentionCommand::List => render(&runtime.attention().map_err(refusal)?),
        AttentionCommand::Show { kind, id } => {
            let mut subject = EkrKernelAttentionSubject {
                kind: Box::new(match kind {
                    AttentionKind::Dispute => EkrKernelAttentionKind::V1,
                    AttentionKind::BlockedIntegration => EkrKernelAttentionKind::V0,
                    AttentionKind::SchemaProposal => EkrKernelAttentionKind::V2,
                }),
                dispute_id: EssPresence::Absent,
                blocker_id: EssPresence::Absent,
                proposal_id: EssPresence::Absent,
            };
            match kind {
                AttentionKind::Dispute => {
                    subject.dispute_id = EssPresence::Present(Box::new(EkrKernelDisputeId(id)))
                }
                AttentionKind::BlockedIntegration => {
                    subject.blocker_id =
                        EssPresence::Present(Box::new(EkrIntegrateIntegrationBlockerId(id)))
                }
                AttentionKind::SchemaProposal => {
                    subject.proposal_id =
                        EssPresence::Present(Box::new(EkrIntegrateSchemaProposalId(id)))
                }
            }
            render(&runtime.attention_item(&subject).map_err(refusal)?)
        }
    }
}
