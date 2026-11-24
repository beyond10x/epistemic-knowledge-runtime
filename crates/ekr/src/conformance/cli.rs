//! The ESS conformance target over the `ekr` component's own commands: the stage commands and
//! `CompareStores` of `ekr.cli` (`systems/ekr/domains/cli.yaml`). Review finding 40 of EKR 0.0.42:
//! no suite held them; `systems/ekr/conformance/cli-suite.json` is that component's suite.
//!
//! [`CliTarget`] runs each command the way the binary does, on an isolated store the
//! [`KernelTarget`] it wraps establishes (the manifest's seed and revision 1, so the head is 1):
//! `BeginStage` is `ekr stage begin`, `AbandonStage` `ekr stage abandon` and `CompareStores`
//! `ekr compare`, each through [`crate::cli::run`]; the view `ekr.cli.Stages` is `ekr stage list`.
//! No verb runs `SealStage` or `PublishStage` alone (`ekr stage publish` runs the one, then the
//! other), so those two run the kernel calls the verb is made of, `Runtime::seal_stage` and
//! `Runtime::publish_stage`, and their refusals are named as the binary names them.
//!
//! * **Providers.** The File provider admits no stage: every stage verb on it is refused
//!   `stage-unsupported-provider` before a stage is read. On File the target runs
//!   `CompareStores` and `BeginStage/unsupported-provider`, and answers every other stage
//!   scenario unsupported. On SQLite it runs every scenario but two: `unsupported-provider`
//!   (the store is not on File) and `PublishStage/object-moved`, whose state needs an object
//!   stored in the store's tenant with no occurrence on its revision stream, which no public
//!   writer of the runtime makes (`crates/ekr-kernel/tests/stage.rs` writes it through the
//!   store's raw object stream). The suite's baseline counts both per provider.
//! * **Events.** The component's own: the stage record's events (`StageBegun`, `StageIndexed`,
//!   `StageSealed`, `StagePublished`, `StageAbandoned`) the store's log gained while the command
//!   ran, read from that log, and `StoresCompared`, counted over the document `ekr compare`
//!   printed. What the kernel logs for a publication (its preparation, the suffix's occurrences)
//!   belongs to `ekr-kernel` and its suite. A write the target makes as another writer while a
//!   command runs is not the command's, and is left out.
//! * **Outcomes.** A command's outcome is what it answered, read against the stage's state the
//!   store's record held before it: a seal of a Sealing stage that answers its result is
//!   `retained-seal`, a publication refused for an abandonment that landed while it ran (the stage
//!   was Sealing before) is `abandoned-meanwhile`, one refused because the stage is Published is
//!   `already-published`.
//! * **External branches.** Each control establishes the state its branch declares through the
//!   runtime's public calls, as `crates/ekr-kernel/tests/stage.rs` establishes it, and never
//!   selects the outcome. A state that arises while the command runs (an abandonment or a proposal
//!   landing between the publication's election and its append, a copy write that fails) is made
//!   at the store's named stage point (`ekr_kernel::on_stage_point`). A branch that acts on a stage
//!   establishes one; the suite names it by a placeholder id, and the target puts the stage it
//!   established in its place. A branch whose stage is not found keeps the placeholder, which no
//!   stage carries.
//! * **The other store.** `CompareStores`'s `other` names a second isolated store, established by
//!   a second [`KernelTarget`] sharing the first's lineages: the same seed and revision 1 for
//!   `compared`, the seed alone for `not-found` (the suite compares at revision 1), and a store
//!   opened and never seeded for `not-seeded`.

use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use ekr_core::{RevisionNumber, StageId};
use ekr_kernel::{
    on_stage_point, CommitError, PersistenceError, Runtime, StageHookGuard, StagePoint,
    StageResult, StageState,
};
use ess_conformance::target::{
    ConformanceTarget, EventObservationRequest, ExternalOutcomeControl, ImplementationIdentity,
    ObservedEvent, RedeliveryRequest, ScenarioContext, SemanticCommandRequest,
    SemanticCommandResult, SemanticViewRequest, SemanticViewResult, TargetError, ViewRow,
};
use ess_primitives::consistency::QueryConsistency;
use ess_primitives::node::Node;

use super::{
    error, event_ref, integer, outcome_ref, response, text_input, token, unavailable, KernelTarget,
    Lineages, Provider,
};
use crate::exit::{Failure, NOT_SEEDED};
use crate::host::CliHostConfigurationV1;

const BEGIN: &str = "ekr.cli.BeginStage";
const SEAL: &str = "ekr.cli.SealStage";
const PUBLISH: &str = "ekr.cli.PublishStage";
const ABANDON: &str = "ekr.cli.AbandonStage";
const COMPARE: &str = "ekr.cli.CompareStores";
const STAGES: &str = "ekr.cli.Stages";

/// The stage record's events, which the `ekr` component publishes.
const STAGE_EVENTS: [&str; 5] = [
    "ekr.store.StageBegun",
    "ekr.store.StageIndexed",
    "ekr.store.StageSealed",
    "ekr.store.StagePublished",
    "ekr.store.StageAbandoned",
];

const FILE_HAS_NO_STAGE: &str = "the File provider admits no stage: every stage verb on it is \
                                 refused `stage-unsupported-provider` before a stage is read";

/// An external branch whose declared state the target establishes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Force {
    UnsupportedProvider,
    NotSeeded,
    BeginUnresolved,
    CopyFailed,
    SealHeadMoved,
    SealNotFound,
    Incomplete,
    SealUnresolved,
    AlreadyPublished,
    AbandonedMeanwhile,
    PublishHeadMoved,
    StreamMoved,
    ObjectMoved,
    PublishUnresolved,
    SuffixRefused,
    PublishNotFound,
    CompareNotFound,
    CompareNotSeeded,
}

impl Force {
    fn of(command: &str, outcome: &str) -> Option<Self> {
        Some(match (command, outcome) {
            (BEGIN, "unsupported-provider") => Self::UnsupportedProvider,
            (BEGIN, "not-seeded") => Self::NotSeeded,
            (BEGIN, "unresolved-preparation") => Self::BeginUnresolved,
            (BEGIN, "copy-failed") => Self::CopyFailed,
            (SEAL, "head-moved") => Self::SealHeadMoved,
            (SEAL, "stage-not-found") => Self::SealNotFound,
            (SEAL, "stage-incomplete") => Self::Incomplete,
            (SEAL, "unresolved-preparation") => Self::SealUnresolved,
            (PUBLISH, "already-published") => Self::AlreadyPublished,
            (PUBLISH, "abandoned-meanwhile") => Self::AbandonedMeanwhile,
            (PUBLISH, "head-moved") => Self::PublishHeadMoved,
            (PUBLISH, "stream-moved") => Self::StreamMoved,
            (PUBLISH, "object-moved") => Self::ObjectMoved,
            (PUBLISH, "unresolved-preparation") => Self::PublishUnresolved,
            (PUBLISH, "suffix-refused") => Self::SuffixRefused,
            (PUBLISH, "stage-not-found") => Self::PublishNotFound,
            (COMPARE, "not-found") => Self::CompareNotFound,
            (COMPARE, "not-seeded") => Self::CompareNotSeeded,
            _ => return None,
        })
    }

    /// Why `provider` cannot establish this branch, if it cannot.
    fn unsupported_on(self, provider: Provider) -> Option<&'static str> {
        match (self, provider) {
            (Self::UnsupportedProvider, Provider::Sqlite) => {
                Some("the branch declares a store on the File provider")
            }
            (Self::ObjectMoved, Provider::Sqlite) => Some(
                "the branch needs an object stored in the store's tenant with no occurrence on \
                 its revision stream, and no public writer of the runtime stores one",
            ),
            (Self::UnsupportedProvider | Self::CompareNotFound | Self::CompareNotSeeded, _)
            | (_, Provider::Sqlite) => None,
            (_, Provider::File) => Some(FILE_HAS_NO_STAGE),
        }
    }
}

/// The conformance target over the `ekr` component's commands on one native provider.
pub struct CliTarget {
    kernel: KernelTarget,
    /// The store `CompareStores`'s `other` names.
    other: KernelTarget,
    /// The external branch the open scenario forces, until its command runs.
    force: Cell<Option<Force>>,
    /// The stage a control established, in place of the stage the suite's input names.
    stand_in: Cell<Option<StageId>>,
    /// Every event this component published in the open scenario.
    observed: RefCell<Vec<ObservedEvent>>,
}

/// What a command answered, and the stage's state the store's record held before it.
struct Answered {
    result: Result<serde_json::Value, Failure>,
    before: Option<StageState>,
}

/// A write another writer makes at a stage point, on the runtime it opened.
type Write = Box<dyn FnOnce(&Runtime) -> Result<(), CommitError>>;

/// A kernel refusal as the store error a stage point hook returns.
fn store_error(error: CommitError) -> PersistenceError {
    match error {
        CommitError::Store(error) => error,
        other => PersistenceError::Backend(other.to_string()),
    }
}

impl CliTarget {
    /// Opens a target over `provider`, reading the kernel target's `manifest.json` and host
    /// documents from `fixtures` and creating isolated stores below `work`: each scenario's own
    /// and the one `CompareStores` compares it with.
    ///
    /// # Errors
    ///
    /// An unreadable or malformed manifest or host document.
    pub fn new(provider: Provider, fixtures: &Path, work: &Path) -> Result<Self, String> {
        let lineages = Lineages::new(&work.join("lineages"));
        let kernel =
            KernelTarget::new(provider, fixtures, &work.join("store"))?.sharing(lineages.clone());
        let other = KernelTarget::new(provider, fixtures, &work.join("other"))?.sharing(lineages);
        Ok(Self {
            kernel,
            other,
            force: Cell::new(None),
            stand_in: Cell::new(None),
            observed: RefCell::new(Vec::new()),
        })
    }

    fn provider(&self) -> Provider {
        self.kernel.provider
    }

    /// `ekr --host <host> --store <store> --backend <provider>`, then `args`, run in process; a
    /// refusal is the command's answer.
    fn answer(&self, args: &[&str]) -> Result<Result<serde_json::Value, Failure>, TargetError> {
        let (backend, store) = store_path(&self.kernel)?;
        let host = self.kernel.fixtures.join(&self.kernel.manifest.host);
        let mut argv: Vec<OsString> = vec![
            "ekr".into(),
            "--host".into(),
            host.into_os_string(),
            "--store".into(),
            store.into_os_string(),
            "--backend".into(),
            backend.into(),
        ];
        argv.extend(args.iter().map(OsString::from));
        match crate::cli::run(argv, &|| self.kernel.tick(), &mut std::io::empty()) {
            Ok(printed) => serde_json::from_str(&printed)
                .map(Ok)
                .map_err(|e| unavailable(&format!("reading `ekr {}`", args.join(" ")), e)),
            Err(failure) => Ok(Err(failure)),
        }
    }

    /// What opens the scenario's store again, as another writer a stage point runs.
    fn opener(
        &self,
    ) -> Result<impl Fn() -> Result<Runtime, PersistenceError> + 'static, TargetError> {
        let (_, store) = store_path(&self.kernel)?;
        let CliHostConfigurationV1 {
            tenant,
            context,
            authority,
            ..
        } = self.kernel.host();
        let provider = self.provider();
        Ok(move || match provider {
            Provider::File => Runtime::file(&store, &tenant, context, authority.clone()),
            Provider::Sqlite => Runtime::sqlite(&store, &tenant, context, authority.clone()),
        })
    }

    /// The proposal fixture the manifest stages as `transaction_document`.
    fn proposal(&self) -> Result<Vec<u8>, TargetError> {
        let name = self
            .kernel
            .manifest
            .documents
            .get("transaction_document")
            .cloned()
            .ok_or_else(|| unavailable("reading a proposal", "no transaction_document"))?;
        self.kernel.fixture(&name)
    }

    fn head(&self) -> Result<RevisionNumber, TargetError> {
        self.kernel
            .runtime()?
            .head()
            .map_err(|e| unavailable("reading the head", e))?
            .map(|root| root.revision)
            .ok_or_else(|| unavailable("reading the head", "the store is not seeded"))
    }

    fn begin(&self) -> Result<StageId, TargetError> {
        self.kernel
            .runtime()?
            .begin_stage()
            .map(|result| result.stage_id)
            .map_err(|e| unavailable("establishing a stage", e))
    }

    fn seal(&self, stage: StageId, head: RevisionNumber) -> Result<(), TargetError> {
        self.kernel
            .runtime()?
            .seal_stage(stage, head)
            .map(|_| ())
            .map_err(|e| unavailable("establishing a sealed stage", e))
    }

    /// One commit on the store's head, through propose, validate and commit.
    fn intervene(&self) -> Result<(), TargetError> {
        let intervening = self.kernel.manifest.setup.intervening.clone();
        self.kernel.establish_commit(&intervening)
    }

    /// A proposal of the fixture through a run joined to `stage`, interrupted at its second
    /// joined write, after its election: the stage holds a decision elected and never published.
    /// `first` runs at the first joined write, after the write's check and before its election.
    fn elect_in(
        &self,
        stage: StageId,
        mut first: impl FnMut() -> Result<(), PersistenceError> + 'static,
    ) -> Result<(), TargetError> {
        let joined = self
            .kernel
            .runtime()?
            .join_stage(stage)
            .map_err(|e| unavailable("joining a stage", e))?;
        let bytes = self.proposal()?;
        let operator = self.kernel.host().context.operator;
        let mut writes = 0;
        let hook = on_stage_point(move |point| {
            if point == StagePoint::JoinedWrite {
                writes += 1;
                match writes {
                    1 => first()?,
                    2 => {
                        return Err(PersistenceError::Backend(
                            "interrupted after the election".into(),
                        ))
                    }
                    _ => {}
                }
            }
            Ok(())
        });
        let proposed = joined.propose_reader(bytes.as_slice(), operator, || self.kernel.tick());
        drop(hook);
        match proposed {
            Err(_) => Ok(()),
            Ok(record) => Err(unavailable(
                "electing a decision in a stage",
                format!("the proposal {} was published", record.transaction_id),
            )),
        }
    }

    /// Establishes the state `force` declares before its command runs, and the stage it acts on.
    fn establish(&self, force: Force) -> Result<Option<StageId>, TargetError> {
        if matches!(
            force,
            Force::UnsupportedProvider
                | Force::NotSeeded
                | Force::CopyFailed
                | Force::SealNotFound
                | Force::PublishNotFound
                | Force::ObjectMoved
                | Force::CompareNotFound
                | Force::CompareNotSeeded
        ) {
            return Ok(None);
        }
        let base = self.head()?;
        Ok(match force {
            Force::BeginUnresolved => {
                // A Sealing stage whose publication was elected and interrupted before its append.
                let stage = self.begin()?;
                self.seal(stage, base)?;
                let hook = on_stage_point(|point| {
                    if point == StagePoint::PublishElected {
                        Err(PersistenceError::Backend(
                            "interrupted after the election".into(),
                        ))
                    } else {
                        Ok(())
                    }
                });
                let published = self.kernel.runtime()?.publish_stage(stage, base);
                drop(hook);
                if published.is_ok() {
                    return Err(unavailable(
                        "establishing an unresolved publication",
                        "the publication landed",
                    ));
                }
                None
            }
            Force::SealHeadMoved => {
                let stage = self.begin()?;
                self.intervene()?;
                Some(stage)
            }
            Force::Incomplete => {
                // A begin whose copy, and then its own abandonment, could not write: its record
                // landed, and no completion receipt of its copy.
                let mut groups = 0;
                let hook = on_stage_point(move |point| {
                    if let StagePoint::Group { .. } = point {
                        groups += 1;
                        if groups > 1 {
                            return Err(PersistenceError::Backend("interrupted begin".into()));
                        }
                    }
                    Ok(())
                });
                let begun = self.kernel.runtime()?.begin_stage();
                drop(hook);
                match begun {
                    Err(CommitError::Store(PersistenceError::StageCopyFailed {
                        stage_id,
                        state: StageState::Begun,
                        ..
                    })) => Some(stage_id),
                    other => {
                        return Err(unavailable(
                            "establishing an incomplete stage",
                            format!("the interrupted begin answered {other:?}"),
                        ))
                    }
                }
            }
            Force::SealUnresolved => {
                let stage = self.begin()?;
                self.elect_in(stage, || Ok(()))?;
                Some(stage)
            }
            Force::AlreadyPublished => {
                // Published at head 2, so the suite's expected head 1 is another expected head.
                self.intervene()?;
                let moved = self.head()?;
                let stage = self.begin()?;
                self.seal(stage, moved)?;
                self.kernel
                    .runtime()?
                    .publish_stage(stage, moved)
                    .map_err(|e| unavailable("establishing a published stage", e))?;
                Some(stage)
            }
            Force::AbandonedMeanwhile => {
                let stage = self.begin()?;
                self.seal(stage, base)?;
                Some(stage)
            }
            Force::StreamMoved => {
                // A run that committed in the stage: its publication appends to the revision
                // stream, so a proposal landing there first moves the stream it captured.
                let stage = self.begin()?;
                self.run_in(stage, base)?;
                self.seal(stage, base)?;
                Some(stage)
            }
            Force::PublishHeadMoved => {
                let stage = self.begin()?;
                self.seal(stage, base)?;
                self.intervene()?;
                Some(stage)
            }
            Force::PublishUnresolved => {
                // A joined write checked while the stage was Begun; the stage is sealed before it
                // elects, and it is interrupted after its election.
                let stage = self.begin()?;
                let open = self.opener()?;
                self.elect_in(stage, move || {
                    open()?
                        .seal_stage(stage, base)
                        .map(|_| ())
                        .map_err(store_error)
                })?;
                Some(stage)
            }
            Force::SuffixRefused => {
                // The store holds a proposal at the base; the run validates and commits it in the
                // stage, and another writer validates it in the store.
                let runtime = self.kernel.runtime()?;
                let operator = self.kernel.host().context.operator;
                let pending = runtime
                    .propose_reader(self.proposal()?.as_slice(), operator, || self.kernel.tick())
                    .map_err(|e| unavailable("establishing a pending proposal", e))?
                    .transaction_id;
                let stage = self.begin()?;
                let joined = runtime
                    .join_stage(stage)
                    .map_err(|e| unavailable("joining a stage", e))?;
                joined
                    .validate(pending, base, || self.kernel.tick())
                    .map_err(|e| unavailable("validating in the stage", e))?;
                joined
                    .commit(pending, operator, || self.kernel.tick())
                    .map_err(|e| unavailable("committing in the stage", e))?;
                drop(joined);
                runtime
                    .validate(pending, base, || self.kernel.tick())
                    .map_err(|e| unavailable("validating in the store", e))?;
                self.seal(stage, base)?;
                Some(stage)
            }
            Force::UnsupportedProvider
            | Force::NotSeeded
            | Force::CopyFailed
            | Force::SealNotFound
            | Force::PublishNotFound
            | Force::ObjectMoved
            | Force::CompareNotFound
            | Force::CompareNotSeeded => None,
        })
    }

    /// One commit of the proposal fixture through a run joined to `stage`, against `base`.
    fn run_in(&self, stage: StageId, base: RevisionNumber) -> Result<(), TargetError> {
        let joined = self
            .kernel
            .runtime()?
            .join_stage(stage)
            .map_err(|e| unavailable("joining a stage", e))?;
        let operator = self.kernel.host().context.operator;
        let proposed = joined
            .propose_reader(self.proposal()?.as_slice(), operator, || self.kernel.tick())
            .map_err(|e| unavailable("proposing in the stage", e))?
            .transaction_id;
        joined
            .validate(proposed, base, || self.kernel.tick())
            .map_err(|e| unavailable("validating in the stage", e))?;
        joined
            .commit(proposed, operator, || self.kernel.tick())
            .map(|_| ())
            .map_err(|e| unavailable("committing in the stage", e))
    }

    /// The stage the request names, or the one a control established in its place.
    fn stage_input(&self, request: &SemanticCommandRequest) -> Result<StageId, TargetError> {
        if let Some(stage) = self.stand_in.get() {
            return Ok(stage);
        }
        text_input(request, "stage_id")?
            .parse()
            .map_err(|e| unavailable("reading `stage_id`", format!("{e:?}")))
    }

    /// The optional revision number `field`, as the exact decimal integer the CLI reads.
    fn revision_input(
        request: &SemanticCommandRequest,
        field: &str,
    ) -> Result<Option<u64>, TargetError> {
        match request.input.get(field) {
            None | Some(Node::Null) => Ok(None),
            Some(Node::Number(number)) => {
                number.exact_text().parse().map(Some).map_err(|_| {
                    unavailable(&format!("reading `{field}`"), "not a revision number")
                })
            }
            Some(other) => Err(unavailable(
                &format!("reading `{field}`"),
                format!("expected an integer, found {}", other.type_name()),
            )),
        }
    }

    fn expect_head(request: &SemanticCommandRequest) -> Result<RevisionNumber, TargetError> {
        Self::revision_input(request, "expect_head")?
            .map(RevisionNumber::new)
            .ok_or_else(|| unavailable("reading `expect_head`", "absent"))
    }

    fn state_of(&self, stage: StageId) -> Result<Option<StageState>, TargetError> {
        Ok(self
            .kernel
            .runtime()?
            .stages()
            .map_err(|e| unavailable("reading the stages", e))?
            .into_iter()
            .find(|listing| listing.stage.stage_id == stage)
            .map(|listing| listing.state))
    }

    /// A stage point hook that runs `write` once, as another writer of the store, when the
    /// publication has elected its attempt and not yet appended, and adds the log positions that
    /// write took to `foreign`.
    fn at_election(
        &self,
        foreign: &Rc<RefCell<BTreeSet<u64>>>,
        write: Write,
    ) -> Result<StageHookGuard, TargetError> {
        let open = self.opener()?;
        let foreign = Rc::clone(foreign);
        let mut write = Some(write);
        Ok(on_stage_point(move |point| {
            if point != StagePoint::PublishElected {
                return Ok(());
            }
            let Some(write) = write.take() else {
                return Ok(());
            };
            let other = open()?;
            let logged = other.published_events()?.len();
            write(&other).map_err(store_error)?;
            foreign.borrow_mut().extend(
                other
                    .published_events()?
                    .iter()
                    .skip(logged)
                    .map(|event| event.position),
            );
            Ok(())
        }))
    }

    /// Runs `command`, with the stage points `force` makes while it runs.
    fn run_command(
        &self,
        command: &str,
        request: &SemanticCommandRequest,
        force: Option<Force>,
        foreign: &Rc<RefCell<BTreeSet<u64>>>,
    ) -> Result<Answered, TargetError> {
        let stage_result = |result: Result<StageResult, CommitError>| {
            result.map_err(Failure::from).and_then(|result| {
                serde_json::to_value(result).map_err(|e| Failure::Fault {
                    message: e.to_string(),
                    diverged: false,
                    replaced: false,
                })
            })
        };
        Ok(match command {
            BEGIN => {
                let _hook = (force == Some(Force::CopyFailed)).then(|| {
                    // The stage's record lands; the first write of its copy is refused.
                    let mut groups = 0;
                    on_stage_point(move |point| {
                        if let StagePoint::Group { .. } = point {
                            groups += 1;
                            if groups == 2 {
                                return Err(PersistenceError::Backend(
                                    "the copy's write was refused".into(),
                                ));
                            }
                        }
                        Ok(())
                    })
                });
                Answered {
                    result: self.answer(&["stage", "begin"])?,
                    before: None,
                }
            }
            SEAL => {
                let stage = self.stage_input(request)?;
                let before = self.state_of(stage)?;
                let expect_head = Self::expect_head(request)?;
                Answered {
                    result: stage_result(self.kernel.runtime()?.seal_stage(stage, expect_head)),
                    before,
                }
            }
            PUBLISH => {
                let stage = self.stage_input(request)?;
                let before = self.state_of(stage)?;
                let expect_head = Self::expect_head(request)?;
                let _hook = match force {
                    Some(Force::AbandonedMeanwhile) => Some(self.at_election(
                        foreign,
                        Box::new(move |other| other.abandon_stage(stage).map(|_| ())),
                    )?),
                    Some(Force::StreamMoved) => {
                        let bytes = self.proposal()?;
                        let operator = self.kernel.host().context.operator;
                        let at = self.kernel.tick();
                        Some(self.at_election(
                            foreign,
                            Box::new(move |other| {
                                other
                                    .propose_reader(bytes.as_slice(), operator, || at)
                                    .map(|_| ())
                            }),
                        )?)
                    }
                    _ => None,
                };
                Answered {
                    result: stage_result(self.kernel.runtime()?.publish_stage(stage, expect_head)),
                    before,
                }
            }
            ABANDON => {
                let stage = self.stage_input(request)?;
                let before = self.state_of(stage)?;
                let id = stage.to_string();
                Answered {
                    result: self.answer(&["stage", "abandon", &id])?,
                    before,
                }
            }
            _ => {
                let (_, other) = store_path(&self.other)?;
                let mut args: Vec<String> = vec![
                    "compare".into(),
                    "--other".into(),
                    other.to_string_lossy().into_owned(),
                ];
                for (field, flag) in [("at", "--at"), ("other_at", "--other-at")] {
                    if let Some(at) = Self::revision_input(request, field)? {
                        args.push(flag.into());
                        args.push(at.to_string());
                    }
                }
                let args: Vec<&str> = args.iter().map(String::as_str).collect();
                Answered {
                    result: self.answer(&args)?,
                    before: None,
                }
            }
        })
    }

    /// Establishes the lineage, then the state the forced branch declares.
    fn arrange(&self, command: &str, force: Option<Force>) -> Result<(), TargetError> {
        if force == Some(Force::NotSeeded) {
            // A store opened and never seeded.
            if let Some(scenario) = self.kernel.scenario.borrow_mut().as_mut() {
                scenario.prepared = true;
            }
            self.kernel.runtime()?;
        } else {
            self.kernel.prepare(command)?;
        }
        if let Some(force) = force {
            if let Some(stage) = self.establish(force)? {
                self.stand_in.set(Some(stage));
            }
        }
        if command == COMPARE {
            match force {
                Some(Force::CompareNotFound) => {
                    let seed = self.other.manifest.setup.seed.clone();
                    self.other.establish_seed(&seed)?;
                }
                Some(Force::CompareNotSeeded) => {
                    self.other.runtime()?;
                }
                _ => self.other.prepare(command)?,
            }
        }
        Ok(())
    }

    fn execute(
        &self,
        request: &SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        let command = request.command.to_string();
        if ![BEGIN, SEAL, PUBLISH, ABANDON, COMPARE].contains(&command.as_str()) {
            return Err(TargetError::unsupported(
                format!("the command `{command}`"),
                "the CLI target answers only the `ekr` component's commands",
            ));
        }
        let force = self.force.take();
        if self.provider() == Provider::File && command != COMPARE && force.is_none() {
            return Err(TargetError::unsupported(
                format!("`{command}` on the File provider"),
                FILE_HAS_NO_STAGE,
            ));
        }
        self.arrange(&command, force)?;

        let logged = self.kernel.provider_log(&self.kernel.runtime()?)?.len();
        let foreign = Rc::new(RefCell::new(BTreeSet::new()));
        let answered = self.run_command(&command, request, force, &foreign)?;
        let log = self.kernel.provider_log(&self.kernel.runtime()?)?;
        let held = log.len();
        let appended: Vec<_> = log
            .into_iter()
            .skip(logged)
            .filter(|event| !foreign.borrow().contains(&event.position))
            .collect();

        let mut result = SemanticCommandResult::undeclared();
        // The read-your-writes token: how many events the store's log held after the command.
        result.consistency = Some(token(held)?);
        match answered.result {
            Ok(answer) => {
                let outcome = match (command.as_str(), answered.before) {
                    (BEGIN, _) => "begun",
                    (SEAL, Some(StageState::Sealing)) => "retained-seal",
                    (SEAL, _) => "sealed",
                    (PUBLISH, Some(StageState::Published)) => "retained-publication",
                    (PUBLISH, _) => "published",
                    (ABANDON, Some(StageState::Abandoned)) => "retained-abandonment",
                    (ABANDON, Some(StageState::Sealing)) => "abandoned-sealed",
                    (ABANDON, _) => "abandoned",
                    _ => "compared",
                };
                result.outcome = Some(outcome_ref(&command, outcome)?);
                let field = if command == COMPARE {
                    result.direct_events.push(compared(&answer)?);
                    "comparison"
                } else {
                    "result"
                };
                let answer: Node = serde_json::from_value(answer)
                    .map_err(|e| unavailable(&format!("reading `{command}`'s answer"), e))?;
                result.response = Some(
                    response(answer)
                        .into_values()
                        .map(|value| (field.to_owned(), value))
                        .collect(),
                );
            }
            Err(Failure::Refused { name, message }) => {
                let (outcome, declared) =
                    refusal(&command, answered.before, name).ok_or_else(|| {
                        unavailable(
                            &format!("running `{command}`"),
                            format!("a refusal the specification does not name: {name}: {message}"),
                        )
                    })?;
                result.outcome = Some(outcome_ref(&command, outcome)?);
                result.error = Some(error(declared)?);
            }
            Err(other) => return Err(unavailable(&format!("running `{command}`"), other)),
        }
        for event in &appended {
            if !STAGE_EVENTS.contains(&event.name.as_str()) {
                continue;
            }
            let mut observed = ObservedEvent::new(event_ref(&event.name)?);
            for (field, value) in event.data.as_object().into_iter().flatten() {
                let value: Node = serde_json::from_value(value.clone())
                    .map_err(|e| unavailable("reading a stage event", e))?;
                observed = observed.with(field.clone(), value);
            }
            result.direct_events.push(observed.at(event.position));
        }
        self.observed
            .borrow_mut()
            .extend(result.direct_events.iter().cloned());
        Ok(result)
    }
}

/// The declared outcome and error of the refusal `name` of `command`, read against the stage's
/// state before it.
fn refusal(
    command: &str,
    before: Option<StageState>,
    name: &str,
) -> Option<(&'static str, &'static str)> {
    Some(match name {
        "stage-not-sealed"
        | "stage-sealed"
        | "stage-already-published"
        | "stage-already-abandoned" => (
            match (command, before, name) {
                (PUBLISH, Some(StageState::Published), _) => "already-published",
                (PUBLISH, Some(StageState::Sealing), "stage-already-abandoned") => {
                    "abandoned-meanwhile"
                }
                _ => "wrong-state",
            },
            "ekr.store.StageStateConflict",
        ),
        "stage-not-found" => ("stage-not-found", "ekr.store.StageNotFound"),
        "stage-head-moved" => ("head-moved", "ekr.store.StageHeadMoved"),
        "stage-stream-moved" => ("stream-moved", "ekr.store.StageStreamMoved"),
        "stage-object-moved" => ("object-moved", "ekr.store.StageObjectMoved"),
        "stage-suffix-refused" => ("suffix-refused", "ekr.store.StageSuffixRefused"),
        "stage-incomplete" => ("stage-incomplete", "ekr.store.StageIncomplete"),
        "stage-copy-failed" => ("copy-failed", "ekr.store.StageCopyFailed"),
        "unresolved-preparation" => ("unresolved-preparation", "ekr.store.UnresolvedPreparation"),
        "stage-unsupported-provider" => {
            ("unsupported-provider", "ekr.store.StageUnsupportedProvider")
        }
        NOT_SEEDED => ("not-seeded", NOT_SEEDED),
        "ekr.kernel.RevisionNotFound" => ("not-found", "ekr.kernel.RevisionNotFound"),
        _ => return None,
    })
}

/// `ekr.cli.StoresCompared`, counted over the `ekr.store-comparison/1` document.
fn compared(document: &serde_json::Value) -> Result<ObservedEvent, TargetError> {
    let refuse = |what: &str| unavailable("reading the comparison", what.to_owned());
    let revision = |side: &str| {
        document[side]["revision"]
            .as_u64()
            .ok_or_else(|| refuse(&format!("no {side}.revision")))
            .and_then(integer)
    };
    let differences = document["categories"]
        .as_array()
        .ok_or_else(|| refuse("no categories"))?
        .iter()
        .map(|category| {
            category["differences"]
                .as_u64()
                .ok_or_else(|| refuse("a category without differences"))
        })
        .sum::<Result<u64, TargetError>>()?;
    let equal = document["equal"]
        .as_bool()
        .ok_or_else(|| refuse("no equal"))?;
    Ok(ObservedEvent::new(event_ref("ekr.cli.StoresCompared")?)
        .with("equal", Node::Bool(equal))
        .with("left_revision", revision("left")?)
        .with("right_revision", revision("right")?)
        .with("differences", integer(differences)?))
}

/// The backend name and store path of `target`'s open scenario, as `--backend` and `--store` name
/// them.
fn store_path(target: &KernelTarget) -> Result<(&'static str, PathBuf), TargetError> {
    let directory = target.directory()?;
    Ok(match target.provider {
        Provider::File => ("file", directory.join("store")),
        Provider::Sqlite => ("sqlite", directory.join("store.sqlite")),
    })
}

impl ConformanceTarget for CliTarget {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        let provider = match self.provider() {
            Provider::File => "file",
            Provider::Sqlite => "sqlite",
        };
        Ok(ImplementationIdentity::new(
            format!("ekr ({provider} provider)"),
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn begin_scenario(&self, scenario: &ScenarioContext) -> Result<(), TargetError> {
        self.force.set(None);
        self.stand_in.set(None);
        self.observed.borrow_mut().clear();
        self.kernel.begin_scenario(scenario)?;
        self.other.begin_scenario(scenario)
    }

    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        self.execute(&request)
    }

    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        let view = request.view.to_string();
        if view != STAGES || !request.params.is_empty() {
            return Err(TargetError::unsupported(
                format!("the view `{view}`"),
                "the `ekr` component declares one view, ekr.cli.Stages, without parameters",
            ));
        }
        if let QueryConsistency::AtLeast { token } = &request.consistency {
            let wanted = token
                .as_str()
                .strip_prefix("occurrences:")
                .and_then(|count| count.parse::<usize>().ok())
                .ok_or_else(|| unavailable("reading a view", "a token this target did not mint"))?;
            let held = self.kernel.provider_log(&self.kernel.runtime()?)?.len();
            if held < wanted {
                return Err(unavailable(
                    "reading a view",
                    format!("the log holds {held} events, the token demands {wanted}"),
                ));
            }
        }
        let listed = self
            .answer(&["stage", "list"])?
            .map_err(|failure| unavailable("running `ekr stage list`", failure))?;
        let rows = listed
            .as_array()
            .ok_or_else(|| unavailable("reading `ekr stage list`", "not a list"))?
            .iter()
            .map(|row| {
                serde_json::from_value::<ViewRow>(row.clone())
                    .map_err(|e| unavailable("reading a stage row", e))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(SemanticViewResult::of(rows))
    }

    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        Ok(self
            .observed
            .borrow()
            .iter()
            .filter(|event| event.event == request.event)
            .cloned()
            .collect())
    }

    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> Result<(), TargetError> {
        let command = request.force.command.to_string();
        let force = Force::of(&command, &request.force.outcome.to_string()).ok_or_else(|| {
            TargetError::unsupported(
                format!("establishing `{}`", request.force),
                "the CLI target knows no precondition for that branch",
            )
        })?;
        if let Some(reason) = force.unsupported_on(self.provider()) {
            return Err(TargetError::unsupported(
                format!("establishing `{}`", request.force),
                reason,
            ));
        }
        self.force.set(Some(force));
        Ok(())
    }

    fn redeliver_event(&self, request: RedeliveryRequest) -> Result<(), TargetError> {
        Err(TargetError::unsupported(
            format!("delivering `{}` again", request.event),
            "the `ekr` component declares no bindings",
        ))
    }

    fn end_scenario(&self, scenario: &ScenarioContext) -> Result<(), TargetError> {
        self.force.set(None);
        self.stand_in.set(None);
        self.observed.borrow_mut().clear();
        self.other.end_scenario(scenario)?;
        self.kernel.end_scenario(scenario)
    }
}
