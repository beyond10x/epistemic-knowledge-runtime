//! Observation import and inspection through the kernel's independent retention port.
use std::io::Read;
use std::path::PathBuf;

use clap::Subcommand;
use ekr_core::contract_data::EkrObserveObservationImport;
use ekr_kernel::{PersistenceError, Runtime};

use super::{input, render, Access, Printed};
use crate::exit::Failure;

/// Supplied observation operations. Import writes; list and show only read.
#[derive(Debug, Subcommand)]
pub enum ObserveCommand {
    /// Retain one typed ObservationImport JSON document and its exact base64 payload.
    Import {
        /// Input file, or `-` for stdin. The limit is eight MiB.
        document: PathBuf,
    },
    /// List retained source records, including those with no accepted interpretation.
    List,
    /// Show exact retained bytes and source metadata.
    Show {
        /// The independently retained observation identity.
        observation_id: ekr_core::ObservationId,
    },
}

impl ObserveCommand {
    pub(super) fn access(&self) -> Access {
        match self {
            Self::Import { .. } => Access::Write,
            Self::List | Self::Show { .. } => Access::Read,
        }
    }
}

fn refusal(error: PersistenceError) -> Failure {
    match error {
        PersistenceError::Document(ref message) if message.starts_with("observation-") => {
            Failure::refused("ekr.observe.KnowledgeRefused", message)
        }
        PersistenceError::PublicationInputConflict => {
            Failure::refused("ekr.observe.KnowledgeRefused", error)
        }
        other => Failure::store(other),
    }
}

pub(super) fn run(
    command: ObserveCommand,
    runtime: &Runtime,
    now: &dyn Fn() -> ekr_core::Timestamp,
    stdin: &mut dyn Read,
) -> Result<Printed, Failure> {
    match command {
        ObserveCommand::Import { document } => {
            let mut bytes = Vec::new();
            input::open(&document, stdin)?
                .take(8 * 1024 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(Failure::fault)?;
            if bytes.len() > 8 * 1024 * 1024 {
                return Err(Failure::refused(
                    "ekr.observe.KnowledgeRefused",
                    "observation input exceeds eight MiB",
                ));
            }
            let input: EkrObserveObservationImport = serde_json::from_slice(&bytes)
                .map_err(|e| Failure::refused("ekr.observe.KnowledgeRefused", e))?;
            render(&runtime.import_observation(&input, now()).map_err(refusal)?)
        }
        ObserveCommand::List => render(&runtime.observations().map_err(Failure::store)?),
        ObserveCommand::Show { observation_id } => {
            render(&runtime.observation(observation_id).map_err(refusal)?)
        }
    }
}
