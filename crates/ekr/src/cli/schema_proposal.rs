//! Typed schema-learning requests over the shared CLI/session dispatch.
use super::{render, Printed};
use crate::exit::Failure;
use clap::Subcommand;
use ekr_kernel::{PersistenceError, Runtime};

/// Read-only discovery for a consumer-supplied schema agent.
#[derive(Debug, Subcommand)]
pub enum SchemaProposalCommand {
    /// Group unresolved blockers by kind and declaration with retained source identities.
    Discover,
}

pub(super) fn run(command: SchemaProposalCommand, runtime: &Runtime) -> Result<Printed, Failure> {
    match command {
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
