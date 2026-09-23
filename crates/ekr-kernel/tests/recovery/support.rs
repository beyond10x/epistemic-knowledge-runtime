//! Fixtures, the port-level fault probe and native readers for `tests/recovery.rs`.
//!
//! The probe wraps the real provider store at the kernel's `RevisionLog` port. It answers
//! `UnknownCommit` before or after delegating the exact elected native request; it models a lost
//! or unsent response and is **not** evidence of a native crash.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{Cardinality, NodeType, PropertyDefinition, Value, ValueType};
use ekr_store::{
    NativePublicationRequest, Publication, PublicationCommandKey, PublicationPreparationV1,
    StoreError,
};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::rc::Rc;

pub const TENANT: &str = "test";

pub fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}
pub fn anchor() -> AuthorityStateV1 {
    let c = context();
    AuthorityStateV1 {
        format: "ekr.authority-state/1".into(),
        agents: [(c.operator, "operator"), (c.validator, "validator")]
            .into_iter()
            .map(|(id, name)| {
                (
                    id,
                    Agent {
                        id,
                        name: name.into(),
                        capabilities: BTreeSet::new(),
                    },
                )
            })
            .collect(),
        validation_profile: ValidationProfileV1::deterministic(c.validator),
    }
}
pub fn open(path: &Path, file: bool) -> Runtime {
    if file {
        Runtime::file(path, TENANT, context(), anchor())
    } else {
        Runtime::sqlite(&path.join("state.db"), TENANT, context(), anchor())
    }
    .unwrap()
}
pub fn seed_fixture() -> SeedDocument {
    let mut seed =
        SeedDocument::from_yaml(include_str!("../fixtures/seed-minimal-v2.yaml")).unwrap();
    let type_id = "00000000-0000-4000-8000-000000000005".parse().unwrap();
    let many = "00000000-0000-4000-8000-000000000006".parse().unwrap();
    let mut declared = NodeType::new(type_id, "Subject");
    let mut definition = PropertyDefinition::new(many, "labels", ValueType::String);
    definition.cardinality = Cardinality::Many;
    declared.properties.insert(many, definition);
    seed.ontology.node_types.push(declared);
    let node = Node::<Value>::new(NodeId::mint(), seed.graph.root.id, type_id, "seed");
    let bytes = b"synthetic human evidence".to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let evidence = Evidence {
        id: EvidenceId::mint(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::from_basis_points(10000).unwrap(),
    };
    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id: seed.graph.root.id,
        subject: Subject::Node(node.id),
        predicate: Predicate::Property(many),
        object: Object::Value(Value::String("seed".into())),
        evidence: BTreeSet::from([evidence.id]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    seed.graph.nodes.insert(node.id, node);
    seed.graph.assertions.insert(assertion.id, assertion);
    seed.graph.evidence.insert(evidence.id, evidence);
    seed.evidence_payloads.insert(hash, bytes);
    seed
}
/// A transaction the deterministic validators accept against the seed, or refuse by type.
pub fn transaction(seed: &SeedDocument, admissible: bool) -> GraphTransaction {
    let id = NodeId::mint();
    let ty = &seed.ontology.node_types[0];
    let many = *ty.properties.keys().next().unwrap();
    let evidence = *seed.graph.evidence.keys().next().unwrap();
    let assertion = Assertion {
        id: AssertionId::mint(),
        root_id: seed.graph.root.id,
        subject: Subject::Node(id),
        predicate: Predicate::Property(many),
        object: Object::Value(Value::String("changed".into())),
        evidence: BTreeSet::from([evidence]),
        proposed_by: context().operator,
        assessment: Assessment::Proposed,
        lifecycle: AssertionLifecycle::Active,
        valid_time: TemporalRange::UNBOUNDED,
        transaction_time: TransactionTime::since(Timestamp::EPOCH),
    };
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![
            GraphOperation::AddAssertion(Box::new(assertion)),
            GraphOperation::CreateNode(NodeDraft {
                id,
                root_id: seed.graph.root.id,
                type_id: if admissible { ty.id } else { TypeId::mint() },
                canonical_name: "created".into(),
                properties: BTreeMap::from([(many, vec![Value::String("same".into())])]),
            }),
        ],
        evidence: BTreeSet::from([evidence]),
    }
}
pub fn encode(tx: &GraphTransaction) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    serde_yaml_ng::to_string(&Wire {
        format: "ekr.transaction-document/1",
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}

/// Every decision kind §94 elects, including the Rejected and Stale outcomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Seed,
    Propose,
    Validated,
    Rejected,
    Committed,
    Stale,
}
pub const KINDS: [Kind; 6] = [
    Kind::Seed,
    Kind::Propose,
    Kind::Validated,
    Kind::Rejected,
    Kind::Committed,
    Kind::Stale,
];
impl Kind {
    pub fn event(self) -> &'static str {
        match self {
            Kind::Seed => "ekr.kernel.Seeded",
            Kind::Propose => "ekr.kernel.TransactionProposed",
            Kind::Validated => "ekr.kernel.TransactionValidated",
            Kind::Rejected => "ekr.kernel.TransactionRejected",
            Kind::Committed => "ekr.kernel.RevisionCommitted",
            Kind::Stale => "ekr.kernel.TransactionStale",
        }
    }
    /// The trusted time the command under test samples on its first, elected attempt.
    pub fn time(self) -> i64 {
        match self {
            Kind::Seed => 10,
            Kind::Propose => 20,
            Kind::Validated | Kind::Rejected => 30,
            Kind::Committed | Kind::Stale => 40,
        }
    }
    /// The retained transaction state a later same-command retry names after publication.
    pub fn state(self) -> Option<&'static str> {
        match self {
            Kind::Seed | Kind::Committed => None,
            Kind::Propose => Some("Proposed"),
            Kind::Validated => Some("Validated"),
            Kind::Rejected => Some("Rejected"),
            Kind::Stale => Some("Stale"),
        }
    }
}

/// What a command under test needs, retained on disk so a child process can repeat it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub kind: Kind,
    pub file: bool,
    pub seed_yaml: String,
    pub tx: TransactionId,
    pub document: Vec<u8>,
}
impl Plan {
    pub fn seed(&self) -> SeedDocument {
        SeedDocument::from_yaml(&self.seed_yaml).unwrap()
    }
}
/// Builds, through the ordinary runtime, the state immediately before the command under test.
pub fn stage(path: &Path, file: bool, kind: Kind) -> Plan {
    let seed = seed_fixture();
    let tx = transaction(&seed, kind != Kind::Rejected);
    let plan = Plan {
        kind,
        file,
        seed_yaml: serde_yaml_ng::to_string(&seed).unwrap(),
        tx: tx.id,
        document: encode(&tx),
    };
    let kernel = open(path, file);
    if kind != Kind::Seed {
        kernel
            .seed(seed.clone(), || Timestamp::from_millis(10))
            .unwrap();
    }
    if matches!(
        kind,
        Kind::Validated | Kind::Rejected | Kind::Committed | Kind::Stale
    ) {
        kernel
            .propose(&plan.document, context().operator, || {
                Timestamp::from_millis(20)
            })
            .unwrap();
    }
    if matches!(kind, Kind::Committed | Kind::Stale) {
        kernel
            .validate(tx.id, RevisionNumber::SEED, || Timestamp::from_millis(30))
            .unwrap();
    }
    if kind == Kind::Stale {
        let other = transaction(&seed, true);
        kernel
            .propose(&encode(&other), context().operator, || {
                Timestamp::from_millis(20)
            })
            .unwrap();
        kernel
            .validate(other.id, RevisionNumber::SEED, || {
                Timestamp::from_millis(30)
            })
            .unwrap();
        kernel
            .commit(other.id, context().operator, || Timestamp::from_millis(35))
            .unwrap();
    }
    plan
}

/// A command's observable answer, comparable across a process boundary.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Outcome {
    /// The complete retained record of the decision the command reported.
    Record(serde_json::Value),
    /// `UnknownCommit`: the publication outcome is uncertain.
    Unknown,
    /// `TransactionStateConflict` naming the actual retained state.
    State(String),
    /// A named store refusal: the `StoreError::Document` code, or the variant name.
    Refused(String),
    /// Anything else, verbatim.
    Other(String),
}
fn store_outcome(error: &StoreError) -> Outcome {
    match error {
        StoreError::UnknownCommit => Outcome::Unknown,
        StoreError::Document(code) => Outcome::Refused(code.clone()),
        StoreError::PublicationInputConflict => Outcome::Refused("PublicationInputConflict".into()),
        StoreError::Conflict => Outcome::Refused("Conflict".into()),
        other => Outcome::Other(format!("{other:?}")),
    }
}
fn commit_outcome(error: &CommitError) -> Outcome {
    match error {
        CommitError::Store(error) => store_outcome(error),
        CommitError::TransactionStateConflict { state, .. } => Outcome::State(format!("{state:?}")),
        other => Outcome::Other(format!("{other:?}")),
    }
}
fn record<T: Serialize>(value: &T) -> Outcome {
    Outcome::Record(serde_json::to_value(value).unwrap())
}

/// The four command handlers, over the public runtime or a probed kernel alike.
pub trait Commands {
    fn seed_command(&self, s: SeedDocument, now: Box<dyn FnOnce() -> Timestamp>) -> Outcome;
    fn propose_command(&self, b: &[u8], now: Box<dyn FnOnce() -> Timestamp>) -> Outcome;
    fn validate_command(&self, id: TransactionId, now: Box<dyn FnOnce() -> Timestamp>) -> Outcome;
    fn commit_command(&self, id: TransactionId, now: Box<dyn FnOnce() -> Timestamp>) -> Outcome;
    fn retained_record(&self, kind: Kind, id: TransactionId) -> Outcome;
}
macro_rules! commands {
    () => {
        fn seed_command(&self, s: SeedDocument, now: Box<dyn FnOnce() -> Timestamp>) -> Outcome {
            match self.seed(s, now) {
                Ok(result) => record(&result),
                Err(SeedError::Store(error)) => store_outcome(&error),
                Err(error) => Outcome::Other(format!("{error:?}")),
            }
        }
        fn propose_command(&self, b: &[u8], now: Box<dyn FnOnce() -> Timestamp>) -> Outcome {
            match self.propose(b, context().operator, now) {
                Ok(result) => record(&result),
                Err(error) => commit_outcome(&error),
            }
        }
        fn validate_command(
            &self,
            id: TransactionId,
            now: Box<dyn FnOnce() -> Timestamp>,
        ) -> Outcome {
            match self.validate(id, RevisionNumber::SEED, now) {
                Ok(ValidationCommandResult::Validated(r)) => record(&r),
                Ok(ValidationCommandResult::Rejected(r)) => record(&r),
                Err(error) => commit_outcome(&error),
            }
        }
        fn commit_command(
            &self,
            id: TransactionId,
            now: Box<dyn FnOnce() -> Timestamp>,
        ) -> Outcome {
            match self.commit(id, context().operator, now) {
                Ok(CommitCommandResult::Committed(r)) => record(&r),
                Ok(CommitCommandResult::Stale(r)) => record(&r),
                Err(error) => commit_outcome(&error),
            }
        }
        fn retained_record(&self, kind: Kind, id: TransactionId) -> Outcome {
            if kind == Kind::Seed {
                return match self.read(None) {
                    Ok(read) => record(&read.seed),
                    Err(error) => commit_outcome(&error),
                };
            }
            let transactions = match self.transactions() {
                Ok(held) => held,
                Err(error) => return commit_outcome(&error),
            };
            let Some(held) = transactions.get(&id) else {
                return Outcome::Other("transaction not retained".into());
            };
            match kind {
                Kind::Seed => unreachable!(),
                Kind::Propose => record(&held.proposal),
                Kind::Validated => record(&held.validation),
                Kind::Rejected => record(&held.rejection),
                Kind::Committed => record(&held.committed),
                Kind::Stale => record(&held.stale),
            }
        }
    };
}
impl Commands for Runtime {
    commands!();
}
impl<S: ekr_store::RevisionLog + ekr_store::ObjectStore + ekr_store::Initialize> Commands
    for Commit<S>
{
    commands!();
}
/// Runs exactly the command under test at the given trusted time.
pub fn run(kernel: &dyn Commands, plan: &Plan, now: Box<dyn FnOnce() -> Timestamp>) -> Outcome {
    match plan.kind {
        Kind::Seed => kernel.seed_command(plan.seed(), now),
        Kind::Propose => kernel.propose_command(&plan.document, now),
        Kind::Validated | Kind::Rejected => kernel.validate_command(plan.tx, now),
        Kind::Committed | Kind::Stale => kernel.commit_command(plan.tx, now),
    }
}
pub fn at(millis: i64) -> Box<dyn FnOnce() -> Timestamp> {
    Box::new(move || Timestamp::from_millis(millis))
}
pub fn no_clock(why: &'static str) -> Box<dyn FnOnce() -> Timestamp> {
    Box::new(move || panic!("{why}: a retry sampled the trusted clock"))
}
/// The elected record, decoded strictly by its own codec, as the command would report it.
pub fn elected_record(kind: Kind, prepared: &PublicationPreparationV1) -> Outcome {
    let bytes = &prepared.decision.objects[&prepared.decision.event.record_hash].bytes;
    match kind {
        Kind::Seed => record(&SeedResultV1::from_bytes(bytes).unwrap()),
        Kind::Propose => record(&ProposalRecordV1::from_bytes(bytes).unwrap()),
        Kind::Validated => record(&ValidationReceiptV1::from_bytes(bytes).unwrap()),
        Kind::Rejected => record(&RejectionRecordV1::from_bytes(bytes).unwrap()),
        Kind::Committed => record(&CommitReceiptV1::from_bytes(bytes).unwrap()),
        Kind::Stale => record(&StaleRecordV1::from_bytes(bytes).unwrap()),
    }
}

/// Port-level fault injection; see the module documentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fault {
    Pass,
    /// Elected, never sent: the resume answers `UnknownCommit` without delegating.
    BeforeWrite,
    /// Sent and applied, response lost: delegates the exact request, then answers `UnknownCommit`.
    AfterWrite,
}
type Hook = Rc<RefCell<Option<Box<dyn FnMut()>>>>;
#[derive(Clone)]
pub struct Hooks {
    pub fault: Rc<Cell<Fault>>,
    /// Every preparation handed to `resume`, in order.
    pub resumed: Rc<RefCell<Vec<PublicationPreparationV1>>>,
    /// Every candidate decision handed to `prepare`, in order.
    pub candidates: Rc<RefCell<Vec<Publication>>>,
    /// Runs once, immediately before the first `prepare` is delegated.
    pub before_prepare: Hook,
    /// Runs once, at the first kernel-authority call made *inside* a delegated `prepare`. Over an
    /// empty slot that call is the store's admission of its own candidate: after the store read
    /// the slot and before its conditional selection write, the window a real race needs.
    pub inside_prepare: Hook,
    /// Set when `inside_prepare` actually ran inside a delegated `prepare`.
    pub fired_inside_prepare: Rc<Cell<bool>>,
    preparing: Rc<Cell<bool>>,
}
impl Hooks {
    pub fn new(fault: Fault) -> Self {
        Self {
            fault: Rc::new(Cell::new(fault)),
            resumed: Rc::default(),
            candidates: Rc::default(),
            before_prepare: Rc::default(),
            inside_prepare: Rc::default(),
            fired_inside_prepare: Rc::default(),
            preparing: Rc::default(),
        }
    }
}
pub struct Probe<S> {
    inner: S,
    hooks: Hooks,
}
impl<S: ekr_store::RevisionLog> ekr_store::RevisionLog for Probe<S> {
    fn preparation(
        &self,
        key: &PublicationCommandKey,
    ) -> Result<Option<PublicationPreparationV1>, StoreError> {
        self.inner.preparation(key)
    }
    fn prepare(
        &self,
        key: &PublicationCommandKey,
        input: ContentHash,
        decision: &Publication,
        previous: Option<&PublicationPreparationV1>,
    ) -> Result<PublicationPreparationV1, StoreError> {
        self.hooks.candidates.borrow_mut().push(decision.clone());
        let hook = self.hooks.before_prepare.borrow_mut().take();
        if let Some(mut hook) = hook {
            hook();
        }
        self.hooks.preparing.set(true);
        let result = self.inner.prepare(key, input, decision, previous);
        self.hooks.preparing.set(false);
        result
    }
    fn resume(&self, p: &PublicationPreparationV1) -> Result<ekr_store::Appended, StoreError> {
        self.hooks.resumed.borrow_mut().push(p.clone());
        match self.hooks.fault.replace(Fault::Pass) {
            Fault::Pass => self.inner.resume(p),
            Fault::BeforeWrite => Err(StoreError::UnknownCommit),
            Fault::AfterWrite => {
                let _written = self.inner.resume(p)?;
                Err(StoreError::UnknownCommit)
            }
        }
    }
    fn history(&self) -> Result<ekr_store::RetainedHistory, StoreError> {
        self.inner.history()
    }
    fn history_at(&self, r: RevisionNumber) -> Result<ekr_store::RetainedHistory, StoreError> {
        self.inner.history_at(r)
    }
    fn publish(&self, p: &Publication) -> Result<ekr_store::Appended, StoreError> {
        self.inner.publish(p)
    }
    fn seed_bytes(&self) -> Result<Option<Vec<u8>>, StoreError> {
        self.inner.seed_bytes()
    }
    fn fold(&self) -> Result<CanonicalGraph, StoreError> {
        self.inner.fold()
    }
    fn head(&self) -> Result<Option<Root>, StoreError> {
        self.inner.head()
    }
    fn replay(&self, r: RevisionNumber) -> Result<CanonicalGraph, StoreError> {
        self.inner.replay(r)
    }
}
impl<S: ekr_store::Initialize> ekr_store::Initialize for Probe<S> {
    fn initialize(&self, p: &Publication) -> Result<ekr_store::Appended, StoreError> {
        self.inner.initialize(p)
    }
}
impl<S: ekr_store::ObjectStore> ekr_store::ObjectStore for Probe<S> {
    fn put(
        &self,
        c: ekr_store::StorageClass,
        b: &[u8],
        t: Timestamp,
    ) -> Result<ekr_store::StoredObject, StoreError> {
        self.inner.put(c, b, t)
    }
    fn get(&self, h: &ContentHash) -> Result<Option<Vec<u8>>, StoreError> {
        self.inner.get(h)
    }
}
/// Opens a probed kernel over the real provider with the real kernel authority.
/// The real kernel authority, unchanged, with the `inside_prepare` seam in front of it.
struct Interleaved {
    inner: KernelAuthority,
    hooks: Hooks,
}
impl Interleaved {
    fn fire(&self) {
        if self.hooks.preparing.get() {
            let hook = self.hooks.inside_prepare.borrow_mut().take();
            if let Some(mut hook) = hook {
                self.hooks.preparing.set(false);
                hook();
                self.hooks.fired_inside_prepare.set(true);
                self.hooks.preparing.set(true);
            }
        }
    }
}
impl ekr_store::CommitAuthority for Interleaved {
    fn required_objects(
        &self,
        history: &ekr_store::RetainedHistory,
    ) -> Result<BTreeSet<ContentHash>, StoreError> {
        self.fire();
        self.inner.required_objects(history)
    }
    fn replay(
        &self,
        history: &ekr_store::RetainedHistory,
        ontology: Option<&ekr_ontology::Ontology>,
        revision: Option<RevisionNumber>,
    ) -> Result<Option<ekr_store::AdmittedRevision>, StoreError> {
        self.fire();
        self.inner.replay(history, ontology, revision)
    }
}
pub fn probed<T>(path: &Path, file: bool, hooks: &Hooks, f: impl FnOnce(&dyn Commands) -> T) -> T {
    let seam = |authority| Interleaved {
        inner: authority,
        hooks: hooks.clone(),
    };
    if file {
        let kernel = Commit::over_with_authority(context(), anchor(), |authority| {
            Ok(Probe {
                inner: ekr_store::FileStore::file(path, TENANT, None)?.under(seam(authority)),
                hooks: hooks.clone(),
            })
        })
        .unwrap();
        f(&kernel)
    } else {
        let kernel = Commit::over_with_authority(context(), anchor(), |authority| {
            Ok(Probe {
                inner: ekr_store::SqliteStore::sqlite(&path.join("state.db"), TENANT, None)?
                    .under(seam(authority)),
                hooks: hooks.clone(),
            })
        })
        .unwrap();
        f(&kernel)
    }
}

pub fn native(
    path: &Path,
    file: bool,
) -> (tokio::runtime::Runtime, Box<dyn eventlog_core::EventStore>) {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let provider: Box<dyn eventlog_core::EventStore> = executor.block_on(async {
        if file {
            Box::new(eventlog_file::FileEventStore::open(path).await.unwrap())
                as Box<dyn eventlog_core::EventStore>
        } else {
            Box::new(
                eventlog_sqlite::SqliteEventStore::open(
                    &path.join("state.db").to_string_lossy(),
                    "ekr",
                )
                .await
                .unwrap(),
            )
        }
    });
    (executor, provider)
}
pub fn tenant() -> eventlog_core::TenantId {
    eventlog_core::TenantId::new(TENANT).unwrap()
}
/// Every native event in feed order, as JSON.
pub fn events(path: &Path, file: bool) -> Vec<serde_json::Value> {
    let (executor, provider) = native(path, file);
    executor.block_on(async {
        let mut result = Vec::new();
        let mut after = 0;
        loop {
            let page = provider.read_feed(&tenant(), after, 100).await.unwrap();
            for event in page.events {
                result.push(serde_json::to_value(event).unwrap());
            }
            if !page.has_more {
                break;
            }
            after = page.next_position;
        }
        result
    })
}
pub fn blob(path: &Path, file: bool, digest: &str) -> Option<Vec<u8>> {
    let (executor, provider) = native(path, file);
    executor
        .block_on(provider.get_blob(&tenant(), digest))
        .unwrap()
}
/// Canonical revision-stream events carrying a given occurrence identity.
pub fn occurrences(events: &[serde_json::Value], event_id: EventId) -> Vec<serde_json::Value> {
    let id = serde_json::to_value(event_id).unwrap();
    events
        .iter()
        .filter(|e| e["stream_type"] == "ekr.revision" && e["data"]["event_id"] == id)
        .cloned()
        .collect()
}
/// Canonical revision-stream events of one kind naming one transaction (or the seed).
pub fn kind_events(
    events: &[serde_json::Value],
    kind: Kind,
    tx: TransactionId,
) -> Vec<serde_json::Value> {
    let tx = serde_json::to_value(tx).unwrap();
    events
        .iter()
        .filter(|e| {
            e["stream_type"] == "ekr.revision"
                && e["name"] == kind.event()
                && (kind == Kind::Seed || e["data"]["payload"]["transaction_id"] == tx)
        })
        .cloned()
        .collect()
}
pub fn prepared_events(events: &[serde_json::Value]) -> Vec<serde_json::Value> {
    events
        .iter()
        .filter(|e| e["name"] == "ekr.store.PublicationPrepared")
        .cloned()
        .collect()
}
pub fn private_key(hash: ContentHash) -> String {
    format!("ekr.private.preparation.{hash}")
}
pub fn slot(key: &PublicationCommandKey) -> String {
    ContentHash::of_bytes(&serde_json::to_vec(key).unwrap()).to_hex()
}
pub fn preparation_hash(prepared: &PublicationPreparationV1) -> ContentHash {
    ContentHash::of_bytes(&serde_json::to_vec(prepared).unwrap())
}

/// Recomputes the provider's own fingerprint over a retained native request.
pub fn fingerprint(request: &NativePublicationRequest) -> String {
    use eventlog_core::{
        AppendGroup, BlobAppendGroup, BlobWrite, CommandMeta, Expected, NewEvent, StreamAppend,
        StreamId, TenantId,
    };
    let appends = request
        .appends
        .iter()
        .map(|append| StreamAppend {
            stream: StreamId::new(
                TenantId::new(append.stream.tenant.clone()).unwrap(),
                &append.stream.stream_type,
                &append.stream.stream_id,
            )
            .unwrap(),
            expected: match append.expected.version {
                None if append.expected.kind == ekr_store::NativeExpectedKind::NoStream => {
                    Expected::NoStream
                }
                Some(version) => Expected::Exact(version),
                None => Expected::Any,
            },
            events: append
                .events
                .iter()
                .map(|e| {
                    NewEvent::new(
                        &e.name,
                        e.schema_version,
                        serde_json::from_slice(&e.data).unwrap(),
                    )
                    .unwrap()
                })
                .collect(),
        })
        .collect();
    let meta = &request.meta;
    let nanos: i128 = meta.occurred_at_unix_nanos.parse().unwrap();
    let group = BlobAppendGroup {
        group: AppendGroup {
            tenant: TenantId::new(request.tenant.clone()).unwrap(),
            appends,
            meta: CommandMeta {
                idempotency_key: meta.idempotency_key.clone(),
                request_hash: meta.request_hash.clone(),
                subject: meta.subject.clone(),
                actor: meta.actor.clone(),
                request_id: meta.request_id.clone(),
                trace_id: meta.trace_id.clone(),
                causation_id: meta.causation_id.clone(),
                causation_depth: meta.causation_depth,
                occurred_at: ::time::OffsetDateTime::from_unix_timestamp_nanos(nanos)
                    .unwrap()
                    .to_offset(
                        ::time::UtcOffset::from_whole_seconds(meta.occurred_at_offset_seconds)
                            .unwrap(),
                    ),
                claim: None,
            },
        },
        blobs: request
            .blobs
            .iter()
            .map(|b| BlobWrite {
                digest: b.digest.clone(),
                bytes: b.bytes.clone(),
            })
            .collect(),
    };
    group.fingerprint().unwrap()
}
/// The first-attempt preparation the store would have built for `decision` in `like`'s slot,
/// reconstructed from public formats. Only valid where every staged object is new.
pub fn would_be(
    like: &PublicationPreparationV1,
    decision: &Publication,
) -> PublicationPreparationV1 {
    use ekr_store::{NativeBlobWrite, NativeExpected, NativeNewEvent, NativeStreamAppend};
    let mut prepared = like.clone();
    prepared.decision = decision.clone();
    let revision = &like.native_request.appends[0];
    let mut appends = vec![NativeStreamAppend {
        stream: revision.stream.clone(),
        expected: revision.expected.clone(),
        events: vec![NativeNewEvent {
            name: decision.event.name().into(),
            schema_version: 2,
            data: serde_json::to_vec(&serde_json::to_value(&decision.event).unwrap()).unwrap(),
        }],
    }];
    for (hash, object) in &decision.objects {
        let mut stream = revision.stream.clone();
        stream.stream_type = "ekr.store.object".into();
        stream.stream_id = hash.to_hex();
        let metadata = serde_json::json!({
            "content_hash": hash,
            "storage_class": object.storage_class,
            "byte_len": object.bytes.len() as u64,
            "stored_at": object.stored_at,
        });
        appends.push(NativeStreamAppend {
            stream,
            expected: NativeExpected {
                kind: ekr_store::NativeExpectedKind::NoStream,
                version: None,
            },
            events: vec![NativeNewEvent {
                name: "ekr.store.ObjectStored".into(),
                schema_version: 2,
                data: serde_json::to_vec(&metadata).unwrap(),
            }],
        });
    }
    prepared.native_request.appends = appends;
    let key = format!("ekr.occurrence.{}.0", decision.event.event_id);
    prepared.native_request.meta.idempotency_key = key.clone();
    prepared.native_request.meta.request_id = key.clone();
    prepared.native_request.meta.trace_id = key;
    prepared.native_request.meta.request_hash = ContentHash::of(&decision.event).to_hex();
    prepared.native_request.blobs = decision
        .objects
        .iter()
        .map(|(hash, object)| NativeBlobWrite {
            digest: hash.to_hex(),
            bytes: object.bytes.clone(),
        })
        .collect();
    prepared.native_fingerprint = fingerprint(&prepared.native_request);
    prepared
}
/// Writes a private preparation record into an empty slot exactly as the store's selection does.
pub fn install_preparation(
    path: &Path,
    file: bool,
    prepared: &PublicationPreparationV1,
    bytes: Option<&[u8]>,
) {
    use eventlog_core::{CommandMeta, Expected, NewEvent, StreamId};
    let hash = preparation_hash(prepared);
    let (executor, provider) = native(path, file);
    executor.block_on(async {
        if let Some(bytes) = bytes {
            provider
                .put_blob(&tenant(), &private_key(hash), bytes)
                .await
                .unwrap();
        }
        let stream =
            StreamId::new(tenant(), "ekr.preparation", slot(&prepared.command_key)).unwrap();
        let key = format!("fixture.{hash}");
        let meta = CommandMeta {
            idempotency_key: key.clone(),
            request_hash: key.clone(),
            subject: "fixture".into(),
            actor: "fixture".into(),
            request_id: key.clone(),
            trace_id: key,
            causation_id: None,
            causation_depth: 0,
            occurred_at: ::time::OffsetDateTime::UNIX_EPOCH,
            claim: None,
        };
        let event = NewEvent::new(
            "ekr.store.PublicationPrepared",
            1,
            serde_json::json!({
                "preparation_hash": hash,
                "attempt_number": prepared.attempt_number,
                "previous_attempt_hash": prepared.previous_attempt_hash,
            }),
        )
        .unwrap();
        provider
            .append(&stream, Expected::NoStream, &[event], &meta)
            .await
            .unwrap();
    });
}
/// Recursively copies a closed provider directory.
pub fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
        }
    }
}
