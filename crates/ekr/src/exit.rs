//! The declared outcome, refusal and fault exit contract of the `ekr` binary.
//!
//! Every declared command outcome — including a recorded Validate rejection and a recorded Commit
//! staleness — is a success and exits 0 with its actual retained record. A named refusal exits 2
//! with its name on stderr and writes nothing. An operational or configuration fault exits 1.

use std::fmt;

use ekr_kernel::{CommitError, DocumentError, PersistenceError, ProjectionError, SeedError};
use ekr_views::ProjectError;

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
        /// Whether the store refused the history the reader holds as diverged from the store at
        /// its path (`PersistenceError::Diverged`); a long-running reader opens it again on this.
        diverged: bool,
        /// Whether the store refused to answer because the SQLite database at its path is no
        /// longer the one it opened (`PersistenceError::Replaced`, `store-replaced`); a
        /// long-running reader opens it again on this too.
        replaced: bool,
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
            diverged: false,
            replaced: false,
        }
    }

    /// What a store's refusal is. A refusal of a stage command, or of a verb joined to a stage, is
    /// a named refusal under its `ekr.store` refusal name (`stage_refusal`). Anything else is a
    /// fault: [`Self::diverged`] for `PersistenceError::Diverged` and [`Self::replaced`] for
    /// `PersistenceError::Replaced`.
    #[must_use]
    pub fn store(error: PersistenceError) -> Self {
        if let Some(name) = stage_refusal(&error) {
            let text = error.to_string();
            let reason = text
                .strip_prefix(name)
                .and_then(|rest| rest.strip_prefix(": "))
                .unwrap_or(&text);
            return Self::refused(name, reason);
        }
        let diverged = matches!(error, PersistenceError::Diverged(_));
        let replaced = matches!(error, PersistenceError::Replaced(_));
        Self::Fault {
            message: error.to_string(),
            diverged,
            replaced,
        }
    }

    /// The fault an `ekr.views` read that could not read the store is: [`Self::diverged`] for
    /// `ProjectError::Diverged` and [`Self::replaced`] for `ProjectError::Replaced`.
    #[must_use]
    pub fn unread(error: ProjectError) -> Self {
        let diverged = matches!(error, ProjectError::Diverged(_));
        let replaced = matches!(error, ProjectError::Replaced(_));
        Self::Fault {
            message: error.to_string(),
            diverged,
            replaced,
        }
    }

    /// Whether this is the fault of a store that refused the history the reader holds as
    /// diverged from the store at its path.
    #[must_use]
    pub fn diverged(&self) -> bool {
        matches!(self, Self::Fault { diverged: true, .. })
    }

    /// Whether this is the fault of a store that refused to answer because the SQLite database at
    /// its path is no longer the one it opened: `store-replaced`.
    #[must_use]
    pub fn replaced(&self) -> bool {
        matches!(self, Self::Fault { replaced: true, .. })
    }

    /// Whether a long-running reader opens the store at its path again on this fault: the store
    /// [`Self::diverged`] or was [`Self::replaced`].
    #[must_use]
    pub fn reopens(&self) -> bool {
        self.diverged() || self.replaced()
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

/// The name a store refuses a stage command, or a verb joined to a stage, under (design § 107;
/// `systems/ekr/domains/store.yaml`, the stage's errors): exit 2, nothing written by the verb.
/// `stage-write-landed` is one too: the write is refused and reported unsuccessful, and its
/// occurrences are in the store only if the stage's publication holds them. A host tenant that
/// carries the stage marker (`stage-tenant-reserved`) is not a refusal of a stage but a
/// host-configuration fault, exit 1, and is not here.
fn stage_refusal(error: &PersistenceError) -> Option<&'static str> {
    Some(match error {
        PersistenceError::StageNotFound(_) => "stage-not-found",
        PersistenceError::StageStateConflict { state, .. } => state.refusal(),
        PersistenceError::StageWriteLanded { .. } => "stage-write-landed",
        PersistenceError::StageHeadMoved { .. } => "stage-head-moved",
        PersistenceError::StageStreamMoved { .. } => "stage-stream-moved",
        PersistenceError::StageObjectMoved { .. } => "stage-object-moved",
        PersistenceError::StageIncomplete(_) => "stage-incomplete",
        PersistenceError::UnresolvedPreparation { .. } => "unresolved-preparation",
        PersistenceError::StageSuffixRefused { .. } => "stage-suffix-refused",
        PersistenceError::StageUnsupportedProvider(_) => "stage-unsupported-provider",
        _ => return None,
    })
}

impl fmt::Display for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Refused { name, message } => write!(formatter, "ekr: {name}: {message}"),
            Self::Fault { message, .. } => write!(formatter, "ekr: {message}"),
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
            SeedError::Store(PersistenceError::ReadOnly(why)) => crate::cli::read_only(&why),
            SeedError::Store(other) => Self::store(other),
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
            // A write that reached a store opened read-only: the refusal a writing open names.
            CommitError::Store(PersistenceError::ReadOnly(why)) => crate::cli::read_only(&why),
            // `Store` is transparent: the store's own refusal, with its text.
            CommitError::Store(error) => Self::store(error),
            error @ CommitError::NotSeeded => Self::fault(error),
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
