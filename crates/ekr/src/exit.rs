//! The declared outcome, refusal and fault exit contract of the `ekr` binary.
//!
//! Every declared command outcome — including a recorded Validate rejection and a recorded Commit
//! staleness — is a success and exits 0 with its actual retained record. A named refusal exits 2
//! with its name on stderr and writes nothing. An operational or configuration fault exits 1.

use std::fmt;

use ekr_kernel::{CommitError, DocumentError, PersistenceError, ProjectionError, SeedError};

/// Why a command produced no result.
#[derive(Debug)]
pub enum Failure {
    /// A named refusal from the kernel; nothing was recorded. Exit 2.
    Refused {
        /// The refusal's name: the `ekr.kernel` ESS error where one is declared.
        name: &'static str,
        /// The kernel's own description of the refusal.
        message: String,
    },
    /// A provider, verification, input-access or host-configuration fault. Exit 1.
    Fault {
        /// What failed.
        message: String,
    },
    /// Command-line arguments clap refused; clap's own usage exit status, 2.
    Usage {
        /// Clap's rendered message.
        message: String,
    },
}

impl Failure {
    /// A named refusal.
    #[must_use]
    pub fn refused(name: &'static str, message: impl fmt::Display) -> Self {
        Self::Refused {
            name,
            message: message.to_string(),
        }
    }

    /// An operational or configuration fault.
    #[must_use]
    pub fn fault(message: impl fmt::Display) -> Self {
        Self::Fault {
            message: message.to_string(),
        }
    }

    /// The process exit status this failure carries.
    #[must_use]
    pub fn code(&self) -> u8 {
        match self {
            Self::Refused { .. } | Self::Usage { .. } => 2,
            Self::Fault { .. } => 1,
        }
    }

    /// The refusal's name, if this is a named refusal.
    #[must_use]
    pub fn name(&self) -> Option<&'static str> {
        match self {
            Self::Refused { name, .. } => Some(name),
            _ => None,
        }
    }
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused { name, message } => write!(formatter, "ekr: {name}: {message}"),
            Self::Fault { message } => write!(formatter, "ekr: {message}"),
            Self::Usage { message } => formatter.write_str(message),
        }
    }
}

impl std::error::Error for Failure {}

impl From<SeedError> for Failure {
    /// `ekr.kernel.InvalidSeed` and `ekr.kernel.AlreadySeeded` are refusals; the kernel maps a
    /// valid-but-different retained host anchor to `AlreadySeeded` itself, so every other store
    /// error — corruption included — stays a fault here rather than being hidden as one.
    fn from(error: SeedError) -> Self {
        match error {
            SeedError::Invalid(reason) => Self::refused("ekr.kernel.InvalidSeed", reason),
            SeedError::Store(PersistenceError::AlreadySeeded) => {
                Self::refused("ekr.kernel.AlreadySeeded", PersistenceError::AlreadySeeded)
            }
            SeedError::Store(other) => Self::fault(other),
        }
    }
}

impl From<CommitError> for Failure {
    fn from(error: CommitError) -> Self {
        match error {
            CommitError::Document(DocumentError::Io(io)) => {
                Self::fault(format!("transaction document read failed: {io}"))
            }
            CommitError::Document(document) => {
                Self::refused("ekr.kernel.StructurallyInvalid", document)
            }
            error @ CommitError::ProposalAttribution { .. } => {
                Self::refused("ekr.kernel.ProposalAttribution", error)
            }
            error @ CommitError::RevisionNotFound { .. } => {
                Self::refused("ekr.kernel.RevisionNotFound", error)
            }
            error @ CommitError::TransactionStateConflict { .. } => {
                Self::refused("ekr.kernel.TransactionStateConflict", error)
            }
            error @ CommitError::TransactionNotFound { .. } => {
                Self::refused("ekr.kernel.TransactionNotFound", error)
            }
            error @ (CommitError::NotSeeded | CommitError::Store(_)) => Self::fault(error),
        }
    }
}

impl From<ProjectionError> for Failure {
    /// `ekr.kernel.AssertionNotFound` is the declared refusal; missing or disagreeing retained
    /// support is a verification fault, never a partial result.
    fn from(error: ProjectionError) -> Self {
        match error {
            error @ ProjectionError::AssertionNotFound { .. } => {
                Self::refused("ekr.kernel.AssertionNotFound", error)
            }
            error @ ProjectionError::Unverified { .. } => Self::fault(error),
        }
    }
}
