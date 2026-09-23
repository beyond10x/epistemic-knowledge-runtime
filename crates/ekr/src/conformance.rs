//! The ESS conformance target over the kernel: `story:ess-conformance-kernel`.
//!
//! [`KernelTarget`] answers the admitted `ekr-kernel` suite (`systems/ekr/conformance/suite.json`)
//! through the same [`Runtime`] provider opening and typed kernel handlers the `ekr` CLI calls:
//! `Runtime::file`/`Runtime::sqlite` under the trusted `ekr.cli-host/1` document, the kernel's own
//! seed parser, the bounded `propose_reader` ingress, `validate`, `commit`, and one verified read
//! for `Snapshot` and `Explain`. It asserts nothing: it reports what those handlers returned and
//! what the retained history holds afterwards, and the ESS runner decides.
//!
//! * **Isolation.** Every scenario opens a fresh provider root below the caller's work directory.
//! * **Documents.** A `SeedDocumentPath` or `TransactionDocumentPath` is a relative path. The
//!   target stages the scenario-owned fixture the manifest names for it at exactly that path under
//!   the scenario's own document root, refusing absolute paths, parent traversal and existing
//!   non-files, then opens the staged file and hands it to the real parser.
//! * **Preconditions.** Transaction commands need a seeded lineage holding the revision the suite
//!   validates against, so the target establishes revision 0 and revision 1 through the same
//!   handlers before the first such command. An external control establishes the state its
//!   branch declares — an intervening canonical commit, a different retained seed, an absent
//!   revision, transaction or assertion — and never selects the reported outcome.
//! * **Observations.** Events are the retained occurrences a command added to the verified
//!   history, read back from their real records; a retained retry adds none. Views are rows
//!   reconstructed from the verified history. Records project into ESS values with exact integer
//!   constructors, RFC 3339 instants, padded base64 bytes and `kind`/`value` tagged unions.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Component, Path, PathBuf};

use ekr_core::{AssertionId, ContentHash, EventId, RevisionNumber, Timestamp, TransactionId};
use ekr_graph::Root;
use ekr_kernel::{
    CommitCommandResult, CommitError, CommitReceiptV1, DocumentError, PersistenceError,
    ProjectionError, ProposalRecordV1, RejectionRecordV1, Runtime, SeedDocument, SeedError,
    SeedResultV1, StaleRecordV1, TransactionRecord, TransactionState, ValidationBasisV1,
    ValidationCommandResult, ValidationReceiptV1, VerifiedRead,
};
use ess_conformance::target::{
    ConformanceTarget, DeclaredErrorValue, EventObservationRequest, ExternalOutcomeControl,
    ImplementationIdentity, ObservedEvent, RedeliveryRequest, ScenarioContext,
    SemanticCommandRequest, SemanticCommandResult, SemanticViewRequest, SemanticViewResult,
    TargetError, ViewRow,
};
use ess_primitives::consistency::{ConsistencyToken, QueryConsistency};
use ess_primitives::facts::Number;
use ess_primitives::node::Node;
use serde::Deserialize;

use crate::host::CliHostConfigurationV1;

/// The native provider a [`KernelTarget`] opens, exactly as `ekr --backend` selects it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Provider {
    /// `Runtime::file`.
    File,
    /// `Runtime::sqlite`.
    Sqlite,
}

/// The conformance target over the real kernel handlers of one native provider.
pub struct KernelTarget {
    provider: Provider,
    fixtures: PathBuf,
    manifest: Manifest,
    host: CliHostConfigurationV1,
    work: PathBuf,
    opened: Cell<u64>,
    clock: Cell<i64>,
    scenario: RefCell<Option<Scenario>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    format: String,
    host: String,
    setup: Setup,
    documents: BTreeMap<String, String>,
    scenarios: BTreeMap<String, BTreeMap<String, String>>,
    proposals: BTreeMap<TransactionId, String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Setup {
    seed: String,
    revision_1: String,
    different_seed: String,
    intervening: String,
}

struct Scenario {
    id: String,
    directory: PathBuf,
    prepared: bool,
    control: Option<Control>,
}

/// An external branch whose declared state the target establishes before the real handler runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    AlreadySeeded,
    InvalidSeed,
    Malformed,
    Misattributed,
    Rejected,
    RevisionAbsent,
    ValidateTransactionAbsent,
    Stale,
    CommitTransactionAbsent,
    SnapshotRevisionAbsent,
    AssertionAbsent,
}

impl Control {
    fn of(command: &str, outcome: &str) -> Option<Self> {
        Some(match (command, outcome) {
            ("ekr.kernel.Seed", "already-seeded") => Self::AlreadySeeded,
            ("ekr.kernel.Seed", "invalid-seed") => Self::InvalidSeed,
            ("ekr.kernel.Propose", "malformed") => Self::Malformed,
            ("ekr.kernel.Propose", "misattributed") => Self::Misattributed,
            ("ekr.kernel.Validate", "rejected") => Self::Rejected,
            ("ekr.kernel.Validate", "revision-not-found") => Self::RevisionAbsent,
            ("ekr.kernel.Validate", "transaction-not-found") => Self::ValidateTransactionAbsent,
            ("ekr.kernel.Commit", "stale") => Self::Stale,
            ("ekr.kernel.Commit", "transaction-not-found") => Self::CommitTransactionAbsent,
            ("ekr.kernel.Snapshot", "not-found") => Self::SnapshotRevisionAbsent,
            ("ekr.kernel.Explain", "not-found") => Self::AssertionAbsent,
            _ => return None,
        })
    }

    /// Whether the lineage is established without revision 1 before the first command.
    fn seed_only(self) -> bool {
        matches!(
            self,
            Self::RevisionAbsent | Self::SnapshotRevisionAbsent | Self::AssertionAbsent
        )
    }
}

const SEED: &str = "ekr.kernel.Seed";
const PROPOSE: &str = "ekr.kernel.Propose";
const VALIDATE: &str = "ekr.kernel.Validate";
const COMMIT: &str = "ekr.kernel.Commit";
const SNAPSHOT: &str = "ekr.kernel.Snapshot";
const EXPLAIN: &str = "ekr.kernel.Explain";

/// The first instant the kernel's host clock reads in every scenario; each sample adds a second.
const CLOCK_START_MS: i64 = 1_800_000_000_000;

fn unavailable(operation: &str, detail: impl std::fmt::Display) -> TargetError {
    TargetError::unavailable(operation, detail.to_string())
}

/// The read-your-writes token: how many retained occurrences the history held after a command.
fn token(count: usize) -> Result<ConsistencyToken, TargetError> {
    ConsistencyToken::new(format!("occurrences:{count}"))
        .map_err(|e| unavailable("minting a consistency token", e))
}

impl KernelTarget {
    /// Opens a target over `provider`, reading `manifest.json` and the trusted host document from
    /// `fixtures` and creating one isolated provider root per scenario below `work`.
    ///
    /// # Errors
    ///
    /// An unreadable or malformed manifest or host document.
    pub fn new(provider: Provider, fixtures: &Path, work: &Path) -> Result<Self, String> {
        let manifest_path = fixtures.join("manifest.json");
        let manifest: Manifest = std::fs::read(&manifest_path)
            .map_err(|e| format!("reading {}: {e}", manifest_path.display()))
            .and_then(|bytes| {
                serde_json::from_slice(&bytes)
                    .map_err(|e| format!("{}: {e}", manifest_path.display()))
            })?;
        if manifest.format != "ekr.conformance-fixtures/1" {
            return Err(format!("unsupported fixture manifest {}", manifest.format));
        }
        let host_path = fixtures.join(simple(&manifest.host)?);
        let host = std::fs::read(&host_path)
            .map_err(|e| format!("reading {}: {e}", host_path.display()))
            .and_then(|bytes| {
                CliHostConfigurationV1::from_json(&bytes)
                    .map_err(|e| format!("{}: {e}", host_path.display()))
            })?;
        Ok(Self {
            provider,
            fixtures: fixtures.to_path_buf(),
            manifest,
            host,
            work: work.to_path_buf(),
            opened: Cell::new(0),
            clock: Cell::new(CLOCK_START_MS),
            scenario: RefCell::new(None),
        })
    }

    /// The kernel's host clock for new decisions: a strictly increasing target-owned instant.
    fn tick(&self) -> Timestamp {
        let now = self.clock.get();
        self.clock.set(now + 1_000);
        Timestamp::from_millis(now)
    }

    /// Opens the scenario's provider exactly as the CLI does for every command.
    fn runtime(&self) -> Result<Runtime, TargetError> {
        let directory = self.directory()?;
        let CliHostConfigurationV1 {
            tenant,
            context,
            authority,
            ..
        } = self.host.clone();
        match self.provider {
            Provider::File => Runtime::file(&directory.join("store"), &tenant, context, authority),
            Provider::Sqlite => {
                Runtime::sqlite(&directory.join("store.sqlite"), &tenant, context, authority)
            }
        }
        .map_err(|e| unavailable("opening the provider", e))
    }

    fn directory(&self) -> Result<PathBuf, TargetError> {
        self.scenario
            .borrow()
            .as_ref()
            .map(|scenario| scenario.directory.clone())
            .ok_or_else(|| unavailable("opening the provider", "no scenario is open"))
    }

    fn fixture(&self, name: &str) -> Result<Vec<u8>, TargetError> {
        let path = self
            .fixtures
            .join(simple(name).map_err(|e| unavailable("reading a fixture", e))?);
        std::fs::read(&path).map_err(|e| unavailable("reading a fixture", format!("{name}: {e}")))
    }

    /// The scenario-owned fixture staged at a synthesized document path.
    fn document_for(&self, relative: &str) -> Result<String, TargetError> {
        let scenario = self.scenario.borrow();
        let id = scenario.as_ref().map(|s| s.id.as_str()).unwrap_or_default();
        self.manifest
            .scenarios
            .get(id)
            .and_then(|documents| documents.get(relative))
            .or_else(|| self.manifest.documents.get(relative))
            .cloned()
            .ok_or_else(|| {
                TargetError::unsupported(
                    format!("the document at `{relative}`"),
                    "the fixture manifest stages no document at that path",
                )
            })
    }

    /// Stages the manifest's fixture at exactly `relative` below the scenario's document root.
    fn stage(&self, relative: &str) -> Result<PathBuf, TargetError> {
        let fixture = self.document_for(relative)?;
        let bytes = self.fixture(&fixture)?;
        let path = self
            .directory()?
            .join("documents")
            .join(contained(relative).map_err(|e| unavailable("staging a document", e))?);
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_file() => {
                let held =
                    std::fs::read(&path).map_err(|e| unavailable("staging a document", e))?;
                if held != bytes {
                    return Err(unavailable(
                        "staging a document",
                        format!("`{relative}` already holds different bytes"),
                    ));
                }
            }
            Ok(_) => {
                return Err(unavailable(
                    "staging a document",
                    format!("`{relative}` exists and is not a regular file"),
                ))
            }
            Err(_) => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| unavailable("staging a document", e))?;
                }
                std::fs::write(&path, &bytes).map_err(|e| unavailable("staging a document", e))?;
            }
        }
        Ok(path)
    }

    /// The shared seed handler, as `ekr seed` runs it over an opened document.
    fn seed_from(
        &self,
        runtime: &Runtime,
        reader: &mut dyn Read,
    ) -> Result<SeedResultV1, SeedError> {
        let mut bytes = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map_err(|e| SeedError::Invalid(format!("seed-read: {e}")))?;
        let text = String::from_utf8(bytes)
            .map_err(|_| SeedError::Invalid("seed-decode: the document is not UTF-8".to_owned()))?;
        let document = SeedDocument::from_yaml(&text)?;
        runtime.seed(document, || self.tick())
    }

    /// Establishes a seeded lineage from a fixture through the real seed handler.
    fn establish_seed(&self, fixture: &str) -> Result<(), TargetError> {
        let runtime = self.runtime()?;
        let bytes = self.fixture(fixture)?;
        self.seed_from(&runtime, &mut bytes.as_slice())
            .map(|_| ())
            .map_err(|e| unavailable("establishing the seed", e))
    }

    /// Establishes one committed revision from a fixture through propose, validate and commit.
    fn establish_commit(&self, fixture: &str) -> Result<(), TargetError> {
        let bytes = self.fixture(fixture)?;
        let operator = self.host.context.operator;
        let runtime = self.runtime()?;
        let head = runtime
            .head()
            .map_err(|e| unavailable("establishing a revision", e))?
            .ok_or_else(|| unavailable("establishing a revision", "the lineage is not seeded"))?;
        let proposal = runtime
            .propose_reader(bytes.as_slice(), operator, || self.tick())
            .map_err(|e| unavailable("establishing a revision", e))?;
        let runtime = self.runtime()?;
        match runtime.validate(proposal.transaction_id, head.revision, || self.tick()) {
            Ok(ValidationCommandResult::Validated(_)) => {}
            other => {
                return Err(unavailable(
                    "establishing a revision",
                    format!("{fixture} did not validate: {other:?}"),
                ))
            }
        }
        let runtime = self.runtime()?;
        match runtime.commit(proposal.transaction_id, operator, || self.tick()) {
            Ok(CommitCommandResult::Committed(_)) => Ok(()),
            other => Err(unavailable(
                "establishing a revision",
                format!("{fixture} did not commit: {other:?}"),
            )),
        }
    }

    /// Establishes the lineage the scenario's first command acts on, once.
    fn prepare(&self, command: &str) -> Result<(), TargetError> {
        let control = {
            let mut scenario = self.scenario.borrow_mut();
            let scenario = scenario
                .as_mut()
                .ok_or_else(|| unavailable("preparing", "no scenario is open"))?;
            if scenario.prepared {
                return Ok(());
            }
            scenario.prepared = true;
            scenario.control
        };
        let setup = &self.manifest.setup;
        if command == SEED {
            if control == Some(Control::AlreadySeeded) {
                self.establish_seed(&setup.different_seed)?;
            }
            return Ok(());
        }
        self.establish_seed(&setup.seed)?;
        if !control.is_some_and(Control::seed_only) {
            self.establish_commit(&setup.revision_1)?;
        }
        Ok(())
    }

    fn take_control(&self) -> Option<Control> {
        self.scenario
            .borrow_mut()
            .as_mut()
            .and_then(|scenario| scenario.control.take())
    }

    fn has_scenario_document(&self, relative: &str) -> bool {
        let scenario = self.scenario.borrow();
        scenario.as_ref().is_some_and(|scenario| {
            self.manifest
                .scenarios
                .get(&scenario.id)
                .is_some_and(|documents| documents.contains_key(relative))
        })
    }

    /// Every retained occurrence of the verified history, keyed by its occurrence identity.
    fn occurrences(&self, runtime: &Runtime) -> Result<Occurrences, TargetError> {
        let read = match runtime.read(None) {
            Ok(read) => read,
            Err(CommitError::NotSeeded) => return Ok(Occurrences::default()),
            Err(e) => return Err(unavailable("reading the retained history", e)),
        };
        occurrences(&read)
    }

    fn execute(
        &self,
        request: &SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        let command = request.command.to_string();
        check_actor(&command, request)?;
        self.prepare(&command)?;
        let control = self.take_control();
        if let Some(control) = control {
            self.precondition(control, request)?;
        }
        let runtime = self.runtime()?;
        let before = self.occurrences(&runtime)?;
        let mut result = match command.as_str() {
            SEED => self.seed(&runtime, request, &before)?,
            PROPOSE => self.propose(&runtime, request)?,
            VALIDATE => self.validate(&runtime, request)?,
            COMMIT => self.commit(&runtime, request, &before)?,
            SNAPSHOT => self.snapshot(&runtime, request)?,
            EXPLAIN => self.explain(&runtime, request)?,
            _ => {
                return Err(TargetError::unsupported(
                    format!("the command `{command}`"),
                    "the kernel target answers only ekr-kernel's commands",
                ))
            }
        };
        let after = self.occurrences(&runtime)?;
        let after_count = after.events.len();
        let added = after.since(&before);
        let published = std::mem::take(&mut result.published);
        if let Some(expected) = published {
            if added.len() != 1 || added[0].0 != expected {
                return Err(unavailable(
                    &format!("observing what `{command}` published"),
                    format!(
                        "the returned record {expected} is not the one new retained occurrence \
                         (new: {:?})",
                        added
                            .iter()
                            .map(|(id, _)| id.to_string())
                            .collect::<Vec<_>>()
                    ),
                ));
            }
        } else if result.outcome.is_some() && !added.is_empty() && result.durable {
            return Err(unavailable(
                &format!("observing what `{command}` published"),
                "a result that names no new record added retained occurrences",
            ));
        }
        let mut observed = SemanticCommandResult::undeclared();
        observed.outcome = match result.outcome {
            Some(outcome) => Some(outcome_ref(&command, outcome)?),
            None => None,
        };
        observed.error = result.error;
        observed.response = result.response;
        observed.direct_events = added.into_iter().map(|(_, event)| event).collect();
        observed.direct_events.extend(result.read_events);
        observed.consistency = Some(token(after_count)?);
        Ok(observed)
    }

    /// Establishes, or verifies, the state an external branch declares before its handler runs.
    fn precondition(
        &self,
        control: Control,
        request: &SemanticCommandRequest,
    ) -> Result<(), TargetError> {
        let runtime = self.runtime()?;
        match control {
            Control::AlreadySeeded | Control::Stale => Ok(()),
            Control::InvalidSeed => self.require_document("seed_document"),
            Control::Malformed | Control::Misattributed | Control::Rejected => {
                self.require_document("transaction_document")
            }
            Control::RevisionAbsent => {
                let id = transaction_input(request)?;
                let fixture = self.manifest.proposals.get(&id).ok_or_else(|| {
                    TargetError::unsupported(
                        format!("a proposed transaction {id}"),
                        "the fixture manifest holds no proposal with that identity",
                    )
                })?;
                let bytes = self.fixture(fixture)?;
                let record = runtime
                    .propose_reader(bytes.as_slice(), self.host.context.operator, || self.tick())
                    .map_err(|e| unavailable("establishing a proposal", e))?;
                if record.transaction_id != id {
                    return Err(unavailable(
                        "establishing a proposal",
                        format!("{fixture} proposes {}", record.transaction_id),
                    ));
                }
                let against = revision_input(request, "against")?
                    .ok_or_else(|| unavailable("establishing an absent revision", "no basis"))?;
                require_absent_revision(&runtime, against)
            }
            Control::ValidateTransactionAbsent | Control::CommitTransactionAbsent => {
                let id = transaction_input(request)?;
                let held = runtime
                    .transactions()
                    .map_err(|e| unavailable("reading transactions", e))?;
                if held.contains_key(&id) {
                    return Err(unavailable(
                        "establishing an absent transaction",
                        format!("{id} is retained"),
                    ));
                }
                Ok(())
            }
            Control::SnapshotRevisionAbsent => match revision_input(request, "at")? {
                Some(at) => require_absent_revision(&runtime, at),
                None => Err(unavailable(
                    "establishing an absent revision",
                    "the newest revision always exists once seeded",
                )),
            },
            Control::AssertionAbsent => {
                let id: AssertionId = parse_text(request, "assertion_id")?;
                let read = runtime
                    .read(None)
                    .map_err(|e| unavailable("establishing an absent assertion", e))?;
                if read.graph.assertions.contains_key(&id) {
                    return Err(unavailable(
                        "establishing an absent assertion",
                        format!("{id} is canonical"),
                    ));
                }
                Ok(())
            }
        }
    }

    fn require_document(&self, relative: &str) -> Result<(), TargetError> {
        if self.has_scenario_document(relative) {
            Ok(())
        } else {
            Err(TargetError::unsupported(
                "the declared external precondition",
                format!("the fixture manifest stages no scenario-owned `{relative}` here"),
            ))
        }
    }

    fn seed(
        &self,
        runtime: &Runtime,
        request: &SemanticCommandRequest,
        before: &Occurrences,
    ) -> Result<Answer, TargetError> {
        let relative = text_input(request, "seed_document")?;
        let path = self.stage(&relative)?;
        let mut file =
            std::fs::File::open(&path).map_err(|e| unavailable("opening the seed document", e))?;
        Ok(match self.seed_from(runtime, &mut file) {
            Ok(result) => {
                // A retained retry answers the occurrence the history already held.
                let retained = before.events.contains_key(&result.event_id);
                Answer {
                    outcome: Some(if retained { "retained-seed" } else { "seeded" }),
                    published: (!retained).then_some(result.event_id),
                    response: Some(response(seed_result(&result)?)),
                    durable: true,
                    ..Answer::default()
                }
            }
            Err(SeedError::Invalid(reason)) => {
                let code = reason
                    .split_once(':')
                    .map_or(reason.as_str(), |(code, _)| code)
                    .trim()
                    .to_owned();
                Answer::refused(
                    "invalid-seed",
                    error("ekr.kernel.InvalidSeed")?
                        .with("code", Node::Text(code))
                        .with("reason", Node::Text(reason)),
                )
            }
            Err(SeedError::Store(PersistenceError::AlreadySeeded)) => {
                let current = runtime
                    .head()
                    .map_err(|e| unavailable("reading the retained head", e))?
                    .ok_or_else(|| unavailable("reading the retained head", "no seed exists"))?
                    .revision;
                Answer::refused(
                    "already-seeded",
                    error("ekr.kernel.AlreadySeeded")?.with("current", revision(current)?),
                )
            }
            Err(SeedError::Store(other)) => return Err(unavailable("seeding", other)),
        })
    }

    fn propose(
        &self,
        runtime: &Runtime,
        request: &SemanticCommandRequest,
    ) -> Result<Answer, TargetError> {
        let relative = text_input(request, "transaction_document")?;
        let path = self.stage(&relative)?;
        let file = std::fs::File::open(&path)
            .map_err(|e| unavailable("opening the transaction document", e))?;
        Ok(
            match runtime.propose_reader(file, self.host.context.operator, || self.tick()) {
                Ok(record) => Answer {
                    outcome: Some("proposed"),
                    published: Some(record.event_id),
                    response: Some(response(proposal(&record)?)),
                    durable: true,
                    ..Answer::default()
                },
                Err(CommitError::Document(DocumentError::Io(io))) => {
                    return Err(unavailable("reading the transaction document", io))
                }
                Err(CommitError::Document(document)) => Answer::refused(
                    "malformed",
                    error("ekr.kernel.StructurallyInvalid")?
                        .with("reason", Node::Text(document.to_string())),
                ),
                Err(CommitError::ProposalAttribution { actor }) => Answer::refused(
                    "misattributed",
                    error("ekr.kernel.ProposalAttribution")?.with("actor", text(actor)),
                ),
                Err(CommitError::TransactionStateConflict { state, .. }) => Answer {
                    error: Some(state_conflict(state)?),
                    ..Answer::default()
                },
                Err(other) => return Err(unavailable("proposing", other)),
            },
        )
    }

    fn validate(
        &self,
        runtime: &Runtime,
        request: &SemanticCommandRequest,
    ) -> Result<Answer, TargetError> {
        let id = transaction_input(request)?;
        let against = revision_input(request, "against")?
            .ok_or_else(|| unavailable("validating", "`against` is required"))?;
        Ok(match runtime.validate(id, against, || self.tick()) {
            Ok(ValidationCommandResult::Validated(receipt)) => Answer {
                outcome: Some("validated"),
                published: Some(receipt.event_id),
                response: Some(response(union("Validated", validation(&receipt)?))),
                durable: true,
                ..Answer::default()
            },
            Ok(ValidationCommandResult::Rejected(record)) => Answer {
                outcome: Some("rejected"),
                published: Some(record.event_id),
                response: Some(response(union("Rejected", rejection(&record)?))),
                durable: true,
                ..Answer::default()
            },
            Err(error) => self.transaction_refusal(error)?,
        })
    }

    fn commit(
        &self,
        runtime: &Runtime,
        request: &SemanticCommandRequest,
        before: &Occurrences,
    ) -> Result<Answer, TargetError> {
        let id = transaction_input(request)?;
        Ok(
            match runtime.commit(id, self.host.context.operator, || self.tick()) {
                Ok(CommitCommandResult::Committed(receipt)) => {
                    let retained = before.events.contains_key(&receipt.event_id);
                    Answer {
                        outcome: Some(if retained {
                            "retained-commit"
                        } else {
                            "committed"
                        }),
                        published: (!retained).then_some(receipt.event_id),
                        response: Some(response(union("Committed", commit_receipt(&receipt)?))),
                        durable: true,
                        ..Answer::default()
                    }
                }
                Ok(CommitCommandResult::Stale(record)) => Answer {
                    outcome: Some("stale"),
                    published: Some(record.event_id),
                    response: Some(response(union("Stale", stale(&record)?))),
                    durable: true,
                    ..Answer::default()
                },
                Err(error) => self.transaction_refusal(error)?,
            },
        )
    }

    /// The declared refusals Validate and Commit share, with their actual fields.
    fn transaction_refusal(&self, refusal: CommitError) -> Result<Answer, TargetError> {
        Ok(match refusal {
            CommitError::TransactionNotFound { transaction_id } => Answer::refused(
                "transaction-not-found",
                error("ekr.kernel.TransactionNotFound")?
                    .with("transaction_id", text(transaction_id)),
            ),
            CommitError::RevisionNotFound { against } => Answer::refused(
                "revision-not-found",
                error("ekr.kernel.RevisionNotFound")?.with("requested", revision(against)?),
            ),
            CommitError::TransactionStateConflict { state, .. } => Answer {
                outcome: Some("wrong-state"),
                error: Some(state_conflict(state)?),
                ..Answer::default()
            },
            other => return Err(unavailable("executing the transaction command", other)),
        })
    }

    fn snapshot(
        &self,
        runtime: &Runtime,
        request: &SemanticCommandRequest,
    ) -> Result<Answer, TargetError> {
        let at = revision_input(request, "at")?;
        let valid_at = match request.input.get("valid_at") {
            None | Some(Node::Null) => None,
            Some(Node::Text(value)) => Some(parse_instant(value)?),
            Some(other) => {
                return Err(unavailable(
                    "reading `valid_at`",
                    format!("{} is not a timestamp", other.type_name()),
                ))
            }
        };
        let read = match runtime.read(at) {
            Ok(read) => read,
            Err(CommitError::RevisionNotFound { against }) => {
                return Ok(Answer::refused(
                    "not-found",
                    error("ekr.kernel.RevisionNotFound")?.with("requested", revision(against)?),
                ))
            }
            Err(other) => return Err(unavailable("reading a snapshot", other)),
        };
        let result = read
            .snapshot(valid_at)
            .map_err(|e| unavailable("projecting a snapshot", e))?;
        let event = ObservedEvent::new(event_ref("ekr.kernel.SnapshotTaken")?)
            .with("number", revision(result.root.revision)?)
            .with("knowledge_root", text(result.root.knowledge_root));
        Ok(Answer {
            outcome: Some("taken"),
            read_events: vec![event],
            ..Answer::default()
        })
    }

    fn explain(
        &self,
        runtime: &Runtime,
        request: &SemanticCommandRequest,
    ) -> Result<Answer, TargetError> {
        let id: AssertionId = parse_text(request, "assertion_id")?;
        let read = runtime
            .read(None)
            .map_err(|e| unavailable("reading the newest revision", e))?;
        Ok(match read.explain(id) {
            Ok(result) => {
                let links = i64::try_from(result.links.len())
                    .map_err(|e| unavailable("counting explanation links", e))?;
                let event = ObservedEvent::new(event_ref("ekr.kernel.Explained")?)
                    .with("assertion_id", text(result.assertion_id))
                    .with("links", Node::Number(Number::from(links)));
                Answer {
                    outcome: Some("explained"),
                    read_events: vec![event],
                    ..Answer::default()
                }
            }
            Err(ProjectionError::AssertionNotFound { requested }) => Answer::refused(
                "not-found",
                error("ekr.kernel.AssertionNotFound")?.with("requested", text(requested)),
            ),
            Err(other) => return Err(unavailable("explaining", other)),
        })
    }

    /// One verified capture of the scenario's history; `None` before the lineage is seeded.
    fn capture(&self) -> Result<Option<VerifiedRead>, TargetError> {
        match self.runtime()?.read(None) {
            Ok(read) => Ok(Some(read)),
            Err(CommitError::NotSeeded) => Ok(None),
            Err(e) => Err(unavailable("reading the retained history", e)),
        }
    }

    /// The rows of one declared view, all from the same verified capture.
    fn rows(read: Option<&VerifiedRead>, view: &str) -> Result<Vec<ViewRow>, TargetError> {
        if !matches!(
            view,
            "ekr.kernel.Transactions"
                | "ekr.kernel.PendingTransactions"
                | "ekr.kernel.Revisions"
                | "ekr.kernel.CurrentRevision"
        ) {
            return Err(TargetError::unsupported(
                format!("the view `{view}`"),
                "the kernel target reads only ekr-kernel's views",
            ));
        }
        let Some(read) = read else {
            return Ok(Vec::new());
        };
        match view {
            "ekr.kernel.Transactions" => read
                .transactions
                .values()
                .map(|record| transaction_row(read, record))
                .collect(),
            "ekr.kernel.PendingTransactions" => read
                .transactions
                .values()
                .filter(|record| record.state() == TransactionState::Proposed)
                .map(|record| {
                    Ok(BTreeMap::from([
                        (
                            "transaction_id".to_owned(),
                            text(record.proposal.transaction_id),
                        ),
                        ("proposer".to_owned(), text(record.proposal.submitter)),
                        (
                            "operation_count".to_owned(),
                            integer(record.proposal.operation_count)?,
                        ),
                    ]))
                })
                .collect(),
            "ekr.kernel.Revisions" => revision_rows(read),
            "ekr.kernel.CurrentRevision" => {
                let mut rows = Vec::new();
                for (number, revision_state) in read.revisions.iter().rev() {
                    rows.push(BTreeMap::from([
                        ("revision_id".to_owned(), text(revision_state.revision_id)),
                        ("number".to_owned(), revision(*number)?),
                        (
                            "knowledge_root".to_owned(),
                            text(revision_state.root.knowledge_root),
                        ),
                    ]));
                }
                Ok(rows)
            }
            _ => unreachable!("the view name was admitted above"),
        }
    }
}

/// What one handler answered, before the target compares it with the retained history.
#[derive(Default)]
struct Answer {
    outcome: Option<&'static str>,
    error: Option<DeclaredErrorValue>,
    response: Option<BTreeMap<String, Node>>,
    /// The occurrence the returned record says it published, when it says it published one.
    published: Option<EventId>,
    /// Whether the command writes durable history at all.
    durable: bool,
    /// Read-only result events built from the actual query result.
    read_events: Vec<ObservedEvent>,
}

impl Answer {
    fn refused(outcome: &'static str, error: DeclaredErrorValue) -> Self {
        Self {
            outcome: Some(outcome),
            error: Some(error),
            ..Self::default()
        }
    }
}

impl ConformanceTarget for KernelTarget {
    fn identity(&self) -> Result<ImplementationIdentity, TargetError> {
        let provider = match self.provider {
            Provider::File => "file",
            Provider::Sqlite => "sqlite",
        };
        Ok(ImplementationIdentity::new(
            format!("ekr-kernel ({provider} provider)"),
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn begin_scenario(&self, scenario: &ScenarioContext) -> Result<(), TargetError> {
        let number = self.opened.get() + 1;
        self.opened.set(number);
        let directory = self.work.join(format!("scenario-{number:04}"));
        if directory.exists() {
            return Err(unavailable(
                "opening an isolated scenario",
                format!("{} already exists", directory.display()),
            ));
        }
        std::fs::create_dir_all(directory.join("store"))
            .map_err(|e| unavailable("opening an isolated scenario", e))?;
        self.clock.set(CLOCK_START_MS);
        *self.scenario.borrow_mut() = Some(Scenario {
            id: scenario.scenario.to_string(),
            directory,
            prepared: false,
            control: None,
        });
        Ok(())
    }

    fn execute_command(
        &self,
        request: SemanticCommandRequest,
    ) -> Result<SemanticCommandResult, TargetError> {
        self.execute(&request)
    }

    fn query_view(&self, request: SemanticViewRequest) -> Result<SemanticViewResult, TargetError> {
        if !request.params.is_empty() {
            return Err(TargetError::unsupported(
                format!("parameters of `{}`", request.view),
                "no ekr-kernel view declares parameters",
            ));
        }
        let read = self.capture()?;
        if let QueryConsistency::AtLeast { token } = &request.consistency {
            let wanted = token
                .as_str()
                .strip_prefix("occurrences:")
                .and_then(|count| count.parse::<usize>().ok())
                .ok_or_else(|| unavailable("reading a view", "a token this target did not mint"))?;
            let held = match &read {
                Some(read) => occurrences(read)?.events.len(),
                None => 0,
            };
            if held < wanted {
                return Err(unavailable(
                    "reading a view",
                    format!("history holds {held} occurrences, the token demands {wanted}"),
                ));
            }
        }
        Ok(SemanticViewResult::of(Self::rows(
            read.as_ref(),
            &request.view.to_string(),
        )?))
    }

    fn observe_events(
        &self,
        request: EventObservationRequest,
    ) -> Result<Vec<ObservedEvent>, TargetError> {
        let occurrences = self.occurrences(&self.runtime()?)?;
        Ok(occurrences
            .events
            .into_values()
            .filter(|event| event.event == request.event)
            .collect())
    }

    fn configure_external_outcome(
        &self,
        request: ExternalOutcomeControl,
    ) -> Result<(), TargetError> {
        let command = request.force.command.to_string();
        let control =
            Control::of(&command, &request.force.outcome.to_string()).ok_or_else(|| {
                TargetError::unsupported(
                    format!("establishing `{}`", request.force),
                    "the kernel target knows no precondition for that branch",
                )
            })?;
        if control == Control::Stale {
            // The declared state is a canonical head that moved after validation: establish it
            // with one real intervening commit on top of the current head.
            self.establish_commit(&self.manifest.setup.intervening.clone())?;
        }
        let mut scenario = self.scenario.borrow_mut();
        let scenario = scenario
            .as_mut()
            .ok_or_else(|| unavailable("configuring an external outcome", "no scenario is open"))?;
        scenario.control = Some(control);
        Ok(())
    }

    fn redeliver_event(&self, request: RedeliveryRequest) -> Result<(), TargetError> {
        Err(TargetError::unsupported(
            format!("delivering `{}` again", request.event),
            "ekr-kernel declares no bindings",
        ))
    }

    fn end_scenario(&self, _scenario: &ScenarioContext) -> Result<(), TargetError> {
        *self.scenario.borrow_mut() = None;
        Ok(())
    }
}

// ---- occurrences -----------------------------------------------------------------------------

/// Retained occurrences in history order of kind, keyed by occurrence identity.
#[derive(Default)]
struct Occurrences {
    events: BTreeMap<EventId, ObservedEvent>,
    order: Vec<EventId>,
}

impl Occurrences {
    fn push(&mut self, id: EventId, event: ObservedEvent) -> Result<(), TargetError> {
        if self.events.insert(id, event).is_some() {
            return Err(unavailable(
                "reading the retained history",
                format!("occurrence {id} is retained twice"),
            ));
        }
        self.order.push(id);
        Ok(())
    }

    /// The occurrences `before` did not hold, in retained order.
    fn since(&self, before: &Self) -> Vec<(EventId, ObservedEvent)> {
        self.order
            .iter()
            .filter(|id| !before.events.contains_key(id))
            .map(|id| (*id, self.events[id].clone()))
            .collect()
    }
}

/// The payload address of a retained record, verified against the captured retained bytes.
fn retained(
    read: &VerifiedRead,
    bytes: Result<Vec<u8>, PersistenceError>,
) -> Result<ContentHash, TargetError> {
    let bytes = bytes.map_err(|e| unavailable("encoding a retained record", e))?;
    let hash = ContentHash::of_bytes(&bytes);
    if read.content(&hash) != Some(bytes.as_slice()) {
        return Err(unavailable(
            "reading a retained record",
            format!("{hash} is not a retained object of this history"),
        ));
    }
    Ok(hash)
}

/// [`retained`], additionally agreeing with the address the kernel's own record carries.
fn addressed(
    read: &VerifiedRead,
    bytes: Result<Vec<u8>, PersistenceError>,
    recorded: Option<ContentHash>,
) -> Result<ContentHash, TargetError> {
    let hash = retained(read, bytes)?;
    if recorded != Some(hash) {
        return Err(unavailable(
            "reading a retained record",
            format!("{hash} is not the address the transaction record holds ({recorded:?})"),
        ));
    }
    Ok(hash)
}

fn occurrence(name: &str, fields: Vec<(&str, Node)>) -> Result<ObservedEvent, TargetError> {
    let mut event = ObservedEvent::new(event_ref(name)?);
    for (field, value) in fields {
        event = event.with(field, value);
    }
    Ok(event)
}

fn occurrences(read: &VerifiedRead) -> Result<Occurrences, TargetError> {
    let mut out = Occurrences::default();
    let seed = &read.seed;
    let origin = read
        .revisions
        .get(&RevisionNumber::SEED)
        .filter(|revision| revision.event_id == seed.event_id)
        .ok_or_else(|| unavailable("reading the retained seed", "revision 0 is not the seed"))?;
    out.push(
        seed.event_id,
        occurrence(
            "ekr.kernel.Seeded",
            vec![
                ("event_id", text(seed.event_id)),
                ("record_hash", text(origin.record_hash)),
                ("revision_id", text(seed.revision_id)),
                ("seed_hash", text(seed.seed_hash)),
            ],
        )?,
    )?;
    for record in read.transactions.values() {
        let proposal = &record.proposal;
        out.push(
            proposal.event_id,
            occurrence(
                "ekr.kernel.TransactionProposed",
                vec![
                    ("event_id", text(proposal.event_id)),
                    (
                        "record_hash",
                        text(addressed(
                            read,
                            proposal.to_bytes(),
                            Some(record.proposal_record_hash),
                        )?),
                    ),
                    ("transaction_id", text(proposal.transaction_id)),
                    ("proposer", text(proposal.submitter)),
                    (
                        "operations_hash",
                        optional(proposal.canonical_operations_hash.map(text)),
                    ),
                ],
            )?,
        )?;
        if let Some(receipt) = &record.validation {
            out.push(
                receipt.event_id,
                occurrence(
                    "ekr.kernel.TransactionValidated",
                    vec![
                        ("event_id", text(receipt.event_id)),
                        (
                            "record_hash",
                            text(addressed(
                                read,
                                receipt.to_bytes(),
                                record.validation_record_hash,
                            )?),
                        ),
                        ("transaction_id", text(proposal.transaction_id)),
                        ("against", revision(receipt.basis.previous_root.revision)?),
                        ("validation_hash", text(receipt.validation_hash)),
                    ],
                )?,
            )?;
        }
        if let Some(rejection) = &record.rejection {
            let issues = i64::try_from(rejection.issues.len())
                .map_err(|e| unavailable("counting issues", e))?;
            out.push(
                rejection.event_id,
                occurrence(
                    "ekr.kernel.TransactionRejected",
                    vec![
                        ("event_id", text(rejection.event_id)),
                        ("record_hash", text(retained(read, rejection.to_bytes())?)),
                        ("transaction_id", text(proposal.transaction_id)),
                        ("issues", Node::Number(Number::from(issues))),
                    ],
                )?,
            )?;
        }
        if let Some(stale_record) = &record.stale {
            out.push(
                stale_record.event_id,
                occurrence(
                    "ekr.kernel.TransactionStale",
                    vec![
                        ("event_id", text(stale_record.event_id)),
                        (
                            "record_hash",
                            text(retained(read, stale_record.to_bytes())?),
                        ),
                        ("transaction_id", text(proposal.transaction_id)),
                        (
                            "validated_against",
                            revision(stale_record.expected_basis.previous_root.revision)?,
                        ),
                        ("current", revision(stale_record.observed_root.revision)?),
                    ],
                )?,
            )?;
        }
        if let Some(receipt) = &record.committed {
            let coordinate = read
                .revisions
                .get(&receipt.result.revision)
                .filter(|revision| revision.event_id == receipt.event_id)
                .ok_or_else(|| {
                    unavailable(
                        "reading a committed revision",
                        format!(
                            "revision {} is not {}",
                            receipt.result.revision, receipt.event_id
                        ),
                    )
                })?;
            out.push(
                receipt.event_id,
                occurrence(
                    "ekr.kernel.RevisionCommitted",
                    vec![
                        ("event_id", text(receipt.event_id)),
                        ("record_hash", text(coordinate.record_hash)),
                        ("transaction_id", text(proposal.transaction_id)),
                        ("revision_id", text(receipt.revision_id)),
                        ("number", revision(receipt.result.revision)?),
                        ("knowledge_root", text(receipt.result.knowledge_root)),
                    ],
                )?,
            )?;
        }
    }
    Ok(out)
}

// ---- views -----------------------------------------------------------------------------------

fn state_name(state: TransactionState) -> &'static str {
    match state {
        TransactionState::Proposed => "Proposed",
        TransactionState::Validated => "Validated",
        TransactionState::Committed => "Committed",
        TransactionState::Rejected => "Rejected",
        TransactionState::Stale => "Stale",
    }
}

fn transaction_row(
    read: &VerifiedRead,
    record: &TransactionRecord,
) -> Result<ViewRow, TargetError> {
    let proposal = &record.proposal;
    let validation = record.validation.as_ref();
    let basis = match (validation, &record.rejection) {
        (Some(receipt), _) => Some(basis_node(&receipt.basis)?),
        (None, Some(rejection)) => Some(basis_node(&rejection.requested_basis)?),
        (None, None) => None,
    };
    let terminal = if let Some(receipt) = &record.committed {
        Some(
            read.revisions
                .get(&receipt.result.revision)
                .ok_or_else(|| unavailable("reading a committed revision", receipt.revision_id))?
                .record_hash,
        )
    } else if let Some(rejection) = &record.rejection {
        Some(retained(read, rejection.to_bytes())?)
    } else if let Some(stale_record) = &record.stale {
        Some(retained(read, stale_record.to_bytes())?)
    } else {
        None
    };
    Ok(BTreeMap::from([
        ("transaction_id".to_owned(), text(proposal.transaction_id)),
        ("proposer".to_owned(), text(proposal.submitter)),
        ("document_hash".to_owned(), text(proposal.document_hash)),
        (
            "proposal_record_hash".to_owned(),
            text(record.proposal_record_hash),
        ),
        (
            "canonical_transaction_hash".to_owned(),
            optional(proposal.canonical_transaction_hash.map(text)),
        ),
        (
            "canonical_operations_hash".to_owned(),
            optional(proposal.canonical_operations_hash.map(text)),
        ),
        (
            "operation_count".to_owned(),
            integer(proposal.operation_count)?,
        ),
        ("evidence_hash".to_owned(), text(proposal.evidence_hash)),
        ("validation_basis".to_owned(), optional(basis)),
        (
            "validated_against".to_owned(),
            optional(
                validation
                    .map(|receipt| revision(receipt.basis.previous_root.revision))
                    .transpose()?,
            ),
        ),
        (
            "validation_hash".to_owned(),
            optional(validation.map(|receipt| text(receipt.validation_hash))),
        ),
        (
            "validation_record_hash".to_owned(),
            optional(record.validation_record_hash.map(text)),
        ),
        (
            "terminal_record_hash".to_owned(),
            optional(terminal.map(text)),
        ),
        (
            "state".to_owned(),
            Node::Text(state_name(record.state()).to_owned()),
        ),
    ]))
}

fn revision_rows(read: &VerifiedRead) -> Result<Vec<ViewRow>, TargetError> {
    let producers: BTreeMap<_, _> = read
        .transactions
        .values()
        .filter_map(|record| {
            record
                .committed
                .as_ref()
                .map(|receipt| (receipt.revision_id, record.proposal.transaction_id))
        })
        .collect();
    let mut rows = Vec::new();
    let mut parent = None;
    for (number, coordinate) in &read.revisions {
        rows.push(BTreeMap::from([
            ("revision_id".to_owned(), text(coordinate.revision_id)),
            ("number".to_owned(), revision(*number)?),
            ("parent".to_owned(), optional(parent.map(text))),
            (
                "ontology_root".to_owned(),
                text(coordinate.root.ontology_root),
            ),
            (
                "knowledge_root".to_owned(),
                text(coordinate.root.knowledge_root),
            ),
            (
                "evidence_root".to_owned(),
                text(coordinate.root.evidence_root),
            ),
            ("agent_root".to_owned(), text(coordinate.root.agent_root)),
            (
                "transaction_id".to_owned(),
                optional(producers.get(&coordinate.revision_id).map(text)),
            ),
            ("committed_at".to_owned(), instant(coordinate.committed_at)?),
            ("state".to_owned(), Node::Text("Committed".to_owned())),
        ]));
        parent = Some(coordinate.revision_id);
    }
    Ok(rows)
}

// ---- lossless record projection --------------------------------------------------------------

fn text(value: impl ToString) -> Node {
    Node::Text(value.to_string())
}

fn optional(value: Option<Node>) -> Node {
    value.unwrap_or(Node::Null)
}

/// An exact native integer; no intermediate floating-point value exists anywhere on this path.
fn integer(value: u64) -> Result<Node, TargetError> {
    i64::try_from(value)
        .map(|exact| Node::Number(Number::from(exact)))
        .map_err(|_| unavailable("projecting an integer", format!("{value} exceeds i64")))
}

fn revision(number: RevisionNumber) -> Result<Node, TargetError> {
    integer(number.get())
}

/// RFC 3339 in UTC with millisecond precision, so no retained millisecond is dropped.
fn instant(at: Timestamp) -> Result<Node, TargetError> {
    let moment =
        time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(at.millis()) * 1_000_000)
            .map_err(|e| unavailable("projecting a timestamp", e))?;
    Ok(Node::Text(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        moment.year(),
        u8::from(moment.month()),
        moment.day(),
        moment.hour(),
        moment.minute(),
        moment.second(),
        moment.millisecond()
    )))
}

/// Parses `YYYY-MM-DDTHH:MM:SS[.mmm]Z`, or the CLI's own `--valid-at` spellings.
fn parse_instant(value: &str) -> Result<Timestamp, TargetError> {
    let refuse = || {
        unavailable(
            "reading a timestamp",
            format!("{value:?} is not RFC 3339 UTC"),
        )
    };
    let Some((date, rest)) = value.split_once('T') else {
        return crate::host::parse_valid_at(value).map_err(|_| refuse());
    };
    let clock = rest.strip_suffix('Z').ok_or_else(refuse)?;
    let (clock, millis) = match clock.split_once('.') {
        Some((clock, fraction)) if fraction.len() == 3 => (clock, fraction),
        Some(_) => return Err(refuse()),
        None => (clock, "000"),
    };
    let day = crate::host::parse_valid_at(date).map_err(|_| refuse())?;
    let parts: Vec<&str> = clock.split(':').collect();
    let digits = |part: &str, max: u32| -> Result<i64, TargetError> {
        if part.len() != 2 || !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err(refuse());
        }
        let value: u32 = part.parse().map_err(|_| refuse())?;
        if value > max {
            return Err(refuse());
        }
        Ok(i64::from(value))
    };
    let [hour, minute, second] = parts.as_slice() else {
        return Err(refuse());
    };
    if !millis.bytes().all(|b| b.is_ascii_digit()) {
        return Err(refuse());
    }
    let millis: i64 = millis.parse().map_err(|_| refuse())?;
    let offset = ((digits(hour, 23)? * 60 + digits(minute, 59)?) * 60 + digits(second, 59)?)
        * 1_000
        + millis;
    day.millis()
        .checked_add(offset)
        .map(Timestamp::from_millis)
        .ok_or_else(refuse)
}

/// Standard padded base64, the spelling ESS admits for `Bytes`.
fn bytes(value: &[u8]) -> Node {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(value.len().div_ceil(3) * 4);
    for chunk in value.chunks(3) {
        let triple = chunk
            .iter()
            .enumerate()
            .fold(0_u32, |acc, (i, b)| acc | (u32::from(*b) << (16 - 8 * i)));
        for position in 0..4 {
            if position <= chunk.len() {
                let index = (triple >> (18 - 6 * position)) & 0x3f;
                out.push(char::from(ALPHABET[index as usize]));
            } else {
                out.push('=');
            }
        }
    }
    Node::Text(out)
}

fn record(fields: Vec<(&str, Node)>) -> Node {
    Node::Map(
        fields
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect(),
    )
}

/// An ESS tagged union: the variant under the declared tag `kind`, its content under `value`.
fn union(kind: &str, content: Node) -> Node {
    record(vec![
        ("kind", Node::Text(kind.to_owned())),
        ("value", content),
    ])
}

fn response(result: Node) -> BTreeMap<String, Node> {
    BTreeMap::from([("result".to_owned(), result)])
}

fn root(value: &Root) -> Result<Node, TargetError> {
    Ok(record(vec![
        ("revision", revision(value.revision)?),
        ("parent", optional(value.parent.map(text))),
        ("ontology_root", text(value.ontology_root)),
        ("knowledge_root", text(value.knowledge_root)),
        ("evidence_root", text(value.evidence_root)),
        ("agent_root", text(value.agent_root)),
        ("transaction", text(value.transaction)),
    ]))
}

fn seed_result(value: &SeedResultV1) -> Result<Node, TargetError> {
    Ok(record(vec![
        ("format", text(&value.format)),
        ("event_id", text(value.event_id)),
        ("revision_id", text(value.revision_id)),
        ("seed_hash", text(value.seed_hash)),
        ("authority_root", text(value.authority_root)),
        ("committed_at", instant(value.committed_at)?),
        ("result", root(&value.result)?),
        ("result_hash", text(value.result_hash)),
    ]))
}

fn proposal(value: &ProposalRecordV1) -> Result<Node, TargetError> {
    Ok(record(vec![
        ("format", text(&value.format)),
        ("event_id", text(value.event_id)),
        ("submitted_at", instant(value.submitted_at)?),
        ("submitter", text(value.submitter)),
        ("document_hash", text(value.document_hash)),
        ("document_bytes", bytes(&value.document_bytes)),
        ("transaction_id", text(value.transaction_id)),
        ("operation_count", integer(value.operation_count)?),
        ("evidence_hash", text(value.evidence_hash)),
        (
            "canonical_transaction_hash",
            optional(value.canonical_transaction_hash.map(text)),
        ),
        (
            "canonical_operations_hash",
            optional(value.canonical_operations_hash.map(text)),
        ),
    ]))
}

fn basis_node(value: &ValidationBasisV1) -> Result<Node, TargetError> {
    Ok(record(vec![
        ("format", text(&value.format)),
        ("graph_root_id", text(value.graph_root_id)),
        ("previous_revision_id", text(value.previous_revision_id)),
        ("previous_event_id", text(value.previous_event_id)),
        ("previous_record_hash", text(value.previous_record_hash)),
        ("previous_root", root(&value.previous_root)?),
        ("previous_root_hash", text(value.previous_root_hash)),
        ("seed_hash", text(value.seed_hash)),
        ("ontology_root", text(value.ontology_root)),
        ("authority_root", text(value.authority_root)),
        (
            "validation_profile_hash",
            text(value.validation_profile_hash),
        ),
    ]))
}

fn validation(value: &ValidationReceiptV1) -> Result<Node, TargetError> {
    Ok(record(vec![
        ("format", text(&value.format)),
        ("event_id", text(value.event_id)),
        ("proposed_event_id", text(value.proposed_event_id)),
        ("proposal_record_hash", text(value.proposal_record_hash)),
        ("transaction_hash", text(value.transaction_hash)),
        ("operations_hash", text(value.operations_hash)),
        ("evidence_hash", text(value.evidence_hash)),
        ("operation_count", integer(value.operation_count)?),
        ("basis", basis_node(&value.basis)?),
        (
            "validators",
            Node::Seq(value.validators.iter().map(text).collect()),
        ),
        ("validated_at", instant(value.validated_at)?),
        ("validation_hash", text(value.validation_hash)),
    ]))
}

fn rejection(value: &RejectionRecordV1) -> Result<Node, TargetError> {
    Ok(record(vec![
        ("format", text(&value.format)),
        ("event_id", text(value.event_id)),
        ("proposed_event_id", text(value.proposed_event_id)),
        ("proposal_record_hash", text(value.proposal_record_hash)),
        ("requested_basis", basis_node(&value.requested_basis)?),
        ("validator", text(value.validator)),
        ("rejected_at", instant(value.rejected_at)?),
        (
            "issues",
            Node::Seq(
                value
                    .issues
                    .iter()
                    .map(|issue| {
                        record(vec![
                            ("id", text(issue.id)),
                            ("transaction_id", text(issue.transaction_id)),
                            ("validator", text(format!("{:?}", issue.validator))),
                            ("code", text(&issue.code)),
                            ("message", text(&issue.message)),
                        ])
                    })
                    .collect(),
            ),
        ),
    ]))
}

fn commit_receipt(value: &CommitReceiptV1) -> Result<Node, TargetError> {
    Ok(record(vec![
        ("format", text(&value.format)),
        ("event_id", text(value.event_id)),
        ("revision_id", text(value.revision_id)),
        ("proposal", proposal(&value.proposal)?),
        ("validation", validation(&value.validation)?),
        ("validation_record_hash", text(value.validation_record_hash)),
        ("committer", text(value.committer)),
        ("committed_at", instant(value.committed_at)?),
        ("result", root(&value.result)?),
        ("result_hash", text(value.result_hash)),
    ]))
}

fn stale(value: &StaleRecordV1) -> Result<Node, TargetError> {
    Ok(record(vec![
        ("format", text(&value.format)),
        ("event_id", text(value.event_id)),
        ("validation_record_hash", text(value.validation_record_hash)),
        ("expected_basis", basis_node(&value.expected_basis)?),
        ("observed_revision_id", text(value.observed_revision_id)),
        ("observed_event_id", text(value.observed_event_id)),
        ("observed_record_hash", text(value.observed_record_hash)),
        ("observed_root", root(&value.observed_root)?),
        ("observed_root_hash", text(value.observed_root_hash)),
        ("stale_at", instant(value.stale_at)?),
    ]))
}

// ---- requests --------------------------------------------------------------------------------

/// The ESS actor each kernel command is granted to, and the host identity the CLI submits as.
fn check_actor(command: &str, request: &SemanticCommandRequest) -> Result<(), TargetError> {
    let granted = match command {
        SEED | SNAPSHOT | EXPLAIN => "ekr.kernel.Operator",
        PROPOSE => "ekr.kernel.Proposer",
        VALIDATE => "ekr.kernel.Validator",
        COMMIT => "ekr.kernel.Committer",
        _ => return Ok(()),
    };
    match &request.actor {
        Some(actor) if actor.to_string() != granted => Err(TargetError::unsupported(
            format!("`{command}` as `{actor}`"),
            format!("the host binds `{command}` to `{granted}`"),
        )),
        _ => Ok(()),
    }
}

fn text_input(request: &SemanticCommandRequest, field: &str) -> Result<String, TargetError> {
    match request.input.get(field) {
        Some(Node::Text(value)) => Ok(value.clone()),
        other => Err(unavailable(
            &format!("reading `{field}`"),
            format!("expected text, found {:?}", other.map(Node::type_name)),
        )),
    }
}

fn parse_text<T: std::str::FromStr>(
    request: &SemanticCommandRequest,
    field: &str,
) -> Result<T, TargetError>
where
    T::Err: std::fmt::Display,
{
    text_input(request, field)?
        .parse()
        .map_err(|e: T::Err| unavailable(&format!("reading `{field}`"), e))
}

fn transaction_input(request: &SemanticCommandRequest) -> Result<TransactionId, TargetError> {
    parse_text(request, "transaction_id")
}

fn revision_input(
    request: &SemanticCommandRequest,
    field: &str,
) -> Result<Option<RevisionNumber>, TargetError> {
    match request.input.get(field) {
        None | Some(Node::Null) => Ok(None),
        Some(Node::Number(number)) => number
            .as_i64()
            .and_then(|exact| u64::try_from(exact).ok())
            .map(|exact| Some(RevisionNumber::new(exact)))
            .ok_or_else(|| {
                unavailable(
                    &format!("reading `{field}`"),
                    format!("{number} is not a revision number"),
                )
            }),
        Some(other) => Err(unavailable(
            &format!("reading `{field}`"),
            format!("{} is not a revision number", other.type_name()),
        )),
    }
}

fn require_absent_revision(runtime: &Runtime, at: RevisionNumber) -> Result<(), TargetError> {
    let read = runtime
        .read(None)
        .map_err(|e| unavailable("establishing an absent revision", e))?;
    if read.revisions.contains_key(&at) {
        return Err(unavailable(
            "establishing an absent revision",
            format!("revision {at} exists"),
        ));
    }
    Ok(())
}

fn state_conflict(state: TransactionState) -> Result<DeclaredErrorValue, TargetError> {
    Ok(error("ekr.kernel.TransactionStateConflict")?
        .with("state", Node::Text(state_name(state).to_owned())))
}

fn error(name: &str) -> Result<DeclaredErrorValue, TargetError> {
    name.parse()
        .map(DeclaredErrorValue::new)
        .map_err(|e| unavailable("naming a declared error", format!("{name}: {e}")))
}

fn event_ref(name: &str) -> Result<ess_conformance::scenario::EventRef, TargetError> {
    name.parse()
        .map_err(|e| unavailable("naming a declared event", format!("{name}: {e}")))
}

fn outcome_ref(
    command: &str,
    outcome: &str,
) -> Result<ess_conformance::scenario::OutcomeRef, TargetError> {
    serde_json::from_value(serde_json::json!({ "command": command, "outcome": outcome })).map_err(
        |e| {
            unavailable(
                "naming a declared outcome",
                format!("{command}/{outcome}: {e}"),
            )
        },
    )
}

/// A manifest file name: one plain path component.
fn simple(name: &str) -> Result<&str, String> {
    let mut components = Path::new(name).components();
    match (components.next(), components.next()) {
        (Some(Component::Normal(_)), None) => Ok(name),
        _ => Err(format!("{name:?} is not a plain fixture file name")),
    }
}

/// A synthesized document path: relative, non-empty and without any parent or root component.
fn contained(relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if relative.is_empty()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!("{relative:?} is not a contained relative path"));
    }
    Ok(path.to_path_buf())
}
