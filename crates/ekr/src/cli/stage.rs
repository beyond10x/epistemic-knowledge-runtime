//! `ekr stage`: begin, publish, abandon and list the stages of a store (design § 107; the
//! commands and the view of `ekr.cli`, `systems/ekr/domains/cli.yaml`).
//!
//! Each verb opens the store's own tenant, never a stage's: it ignores `EKR_STAGE`, and `--stage`
//! with it is a usage error (`super::Configured::resolve`). It then calls one kernel command on
//! that runtime — `Runtime::begin_stage`, `Runtime::seal_and_publish_stage`,
//! `Runtime::abandon_stage` — or reads `Runtime::stages`, and prints what it answers. Nothing here
//! copies, seals, publishes or forgets anything itself. `ekr stage publish` is
//! `seal_and_publish_stage`, which reads the stage's record and seals only a Begun stage, so a
//! publish retried after its seal or after its append reaches `PublishStage` alone (spec review
//! B1, design § 107.4).

use clap::Subcommand;
use ekr_core::{RevisionId, RevisionNumber, StageId};
use ekr_kernel::{Runtime, StageListing, StageResult, StageState};
use serde::Serialize;

use crate::exit::Failure;

/// The stage verbs, under `ekr stage`.
#[derive(Debug, Subcommand)]
pub enum StageCommand {
    /// Begin a stage (`ekr.cli.BeginStage`): mint its id, record it Begun at the store's head and
    /// copy the store into the stage's own tenant. Prints the stage: `stage_id` is the id every
    /// verb of the run joins with `EKR_STAGE=<id>` or `--stage <id>`.
    ///
    /// SQLite and PostgreSQL stores only; on a File store it is refused
    /// `stage-unsupported-provider` (exit 2). The store's head and history do not move.
    Begin,
    /// Publish a stage (`ekr.cli.SealStage`, then `ekr.cli.PublishStage`): every revision the run
    /// committed lands in the store at once, or none does. Prints the stage, `Published`, with the
    /// first and last revision it added.
    ///
    /// The store's head must be --expect-head and the stage's base; otherwise it is refused
    /// `stage-head-moved` (exit 2) and nothing changes. A Begun stage is sealed first; a stage
    /// already sealing is published, and one already published with the same --expect-head
    /// answers its original result and forgets what is left of its tenant.
    Publish {
        /// The stage, as `ekr stage begin` printed it.
        stage_id: StageId,
        /// The store's head (`ekr head`, `revision`) the run began from.
        #[arg(long, value_name = "REVISION")]
        expect_head: u64,
    },
    /// Abandon a stage (`ekr.cli.AbandonStage`): record it Abandoned and forget its tenant. The
    /// store's head and history do not move. A stage already abandoned answers its original
    /// result; a published one is refused `stage-already-published` (exit 2).
    Abandon {
        /// The stage, as `ekr stage begin` printed it.
        stage_id: StageId,
    },
    /// List every stage the store has had, in every state (`ekr.cli.Stages`): how a stage whose
    /// `ekr stage begin` answer was lost is found, then joined or abandoned. A File store lists
    /// none.
    List,
}

impl StageCommand {
    /// Whether the verb writes the store: all but `list`.
    pub(super) const fn writes(&self) -> bool {
        !matches!(self, Self::List)
    }
}

/// One stage as `ekr stage list` prints it: a row of the `ekr.cli.Stages` view, field for field.
#[derive(Serialize)]
pub(super) struct Listed {
    stage_id: StageId,
    store: String,
    base: RevisionNumber,
    base_revision: RevisionId,
    published_revisions: Vec<RevisionId>,
    state: StageState,
}

impl From<StageListing> for Listed {
    fn from(listing: StageListing) -> Self {
        let StageListing { stage, state } = listing;
        Self {
            stage_id: stage.stage_id,
            store: stage.store.0,
            base: stage.base,
            base_revision: stage.base_revision,
            published_revisions: stage.published_revisions,
            state,
        }
    }
}

/// What a stage verb prints.
pub(super) enum Answered {
    /// The stage as begin, publish or abandon left it.
    Stage(StageResult),
    /// Every stage of the store.
    Stages(Vec<Listed>),
}

/// Runs `command` on the store's own `runtime`.
pub(super) fn run(runtime: &Runtime, command: StageCommand) -> Result<Answered, Failure> {
    Ok(match command {
        StageCommand::Begin => Answered::Stage(runtime.begin_stage()?),
        StageCommand::Publish {
            stage_id,
            expect_head,
        } => Answered::Stage(
            runtime.seal_and_publish_stage(stage_id, RevisionNumber::new(expect_head))?,
        ),
        StageCommand::Abandon { stage_id } => Answered::Stage(runtime.abandon_stage(stage_id)?),
        StageCommand::List => {
            Answered::Stages(runtime.stages()?.into_iter().map(Listed::from).collect())
        }
    })
}
