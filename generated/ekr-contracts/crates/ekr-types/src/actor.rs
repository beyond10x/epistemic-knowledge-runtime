// generated from ekr v1
// model digest 890aea90d130e26fabda8485a25a78599aa119258635fdea52179df20555f601
// contract digest 281bbd37905ec8f6f636fc68d1767f3895a88fb29b6dbd4e383ae18d0e714c43
// do not edit: regenerate with `ess synthesize`

//! Every actor the specification declares, and the commands each may invoke — as data.
//!
//! A grant is checked against a caller identity, which these types do not read from anywhere:
//! whatever authenticates a request builds a [`Caller`], and a served surface checks it against
//! [`may`] before the command runs. The `PLAN.md` beside this workspace says, per actor,
//! whether a generated surface enforces the grant or the caller does.

/// An actor the specification declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Actor {
    /// `ekr.integrate.Applier`.
    Applier,
    /// `ekr.integrate.HumanReviewer`.
    HumanReviewer,
    /// `ekr.integrate.KnowledgeProposer`.
    KnowledgeProposer,
    /// `ekr.kernel.Committer`.
    Committer,
    /// `ekr.kernel.HumanOperator`.
    HumanOperator,
    /// `ekr.kernel.KnowledgeReader`.
    KnowledgeReader,
    /// `ekr.kernel.Operator`.
    Operator,
    /// `ekr.kernel.Proposer`.
    Proposer,
    /// `ekr.kernel.Validator`.
    Validator,
    /// `ekr.observe.KnowledgeSupplier`.
    KnowledgeSupplier,
    /// `ekr.views.Reader`.
    Reader,
}

impl Actor {
    /// Every declared actor, ordered by qualified name.
    pub const ALL: &'static [Actor] = &[
        Actor::Applier,
        Actor::HumanReviewer,
        Actor::KnowledgeProposer,
        Actor::Committer,
        Actor::HumanOperator,
        Actor::KnowledgeReader,
        Actor::Operator,
        Actor::Proposer,
        Actor::Validator,
        Actor::KnowledgeSupplier,
        Actor::Reader,
    ];

    /// The actor's qualified name, as the specification spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Actor::Applier => "ekr.integrate.Applier",
            Actor::HumanReviewer => "ekr.integrate.HumanReviewer",
            Actor::KnowledgeProposer => "ekr.integrate.KnowledgeProposer",
            Actor::Committer => "ekr.kernel.Committer",
            Actor::HumanOperator => "ekr.kernel.HumanOperator",
            Actor::KnowledgeReader => "ekr.kernel.KnowledgeReader",
            Actor::Operator => "ekr.kernel.Operator",
            Actor::Proposer => "ekr.kernel.Proposer",
            Actor::Validator => "ekr.kernel.Validator",
            Actor::KnowledgeSupplier => "ekr.observe.KnowledgeSupplier",
            Actor::Reader => "ekr.views.Reader",
        }
    }
}

/// The qualified names of the commands `actor` may invoke, ordered by name; empty for an
/// actor that only observes.
pub fn may(actor: Actor) -> &'static [&'static str] {
    match actor {
        Actor::Applier => &[
            "ekr.integrate.ApplyExtraction",
        ],
        Actor::HumanReviewer => &[
            "ekr.integrate.ApproveSchemaProposal",
            "ekr.integrate.RejectSchemaProposal",
        ],
        Actor::KnowledgeProposer => &[
            "ekr.integrate.ApplySchemaProposal",
            "ekr.integrate.DiscoverSchemaGaps",
            "ekr.integrate.ImportInterpretation",
            "ekr.integrate.ListInterpretations",
            "ekr.integrate.ShowInterpretation",
            "ekr.integrate.ShowSchemaProposal",
            "ekr.integrate.SubmitSchemaProposal",
        ],
        Actor::Committer => &[
            "ekr.kernel.Commit",
        ],
        Actor::HumanOperator => &[
            "ekr.kernel.AnswerAttention",
            "ekr.kernel.ApplyUpgrade",
        ],
        Actor::KnowledgeReader => &[
            "ekr.kernel.ListAttention",
            "ekr.kernel.PreviewUpgrade",
            "ekr.kernel.ShowAttention",
        ],
        Actor::Operator => &[
            "ekr.kernel.Explain",
            "ekr.kernel.Seed",
            "ekr.kernel.Snapshot",
        ],
        Actor::Proposer => &[
            "ekr.kernel.Propose",
        ],
        Actor::Validator => &[
            "ekr.kernel.Validate",
        ],
        Actor::KnowledgeSupplier => &[
            "ekr.observe.ImportObservation",
            "ekr.observe.ListObservations",
            "ekr.observe.ShowObservation",
        ],
        Actor::Reader => &[
            "ekr.views.ChangesSince",
            "ekr.views.DescribeNode",
            "ekr.views.DrawFactSample",
            "ekr.views.ExpandNeighbourhood",
            "ekr.views.ExportOcel",
            "ekr.views.FindCodeNames",
            "ekr.views.ProjectGraph",
            "ekr.views.ProjectOverview",
            "ekr.views.ProjectTimeline",
            "ekr.views.ReportFactQuality",
            "ekr.views.ReportStoreQuality",
            "ekr.views.SearchNodes",
        ],
    }
}

/// Who a request was authenticated as.
///
/// Built by whatever authenticates the request — a session, a token, a certificate — and
/// handed to the served surface's `dispatch` and `handle`, which check its grant
/// before the command runs. Never derived from the request itself: a client can write
/// anything into a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Caller {
    /// The declared actor.
    pub actor: Actor,
}

impl Caller {
    /// `true` when this caller may invoke `command`, named by its qualified name.
    pub fn may(&self, command: &str) -> bool {
        may(self.actor).contains(&command)
    }
}
