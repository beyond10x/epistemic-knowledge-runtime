//! Durable local interpretation import and inspection.
use super::{input, render, Access, Printed};
use crate::exit::Failure;
use clap::Subcommand;
use ekr_core::contract_data::{
    EkrIntegrateInterpretationImport, EkrIntegrateInterpretationVersion,
};
use ekr_core::generated_identity::{Identity, InterpretationId};
use ekr_kernel::{PersistenceError, Runtime};
use std::{io::Read, path::PathBuf};

/// Local interpretation operations; imports retain knowledge without canonical mutation.
#[derive(Debug, Subcommand)]
pub enum IncubateCommand {
    /// Import one InterpretationImport JSON document, limited to eight MiB.
    Import {
        /// Input file, or `-` for stdin.
        document: PathBuf,
    },
    /// List retained immutable document coordinates and digests.
    List,
    /// Show local facts, declarations, blockers and receipts at an exact version.
    Show {
        /// Stable interpretation identity.
        #[arg(value_parser = InterpretationId::parse_identity)]
        interpretation_id: InterpretationId,
        /// Positive document version.
        version: u64,
        /// Digest of the exact document bytes returned by import/list.
        document_digest: ekr_core::ContentHash,
    },
}
impl IncubateCommand {
    pub(super) fn access(&self) -> Access {
        match self {
            Self::Import { .. } => Access::Write,
            Self::List | Self::Show { .. } => Access::Read,
        }
    }
}
fn refusal(error: PersistenceError) -> Failure {
    match error {
        PersistenceError::Document(ref message)
            if message.starts_with("incubation-") || message.starts_with("observation-") =>
        {
            Failure::refused("ekr.integrate.KnowledgeRefused", message)
        }
        PersistenceError::PublicationInputConflict => {
            Failure::refused("ekr.integrate.KnowledgeRefused", error)
        }
        other => Failure::store(other),
    }
}
pub(super) fn run(
    command: IncubateCommand,
    runtime: &Runtime,
    now: &dyn Fn() -> ekr_core::Timestamp,
    stdin: &mut dyn Read,
) -> Result<Printed, Failure> {
    match command {
        IncubateCommand::Import { document } => {
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
            let input: EkrIntegrateInterpretationImport = serde_json::from_slice(&bytes)
                .map_err(|e| Failure::refused("ekr.integrate.KnowledgeRefused", e))?;
            render(
                &runtime
                    .import_interpretation(&input, now())
                    .map_err(refusal)?,
            )
        }
        IncubateCommand::List => render(&runtime.interpretations().map_err(Failure::store)?),
        IncubateCommand::Show {
            interpretation_id,
            version,
            document_digest,
        } => {
            let version = EkrIntegrateInterpretationVersion {
                interpretation_id: Box::new(interpretation_id),
                version: version.into(),
                document_digest: Box::new(ekr_core::contract_data::EkrKernelContentHash(
                    document_digest.to_hex(),
                )),
            };
            render(&runtime.interpretation(&version).map_err(refusal)?)
        }
    }
}
