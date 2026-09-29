//! Adversary pass 1 on `task:checkpoint-cadence-costs-each-commit` (design § 99), on both
//! providers and under validation profiles v1, v2 and v3.
//!
//! - A fixed-seed interleaving of small and large commits, schema changes, rejections, stale
//!   commits (by basis and by race), pending proposals and validated-but-uncommitted transactions,
//!   each command run by a one-shot handle, a full-replay one-shot handle or one of two long-lived
//!   session handles. After every step a fresh open, both session handles and a full replay give
//!   the same head, graph and records; after every commit the newest checkpoint is within the
//!   § 99.2 bound of the head.
//! - Crash-like states: the newest pointer older than the head, naming an older genuine
//!   checkpoint, naming a missing checkpoint, and naming another history's checkpoint.
//! - § 99.1: a handle opened with `--full-replay` admits none, so its first commit writes one,
//!   and from there it counts from the one it wrote.
//! - § 99.1 item 2: a validation against a revision before the retained checkpoint's head makes
//!   the next commit write one.
use ekr_core::*;
use ekr_graph::*;
use ekr_kernel::*;
use ekr_ontology::{EdgeType, NodeType, PropertyDefinition, Value, ValueType};
use ekr_store::RevisionLog;
use serde::Serialize;
use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const TENANT: &str = "cadence";
const RELATES: &str = "00000000-0000-4000-8000-000000000008";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Profile {
    V1,
    V2,
    V3,
}

fn context() -> BootstrapContext {
    BootstrapContext {
        operator: "00000000-0000-4000-8000-000000000003".parse().unwrap(),
        validator: "00000000-0000-4000-8000-000000000004".parse().unwrap(),
    }
}
fn anchor(profile: Profile) -> AuthorityStateV1 {
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
        validation_profile: match profile {
            Profile::V1 => ValidationProfileV1::deterministic(c.validator),
            Profile::V2 => ValidationProfileV1::schema_evolving(c.validator),
            Profile::V3 => ValidationProfileV1::identity_keeping(c.validator),
        },
    }
}
fn open(path: &Path, file: bool, profile: Profile) -> Runtime {
    if file {
        Runtime::file(path, TENANT, context(), anchor(profile))
    } else {
        Runtime::sqlite(&path.join("state.db"), TENANT, context(), anchor(profile))
    }
    .unwrap()
}
fn open_in_full(path: &Path, file: bool, profile: Profile) -> Runtime {
    let mut runtime = open(path, file, profile);
    runtime.set_full_replay(true);
    runtime
}
fn seed_with(evidence_bytes: &[u8]) -> SeedDocument {
    let mut seed = SeedDocument::from_yaml(include_str!("fixtures/seed-minimal-v2.yaml")).unwrap();
    let type_id = "00000000-0000-4000-8000-000000000005".parse().unwrap();
    let label = "00000000-0000-4000-8000-000000000006".parse().unwrap();
    let mut declared = NodeType::new(type_id, "Subject");
    declared.properties.insert(
        label,
        PropertyDefinition::new(label, "label", ValueType::String),
    );
    seed.ontology.node_types.push(declared);
    let mut relates = EdgeType::new(RELATES.parse().unwrap(), "relates");
    relates.source_types = [type_id].into_iter().collect();
    relates.target_types = [type_id].into_iter().collect();
    relates.cardinality = ekr_ontology::Cardinality::Many;
    seed.ontology.edge_types.push(relates);
    let bytes = evidence_bytes.to_vec();
    let hash = ContentHash::of_bytes(&bytes);
    let evidence = Evidence {
        id: "00000000-0000-4000-8000-000000000007".parse().unwrap(),
        source: EvidenceSource::HumanStatement {
            identity: Some("operator".into()),
        },
        content_hash: hash,
        extracted_by: context().operator,
        observed_at: Timestamp::EPOCH,
        confidence: Confidence::from_basis_points(10000).unwrap(),
    };
    seed.graph.evidence.insert(evidence.id, evidence);
    seed.evidence_payloads.insert(hash, bytes.into());
    seed
}
fn seed() -> SeedDocument {
    seed_with(b"synthetic human evidence")
}
fn encode(tx: &GraphTransaction) -> Vec<u8> {
    #[derive(Serialize)]
    struct Wire<'a> {
        format: &'static str,
        transaction: &'a GraphTransaction,
    }
    let format = if tx.operations.len() > 256 {
        "ekr.transaction-document/2"
    } else {
        "ekr.transaction-document/1"
    };
    serde_yaml_ng::to_string(&Wire {
        format,
        transaction: tx,
    })
    .unwrap()
    .into_bytes()
}
/// `nodes` new `Subject` nodes, each with one evidenced assertion: `2 * nodes` operations.
fn data(seed: &SeedDocument, tag: u64, nodes: u64) -> GraphTransaction {
    let ty = &seed.ontology.node_types[0];
    let label = *ty.properties.keys().next().unwrap();
    let evidence = *seed.graph.evidence.keys().next().unwrap();
    let root = seed.graph.root.id;
    let mut operations = Vec::new();
    for k in 0..nodes {
        let id = NodeId::mint();
        operations.push(GraphOperation::CreateNode(NodeDraft {
            id,
            root_id: root,
            type_id: ty.id,
            canonical_name: format!("subject {tag}.{k}"),
            properties: BTreeMap::new(),
            aliases: Vec::new(),
        }));
        operations.push(GraphOperation::AddAssertion(Box::new(Assertion {
            id: AssertionId::mint(),
            root_id: root,
            subject: Subject::Node(id),
            predicate: Predicate::Property(label),
            object: Object::Value(Value::String(format!("label {tag}.{k}"))),
            evidence: BTreeSet::from([evidence]),
            proposed_by: context().operator,
            assessment: Assessment::Proposed,
            lifecycle: AssertionLifecycle::Active,
            valid_time: TemporalRange::UNBOUNDED,
            transaction_time: TransactionTime::since(Timestamp::EPOCH),
        })));
    }
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations,
        evidence: BTreeSet::from([evidence]),
        schema_version: None,
    }
}
/// A node of the edge type: refused by the type check.
fn refused(seed: &SeedDocument) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::CreateNode(NodeDraft {
            id: NodeId::mint(),
            root_id: seed.graph.root.id,
            type_id: RELATES.parse().unwrap(),
            canonical_name: "not a subject".into(),
            properties: BTreeMap::new(),
            aliases: Vec::new(),
        })],
        evidence: BTreeSet::new(),
        schema_version: None,
    }
}
/// A schema-only transaction defining one new node type.
fn schema(tag: u64) -> GraphTransaction {
    GraphTransaction {
        id: TransactionId::mint(),
        proposer: context().operator,
        operations: vec![GraphOperation::DefineNodeType(Box::new(NodeType::new(
            TypeId::mint(),
            format!("Kind{tag}"),
        )))],
        evidence: BTreeSet::new(),
        schema_version: Some(SchemaVersionId::mint()),
    }
}

type Answers = (
    Option<Root>,
    CanonicalGraph,
    std::sync::Arc<BTreeMap<TransactionId, TransactionRecord>>,
);
fn answers(runtime: &Runtime) -> Answers {
    (
        runtime.head().unwrap(),
        runtime.snapshot().unwrap(),
        runtime.transactions().unwrap(),
    )
}
fn pointers(runtime: &Runtime) -> Vec<serde_json::Value> {
    runtime
        .published_events()
        .unwrap()
        .into_iter()
        .filter(|event| event.name == "ekr.store.CheckpointWritten")
        .map(|event| event.data)
        .collect()
}
fn provider_blob(path: &Path, file: bool, digest: &str) -> Option<Vec<u8>> {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let tenant = eventlog_core::TenantId::new(TENANT).unwrap();
    executor.block_on(async {
        use eventlog_core::EventStore;
        if file {
            let provider = eventlog_file::FileEventStore::open(path).await.unwrap();
            provider.get_blob(&tenant, digest).await.unwrap()
        } else {
            let provider = eventlog_sqlite::SqliteEventStore::open(
                &path.join("state.db").to_string_lossy(),
                "ekr",
            )
            .await
            .unwrap();
            provider.get_blob(&tenant, digest).await.unwrap()
        }
    })
}
fn delete_provider_blob(path: &Path, file: bool, digest: &str) {
    let executor = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let tenant = eventlog_core::TenantId::new(TENANT).unwrap();
    executor.block_on(async {
        use eventlog_core::EventStore;
        if file {
            let provider = eventlog_file::FileEventStore::open(path).await.unwrap();
            provider.delete_blob(&tenant, digest).await.unwrap();
        } else {
            let provider = eventlog_sqlite::SqliteEventStore::open(
                &path.join("state.db").to_string_lossy(),
                "ekr",
            )
            .await
            .unwrap();
            provider.delete_blob(&tenant, digest).await.unwrap();
        }
    });
}
fn checkpoint_key(pointer: &serde_json::Value) -> String {
    format!(
        "ekr.private.checkpoint.{}",
        pointer["checkpoint_hash"].as_str().unwrap()
    )
}
fn checkpoint_bytes(path: &Path, file: bool, pointer: &serde_json::Value) -> Vec<u8> {
    provider_blob(path, file, &checkpoint_key(pointer)).expect("the named checkpoint is retained")
}
fn checkpoint_revision(path: &Path, file: bool, pointer: &serde_json::Value) -> u64 {
    let checkpoint: serde_json::Value =
        serde_json::from_slice(&checkpoint_bytes(path, file, pointer)).unwrap();
    checkpoint["revision"].as_u64().unwrap()
}
/// Appends a pointer through the store's own writer, as a handle with no kernel authority.
fn install(path: &Path, file: bool, covered: u64, binding: ContentHash, bytes: Option<&[u8]>) {
    if file {
        ekr_store::FileStore::file_existing(path, TENANT, None)
            .unwrap()
            .write_checkpoint(covered, binding, bytes)
            .unwrap();
    } else {
        ekr_store::SqliteStore::sqlite_existing(&path.join("state.db"), TENANT, None)
            .unwrap()
            .write_checkpoint(covered, binding, bytes)
            .unwrap();
    }
}
fn binding_of(pointer: &serde_json::Value) -> ContentHash {
    serde_json::from_value(pointer["binding"].clone()).unwrap()
}

/// A small deterministic generator, so that every run makes the same interleaving.
struct Lcg(u64);
impl Lcg {
    fn next(&mut self, below: u64) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) % below
    }
}

#[derive(Clone, Copy, Debug)]
enum Who {
    OneShot,
    Full,
    A,
    B,
}

struct World<'a> {
    path: &'a Path,
    file: bool,
    profile: Profile,
    a: Runtime,
    b: Runtime,
    clock: Cell<i64>,
}
impl World<'_> {
    fn now(&self) -> Timestamp {
        let at = self.clock.get() + 10;
        self.clock.set(at);
        Timestamp::from_millis(at)
    }
    fn on<T>(&self, who: Who, f: impl FnOnce(&Runtime) -> T) -> T {
        match who {
            Who::OneShot => f(&open(self.path, self.file, self.profile)),
            Who::Full => f(&open_in_full(self.path, self.file, self.profile)),
            Who::A => f(&self.a),
            Who::B => f(&self.b),
        }
    }
    fn propose(&self, who: Who, tx: &GraphTransaction) {
        let at = self.now();
        self.on(who, |runtime| {
            runtime
                .propose(&encode(tx), context().operator, || at)
                .unwrap();
        });
    }
    fn validate(&self, who: Who, tx: &GraphTransaction, basis: u64) -> ValidationCommandResult {
        let at = self.now();
        self.on(who, |runtime| {
            runtime
                .validate(tx.id, RevisionNumber::new(basis), || at)
                .unwrap()
        })
    }
    fn commit(&self, who: Who, tx: &GraphTransaction) -> CommitCommandResult {
        let at = self.now();
        self.on(who, |runtime| {
            runtime.commit(tx.id, context().operator, || at).unwrap()
        })
    }
}

fn pick_who(rng: &mut Lcg) -> Who {
    match rng.next(4) {
        0 => Who::OneShot,
        1 => Who::Full,
        2 => Who::A,
        _ => Who::B,
    }
}

/// After every step of a mixed history, a fresh open, both session handles and a full replay
/// answer alike; after every commit the newest checkpoint is within § 99.2's bound of the head
/// (fewer than `REPLAY_CHECKPOINT_COMMITS` commits and fewer than
/// `REPLAY_CHECKPOINT_OPERATIONS` operations after it); and where no validation after the
/// checkpoint names a revision before it, a fresh open replays nothing from the seed.
fn mixed_history(file: bool, profile: Profile, seed_value: u64) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path();
    let seed = seed();
    open(path, file, profile)
        .seed(seed.clone(), || Timestamp::from_millis(10))
        .unwrap();
    let world = World {
        path,
        file,
        profile,
        a: open(path, file, profile),
        b: open(path, file, profile),
        clock: Cell::new(100),
    };
    // Both session handles read before anything else is written, as a session's first verb does.
    world.a.head().unwrap();
    world.b.head().unwrap();
    let mut rng = Lcg(seed_value);
    let mut head: u64 = 0;
    let mut commits = 0;
    let mut newest_checkpoint =
        pointers(&open(path, file, profile)).pop().unwrap()["checkpoint_hash"].clone();
    let mut checkpoint_at: u64 = 0;
    // Whether a validation after the newest checkpoint names a revision before its head.
    let mut early = false;
    for step in 0..36_u64 {
        let action = rng.next(100);
        let how = format!("file={file} {profile:?} seed={seed_value} step={step} action={action}");
        let mut committed_now = false;
        match action {
            // A small commit against the head, each verb by any handle.
            0..=37 => {
                let nodes = if action < 30 { 1 + rng.next(3) } else { 130 };
                let tx = data(&seed, step, nodes);
                world.propose(pick_who(&mut rng), &tx);
                let verdict = world.validate(pick_who(&mut rng), &tx, head);
                assert!(
                    matches!(verdict, ValidationCommandResult::Validated(_)),
                    "{how}: {verdict:?}"
                );
                let result = world.commit(pick_who(&mut rng), &tx);
                assert!(
                    matches!(result, CommitCommandResult::Committed(_)),
                    "{how}: {result:?}"
                );
                head += 1;
                committed_now = true;
            }
            // A rejection.
            38..=47 => {
                let tx = refused(&seed);
                world.propose(pick_who(&mut rng), &tx);
                let verdict = world.validate(pick_who(&mut rng), &tx, head);
                assert!(
                    matches!(verdict, ValidationCommandResult::Rejected(_)),
                    "{how}: {verdict:?}"
                );
            }
            // Stale by basis: validated against an earlier revision.
            48..=57 if head >= 1 => {
                let basis = rng.next(head);
                let tx = data(&seed, step, 1);
                world.propose(pick_who(&mut rng), &tx);
                let verdict = world.validate(pick_who(&mut rng), &tx, basis);
                assert!(
                    matches!(verdict, ValidationCommandResult::Validated(_)),
                    "{how}: {verdict:?}"
                );
                if basis < checkpoint_at {
                    early = true;
                }
                let result = world.commit(pick_who(&mut rng), &tx);
                assert!(
                    matches!(result, CommitCommandResult::Stale(_)),
                    "{how}: {result:?}"
                );
            }
            // Stale by race: validated against the head, which another commit then moves.
            58..=65 => {
                let late = data(&seed, step, 1);
                world.propose(pick_who(&mut rng), &late);
                let verdict = world.validate(pick_who(&mut rng), &late, head);
                assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
                let first = data(&seed, step + 1000, 2);
                world.propose(pick_who(&mut rng), &first);
                world.validate(pick_who(&mut rng), &first, head);
                let result = world.commit(pick_who(&mut rng), &first);
                assert!(
                    matches!(result, CommitCommandResult::Committed(_)),
                    "{how}: {result:?}"
                );
                head += 1;
                let result = world.commit(pick_who(&mut rng), &late);
                assert!(
                    matches!(result, CommitCommandResult::Stale(_)),
                    "{how}: {result:?}"
                );
                committed_now = true;
            }
            // A schema change, where the profile admits one.
            66..=75 if profile != Profile::V1 => {
                let tx = schema(step);
                world.propose(pick_who(&mut rng), &tx);
                let verdict = world.validate(pick_who(&mut rng), &tx, head);
                assert!(
                    matches!(verdict, ValidationCommandResult::Validated(_)),
                    "{how}: {verdict:?}"
                );
                let result = world.commit(pick_who(&mut rng), &tx);
                assert!(
                    matches!(result, CommitCommandResult::Committed(_)),
                    "{how}: {result:?}"
                );
                head += 1;
                committed_now = true;
            }
            // A proposal left pending.
            76..=87 => {
                world.propose(pick_who(&mut rng), &data(&seed, step, 1));
            }
            // A validation left uncommitted, against the head or the revision before it.
            _ => {
                let basis = if head >= 1 && rng.next(2) == 0 {
                    head - 1
                } else {
                    head
                };
                let tx = data(&seed, step, 1);
                world.propose(pick_who(&mut rng), &tx);
                let verdict = world.validate(pick_who(&mut rng), &tx, basis);
                assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
                if basis < checkpoint_at {
                    early = true;
                }
            }
        }
        if committed_now {
            commits += 1;
            let written = pointers(&open(path, file, profile));
            let newest = written.last().unwrap();
            if newest["checkpoint_hash"] != newest_checkpoint {
                newest_checkpoint = newest["checkpoint_hash"].clone();
                checkpoint_at = checkpoint_revision(path, file, newest);
                early = false;
            }
        }
        let truth = answers(&open_in_full(path, file, profile));
        assert_eq!(
            truth.0.unwrap().revision,
            RevisionNumber::new(head),
            "{how}"
        );
        let fresh = open(path, file, profile);
        assert_eq!(answers(&fresh), truth, "{how}: a fresh open");
        if !early {
            assert_eq!(
                fresh.seed_replays(),
                0,
                "{how}: head {head}, checkpoint {checkpoint_at}: a fresh open continues from the \
                 checkpoint"
            );
        }
        assert_eq!(answers(&world.a), truth, "{how}: session handle A");
        assert_eq!(answers(&world.b), truth, "{how}: session handle B");
        if committed_now {
            assert!(
                head - checkpoint_at < REPLAY_CHECKPOINT_COMMITS,
                "{how}: head {head} is {} commits past the newest checkpoint {checkpoint_at}",
                head - checkpoint_at
            );
            let operations: u64 = truth
                .2
                .values()
                .filter_map(|record| {
                    record
                        .committed
                        .as_ref()
                        .filter(|receipt| receipt.result.revision.get() > checkpoint_at)
                        .map(|_| record.proposal.operation_count)
                })
                .sum();
            assert!(
                operations < REPLAY_CHECKPOINT_OPERATIONS,
                "{how}: {operations} operations after the newest checkpoint {checkpoint_at}"
            );
        }
    }
    let written = pointers(&open(path, file, profile));
    let checkpoints = (1..written.len())
        .filter(|&i| written[i]["checkpoint_hash"] != written[i - 1]["checkpoint_hash"])
        .count();
    eprintln!(
        "file={file} {profile:?} seed={seed_value}: head {head}, {commits} commit steps, \
         {checkpoints} checkpoints after the seed's, {} pointers",
        written.len()
    );
    assert!(commits >= 8, "the interleaving commits enough: {commits}");
    assert!(
        checkpoints >= 2 && (checkpoints as u64) < head,
        "the interleaving writes checkpoints and pointer-only commits: {checkpoints} of {head}"
    );
}

#[test]
fn adversary_checkpoint_cadence_mixed_history_v1() {
    for file in [false, true] {
        for seed_value in [7, 1_001] {
            mixed_history(file, Profile::V1, seed_value);
        }
    }
}
#[test]
fn adversary_checkpoint_cadence_mixed_history_v2() {
    for file in [false, true] {
        for seed_value in [7, 1_001] {
            mixed_history(file, Profile::V2, seed_value);
        }
    }
}
#[test]
fn adversary_checkpoint_cadence_mixed_history_v3() {
    for file in [false, true] {
        for seed_value in [7, 1_001] {
            mixed_history(file, Profile::V3, seed_value);
        }
    }
}

/// Seeds and commits `commits` small transactions through one-shot handles.
fn build(path: &Path, file: bool, profile: Profile, seed: &SeedDocument, commits: u64) {
    open(path, file, profile)
        .seed(seed.clone(), || Timestamp::from_millis(10))
        .unwrap();
    for n in 1..=commits {
        commit_one(path, file, profile, seed, n);
    }
}
fn commit_one(path: &Path, file: bool, profile: Profile, seed: &SeedDocument, n: u64) {
    let tx = data(seed, n, 2);
    let at = i64::try_from(n * 100).unwrap();
    open(path, file, profile)
        .propose(&encode(&tx), context().operator, || {
            Timestamp::from_millis(at)
        })
        .unwrap();
    let verdict = open(path, file, profile)
        .validate(tx.id, RevisionNumber::new(n - 1), || {
            Timestamp::from_millis(at + 1)
        })
        .unwrap();
    assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
    let result = open(path, file, profile)
        .commit(tx.id, context().operator, || Timestamp::from_millis(at + 2))
        .unwrap();
    assert!(matches!(result, CommitCommandResult::Committed(_)));
}

/// The states a crash or an interrupted write can leave, built directly: after them a fresh open
/// answers as a full replay, and the next commit leaves a checkpoint within the bound.
#[test]
fn adversary_checkpoint_cadence_crash_states_reopen_as_a_full_replay() {
    let profile = Profile::V3;
    let n = REPLAY_CHECKPOINT_COMMITS;
    for file in [false, true] {
        for case in 0..4 {
            let directory = tempfile::tempdir().unwrap();
            let path = directory.path();
            let seed = seed();
            // Checkpoints at the seed and at revision n; pointer-only commits n + 1 and n + 2.
            build(path, file, profile, &seed, n + 2);
            let written = pointers(&open(path, file, profile));
            assert_eq!(written.len() as u64, n + 3, "file={file}");
            let at_n = written[n as usize].clone();
            assert_eq!(checkpoint_revision(path, file, &at_n), n, "file={file}");
            let older = checkpoint_bytes(path, file, &at_n);
            let at_n_plus_1 = written[n as usize + 1].clone();
            // Two more commits and the checkpoint of revision 2n replaces n's.
            for revision in n + 3..=2 * n {
                commit_one(path, file, profile, &seed, revision);
            }
            let newest = pointers(&open(path, file, profile)).pop().unwrap();
            assert_eq!(checkpoint_revision(path, file, &newest), 2 * n);
            let how = format!("file={file} case={case}");
            match case {
                // A pointer older than the head: the commit after revision n + 1 left none.
                0 => install(
                    path,
                    file,
                    at_n_plus_1["covered"].as_u64().unwrap(),
                    binding_of(&at_n_plus_1),
                    None,
                ),
                // The newest pointer names the older genuine checkpoint of revision n.
                1 => install(
                    path,
                    file,
                    at_n_plus_1["covered"].as_u64().unwrap(),
                    binding_of(&at_n_plus_1),
                    Some(&older),
                ),
                // The checkpoint the newest pointer names is gone.
                2 => delete_provider_blob(path, file, &checkpoint_key(&newest)),
                // The newest pointer covers every occurrence with this history's binding and
                // names another history's checkpoint at the same revision.
                _ => {
                    let other = tempfile::tempdir().unwrap();
                    let other_seed = seed_with(b"another history's evidence");
                    build(other.path(), file, profile, &other_seed, 2 * n);
                    let theirs = pointers(&open(other.path(), file, profile)).pop().unwrap();
                    let bytes = checkpoint_bytes(other.path(), file, &theirs);
                    install(
                        path,
                        file,
                        newest["covered"].as_u64().unwrap(),
                        binding_of(&newest),
                        Some(&bytes),
                    );
                }
            }
            let truth = answers(&open_in_full(path, file, profile));
            assert_eq!(
                truth.0.unwrap().revision,
                RevisionNumber::new(2 * n),
                "{how}"
            );
            assert_eq!(answers(&open(path, file, profile)), truth, "{how}");
            // The next commit, by a one-shot handle, leaves a readable checkpoint within the
            // bound, and a fresh open still answers as a full replay.
            commit_one(path, file, profile, &seed, 2 * n + 1);
            let newest = pointers(&open(path, file, profile)).pop().unwrap();
            let checkpoint_at = checkpoint_revision(path, file, &newest);
            assert!(
                2 * n + 1 - checkpoint_at < n,
                "{how}: head {} and newest checkpoint {checkpoint_at}",
                2 * n + 1
            );
            let fresh = open(path, file, profile);
            assert_eq!(
                answers(&fresh),
                answers(&open_in_full(path, file, profile)),
                "{how}"
            );
            assert_eq!(fresh.seed_replays(), 0, "{how}");
        }
    }
}

/// Design § 99.1, as corrected after this pass: "A handle opened with `--full-replay` admits
/// none, so its first commit writes one; from there it counts from the one it wrote, as any
/// handle does." `ekr session` under `EKR_FULL_REPLAY=1` is one such handle serving several
/// commits. (Pass 1 held the sentence it replaced — a checkpoint at each of its commits — which
/// the code never did; the coordinator chose to correct the sentence.)
#[test]
fn adversary_checkpoint_cadence_a_full_replay_handle_writes_a_checkpoint_at_its_first_commit() {
    let profile = Profile::V1;
    let bound = REPLAY_CHECKPOINT_COMMITS;
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = seed();
        build(path, file, profile, &seed, 0);
        let session = open_in_full(path, file, profile);
        for n in 1..=bound + 1 {
            let tx = data(&seed, n, 1);
            let at = i64::try_from(n * 100).unwrap();
            session
                .propose(&encode(&tx), context().operator, || {
                    Timestamp::from_millis(at)
                })
                .unwrap();
            session
                .validate(tx.id, RevisionNumber::new(n - 1), || {
                    Timestamp::from_millis(at + 1)
                })
                .unwrap();
            let result = session
                .commit(tx.id, context().operator, || Timestamp::from_millis(at + 2))
                .unwrap();
            assert!(matches!(result, CommitCommandResult::Committed(_)));
            let newest = pointers(&open(path, file, profile)).pop().unwrap();
            // Its first commit, then every `bound`th after it.
            let expected = 1 + (n - 1) / bound * bound;
            assert_eq!(
                checkpoint_revision(path, file, &newest),
                expected,
                "file={file}: after the full-replay handle's commit of revision {n}"
            );
        }
    }
}

/// A validation against a revision before the retained checkpoint's head is re-derived by every
/// open from the seed, because the checkpoint holds no graph of that revision (design § 99.2).
/// Before § 99 the next commit wrote a checkpoint covering it; now up to
/// `REPLAY_CHECKPOINT_COMMITS - 1` further commits leave every open replaying the whole lineage.
/// Asserted: once a commit follows it, an open replays nothing from the seed again.
#[test]
fn adversary_checkpoint_cadence_an_early_basis_validation_is_covered_by_the_next_commit() {
    let profile = Profile::V1;
    let n = REPLAY_CHECKPOINT_COMMITS;
    for file in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path();
        let seed = seed();
        // The checkpoint of revision n, and one pointer-only commit after it.
        build(path, file, profile, &seed, n + 1);
        let newest = pointers(&open(path, file, profile)).pop().unwrap();
        assert_eq!(checkpoint_revision(path, file, &newest), n, "file={file}");
        // An agent validates against revision 1, which it read earlier; its commit is stale.
        let tx = data(&seed, 900, 1);
        open(path, file, profile)
            .propose(&encode(&tx), context().operator, || {
                Timestamp::from_millis(90_000)
            })
            .unwrap();
        let verdict = open(path, file, profile)
            .validate(tx.id, RevisionNumber::new(1), || {
                Timestamp::from_millis(90_001)
            })
            .unwrap();
        assert!(matches!(verdict, ValidationCommandResult::Validated(_)));
        let stale = open(path, file, profile)
            .commit(tx.id, context().operator, || Timestamp::from_millis(90_002))
            .unwrap();
        assert!(matches!(stale, CommitCommandResult::Stale(_)), "{stale:?}");
        commit_one(path, file, profile, &seed, n + 2);
        let fresh = open(path, file, profile);
        assert_eq!(
            answers(&fresh),
            answers(&open_in_full(path, file, profile)),
            "file={file}"
        );
        assert_eq!(
            fresh.seed_replays(),
            0,
            "file={file}: after the commit of revision {}, the open still replays from the seed",
            n + 2
        );
    }
}
