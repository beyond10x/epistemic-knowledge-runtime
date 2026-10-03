//! Evidence-backed inbox queries and externally reviewed answers.
use super::{input, render, Access, Printed};
use crate::exit::Failure;
use clap::{Subcommand, ValueEnum};
use ekr_core::contract_data::*;
use ekr_kernel::{human_review, CommitError, PersistenceError, Runtime};
use std::io::Read;
use std::path::PathBuf;

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
/// Inspect questions or submit an exact externally signed human answer.
#[derive(Debug, Subcommand)]
pub enum AttentionCommand {
    /// List unresolved questions with evidence and the exact review basis.
    List,
    /// Apply an AttentionAnswerApplication JSON document using independently provisioned trust.
    Answer {
        /// JSON file or `-` for stdin, at most eight MiB; see docs/cli.md.
        document: PathBuf,
    },
    /// Read immutable HumanAnswerRecord history, including settled and unresolved answers.
    History {
        /// Restrict history to this dispute UUID.
        #[arg(long)]
        dispute: Option<ekr_core::AssertionId>,
    },
    /// Show a question using the kind and identity returned by list.
    Show {
        /// The kind of retained state from which the question is projected.
        #[arg(value_enum)]
        kind: AttentionKind,
        /// The dispute, blocker or proposal UUID returned by list.
        id: String,
    },
}
impl AttentionCommand {
    pub(super) fn access(&self) -> Access {
        match self {
            Self::Answer { .. } => Access::Write,
            _ => Access::Read,
        }
    }
}
fn refused(error: impl std::fmt::Display) -> Failure {
    Failure::refused("ekr.kernel.KnowledgeRefused", error)
}
fn failure(error: CommitError) -> Failure {
    match error {
        CommitError::Store(error @ PersistenceError::PublicationInputConflict) => refused(error),
        CommitError::Store(PersistenceError::Document(ref detail))
            if detail.contains("attention-answer:") || detail.contains("answer-") =>
        {
            refused(detail)
        }
        other => other.into(),
    }
}
fn refusal(error: PersistenceError) -> Failure {
    match error {
        PersistenceError::Document(ref message) if message.starts_with("attention-") => {
            Failure::refused("ekr.kernel.KnowledgeRefused", message)
        }
        other => Failure::store(other),
    }
}
pub(super) fn run(
    command: AttentionCommand,
    runtime: &Runtime,
    now: &dyn Fn() -> ekr_core::Timestamp,
    stdin: &mut dyn Read,
) -> Result<Printed, Failure> {
    match command {
        AttentionCommand::Answer { document } => {
            let mut bytes = Vec::new();
            input::open(&document, stdin)?
                .take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(Failure::fault)?;
            if bytes.len() > 8 * 1024 * 1024 {
                return Err(refused("answer input exceeds eight MiB"));
            }
            let wire: EkrKernelAttentionAnswerApplication =
                serde_json::from_slice(&bytes).map_err(refused)?;
            let application = human_review::answer_from_document(&wire)
                .map_err(|e| refused(format!("{}: {}", e.code, e.reason)))?;
            render(
                &runtime
                    .answer_attention(&application, now)
                    .map_err(failure)?,
            )
        }
        AttentionCommand::History { dispute } => {
            let id = dispute.map(|id| {
                ekr_core::contracts::kernel::DisputeId(ekr_core::contracts::primitives::Uuid(
                    id.to_string(),
                ))
            });
            render(&runtime.answer_history(id.as_ref()).map_err(failure)?)
        }
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

#[cfg(test)]
mod tests;
