//! Typed schema-learning requests over the shared CLI/session dispatch.
use super::{input, render, Access, Printed};
use crate::exit::Failure;
use clap::Subcommand;
use ekr_core::generated_identity::{Identity, SchemaProposalId};
use ekr_kernel::{PersistenceError, Runtime};
use std::{io::Read, path::PathBuf};

/// Evidence-led schema proposals supplied by a consumer agent.
#[derive(Debug, Subcommand)]
pub enum SchemaProposalCommand {
    /// Group unresolved blockers by kind and declaration with retained source identities.
    Discover,
    /// Retain a SchemaProposalImport envelope and preview its exact mappings.
    Submit {
        /// Input JSON file, or `-` for stdin; limited to eight MiB.
        document: PathBuf,
    },
    /// Show retained proposal bytes and the current material review basis.
    Show {
        /// Stable proposal identity returned by submit.
        #[arg(value_parser = SchemaProposalId::parse_identity)]
        proposal_id: SchemaProposalId,
    },
}

impl SchemaProposalCommand {
    pub(super) fn access(&self) -> Access {
        match self {
            Self::Submit { .. } => Access::Write,
            _ => Access::Read,
        }
    }
}
fn refusal(error: PersistenceError) -> Failure {
    match error {
        PersistenceError::Document(_) | PersistenceError::PublicationInputConflict => {
            Failure::refused("ekr.integrate.KnowledgeRefused", error)
        }
        other => Failure::store(other),
    }
}
pub(super) fn run(
    command: SchemaProposalCommand,
    runtime: &Runtime,
    now: &dyn Fn() -> ekr_core::Timestamp,
    stdin: &mut dyn Read,
) -> Result<Printed, Failure> {
    match command {
        SchemaProposalCommand::Submit { document } => {
            let mut bytes = Vec::new();
            input::open(&document, stdin)?
                .take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(Failure::fault)?;
            if bytes.len() > 8 * 1024 * 1024 {
                return Err(Failure::refused(
                    "ekr.integrate.KnowledgeRefused",
                    "input exceeds eight MiB",
                ));
            }
            let input = serde_json::from_slice(&bytes)
                .map_err(|e| Failure::refused("ekr.integrate.KnowledgeRefused", e))?;
            render(
                &runtime
                    .submit_schema_proposal(&input, now())
                    .map_err(refusal)?,
            )
        }
        SchemaProposalCommand::Show { proposal_id } => {
            render(&runtime.schema_proposal(&proposal_id).map_err(refusal)?)
        }
        SchemaProposalCommand::Discover => {
            render(&runtime.discover_schema_gaps().map_err(|err| match err {
                PersistenceError::Document(message) => {
                    Failure::refused("ekr.integrate.KnowledgeRefused", message)
                }
                other => Failure::store(other),
            })?)
        }
    }
}
