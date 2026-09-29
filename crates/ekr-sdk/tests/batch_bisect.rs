//! `story:sdk-resolve-and-batch`: batches that respect the kernel's caps, retry a `Stale` commit
//! and bisect a rejection down to the group that caused it.
//!
//! Every case drives a real `ekr`, built once from this checkout (`ekr_path`), through one child
//! `ekr session`. A "second process" is the same binary run one-shot against the same store while
//! the session holds it.

#![cfg(unix)]

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;

use ekr_sdk::batch::{
    BatchError, BatchReport, Batcher, CallError, CommittedTransaction, Issue, RejectedOperation,
    Rejection,
};
use ekr_sdk::binary::EkrBinary;
use ekr_sdk::document::{
    AgentId, AliasAddition, GraphRootId, NodeDraft, NodeId, Operation, TransactionBuilder,
    TransactionId, TypeId,
};
use ekr_sdk::reply::{Answer, Reply};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{RecordingTransport, Request, Transport, TransportError};
use serde_json::Value as Json;

const BACKENDS: [Backend; 2] = [Backend::File, Backend::Sqlite];
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const UNDECLARED_TYPE: &str = "00000000-0000-4000-8000-000000000999";

/// The workspace root, found at run time from the directory holding `Cargo.lock`: never
/// `env!("CARGO_MANIFEST_DIR")` (`AGENTS.md` § The gate).
fn workspace_root() -> PathBuf {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    manifest
        .ancestors()
        .find(|directory| directory.join("Cargo.lock").is_file())
        .expect("a workspace root above the crate")
        .to_path_buf()
}

/// The `ekr` binary of this checkout, built once per test process through cargo.
fn ekr_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
        let output = std::process::Command::new(cargo)
            .arg("build")
            .arg("--manifest-path")
            .arg(workspace_root().join("Cargo.toml"))
            .args(["--locked", "-p", "ekr", "--bin", "ekr"])
            .arg("--message-format=json-render-diagnostics")
            .stderr(Stdio::inherit())
            .output()
            .expect("running cargo build -p ekr");
        assert!(output.status.success(), "cargo build -p ekr failed");
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str::<Json>(line).ok())
            .filter(|message| {
                message["reason"] == "compiler-artifact" && message["target"]["name"] == "ekr"
            })
            .find_map(|message| message["executable"].as_str().map(PathBuf::from))
            .expect("cargo named the ekr executable it built")
    })
}

fn id<T: std::str::FromStr>(text: &str) -> T
where
    T::Err: std::fmt::Debug,
{
    text.parse().unwrap()
}

fn root() -> GraphRootId {
    id(ROOT)
}

fn operator() -> AgentId {
    id(OPERATOR)
}

fn organization() -> TypeId {
    id(ORGANIZATION)
}

/// Exit 0 and stdout as text, from the built binary with no store.
fn ekr_text(args: &[&str]) -> String {
    let output = std::process::Command::new(ekr_path())
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// A directory holding the example host and seed, and a store path on one provider.
struct World {
    directory: tempfile::TempDir,
    backend: Backend,
}

impl World {
    fn new(backend: Backend) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        std::fs::write(
            world.path().join("host.json"),
            ekr_text(&["example", "ekr.cli-host/1"]),
        )
        .unwrap();
        std::fs::write(
            world.path().join("seed.yaml"),
            ekr_text(&["example", "ekr-seed/2"]),
        )
        .unwrap();
        world
    }

    fn path(&self) -> &Path {
        self.directory.path()
    }

    fn store(&self) -> StoreConfig {
        let store = match self.backend {
            Backend::File => self.path().join("store"),
            Backend::Sqlite => self.path().join("state.db"),
        };
        StoreConfig {
            host: self.path().join("host.json"),
            store,
            backend: self.backend,
        }
    }

    /// A session that has seeded the store.
    fn seeded(&self) -> ProcessSession {
        let binary = EkrBinary::open(ekr_path()).unwrap();
        let options = SessionOptions {
            current_dir: Some(self.path().to_path_buf()),
            ..SessionOptions::default()
        };
        let mut session = ProcessSession::start(&binary, self.store(), options).unwrap();
        let seeded = ok(&mut session, Request::new(["seed", "seed.yaml"]));
        assert_eq!(seeded["result"]["revision"], 0);
        session
    }

    /// `ekr <args>` as a second process over the same store: the exit-0 document.
    fn second_process(&self, args: &[&str], stdin: &str) -> Json {
        let store = self.store();
        let mut child = std::process::Command::new(ekr_path())
            .env_clear()
            .env("EKR_HOST", &store.host)
            .env("EKR_STORE", &store.store)
            .env("EKR_BACKEND", store.backend.as_str())
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// One `CreateNode`, committed by a second process.
    fn commit_elsewhere(&self, alias: &str) {
        let document = TransactionBuilder::new(operator())
            .push(organization_node(alias).into())
            .build()
            .unwrap();
        let transaction = document.transaction.id.to_string();
        self.second_process(&["propose", "-"], &document.to_yaml().unwrap());
        let validated = self.second_process(&["validate", &transaction], "");
        assert_eq!(validated["kind"], "Validated", "{validated}");
        let committed = self.second_process(&["commit", &transaction], "");
        assert_eq!(committed["kind"], "Committed", "{committed}");
    }
}

/// The document of an exit-0 answer.
fn ok(transport: &mut dyn Transport, request: Request) -> Json {
    let reply = transport.request(&request).unwrap();
    match reply.answer() {
        Answer::Outcome(outcome) => outcome.document().clone(),
        other => panic!("{:?}: {other:?}", request.argv),
    }
}

fn organization_node(alias: &str) -> NodeDraft {
    NodeDraft::new(root(), organization(), alias).with_alias(alias)
}

/// The ids `ekr transactions --state <state>` lists.
fn transactions(transport: &mut dyn Transport, state: &str) -> BTreeSet<String> {
    ok(transport, Request::new(["transactions", "--state", state]))
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["transaction_id"].as_str().unwrap().to_owned())
        .collect()
}

/// How many nodes of `type_id` the head holds.
fn count_of(transport: &mut dyn Transport, type_id: TypeId) -> usize {
    let snapshot = ok(transport, Request::new(["snapshot"]));
    snapshot["graph"]["graph"]["nodes"]
        .as_object()
        .unwrap()
        .values()
        .filter(|node| node["type_id"] == type_id.to_string())
        .count()
}

/// Every group index the report lists as committed, each once.
fn committed_groups(report: &BatchReport) -> Vec<usize> {
    let mut groups: Vec<usize> = report
        .committed
        .iter()
        .flat_map(|committed: &CommittedTransaction| committed.groups.iter().copied())
        .collect();
    groups.sort_unstable();
    groups
}

/// 2,000 operations in 1,800 groups: 1,600 single `CreateNode`s and 200 dependency groups whose
/// `AddAlias` comes before the `CreateNode` of the node it names, so a group split in two would
/// be rejected (`unresolved-node`). Group 777 is replaced by a node of an undeclared type.
fn fixture() -> (Vec<Vec<Operation>>, Operation) {
    let mut groups: Vec<Vec<Operation>> = (0..1600)
        .map(|n| vec![organization_node(&format!("n-{n}")).into()])
        .collect();
    for n in 1600..1800 {
        let node = organization_node(&format!("n-{n}"));
        groups.push(vec![
            AliasAddition::new(node.id, format!("n-{n}-b")).into(),
            node.into(),
        ]);
    }
    let planted: Operation = NodeDraft::new(root(), id(UNDECLARED_TYPE), "planted").into();
    groups[777] = vec![planted.clone()];
    (groups, planted)
}

/// Acceptance: one planted invalid operation among 2,000 is the only rejection reported, named as
/// that operation with its issues, its batch and its group; every other group is committed and
/// listed with its transaction id.
#[test]
fn one_planted_invalid_operation_among_2000_is_the_only_rejection() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut session = world.seeded();
        let (groups, planted) = fixture();
        assert_eq!(groups.iter().map(Vec::len).sum::<usize>(), 2000);

        let report = Batcher::new(operator())
            .commit(&mut session, &groups)
            .unwrap();

        let [RejectedOperation {
            batch,
            group,
            index,
            operation,
            rejection,
        }] = report.rejected.as_slice()
        else {
            panic!(
                "{backend:?}: exactly one rejected operation: {:?}",
                report.rejected
            )
        };
        assert_eq!((*batch, *group, *index), (0, 777, 0), "{backend:?}");
        assert_eq!(operation, &planted, "{backend:?}");
        let Rejection::Rejected {
            transaction: rejected_by,
            issues,
        } = rejection
        else {
            panic!("{backend:?}: {rejection:?}")
        };
        assert!(
            issues
                .iter()
                .any(|issue: &Issue| issue.code == "unknown-type"
                    && issue.validator == "Type"
                    && issue.message.contains(UNDECLARED_TYPE)),
            "{backend:?}: {issues:?}"
        );

        let mut every_other: Vec<usize> = (0..groups.len()).filter(|&g| g != 777).collect();
        every_other.sort_unstable();
        assert_eq!(committed_groups(&report), every_other, "{backend:?}");
        assert!(report
            .committed
            .iter()
            .all(|committed| committed.batch == 0));

        let committed = transactions(&mut session, "Committed");
        for transaction in &report.committed {
            assert!(
                committed.contains(&transaction.transaction.to_string()),
                "{backend:?}: {} is not committed",
                transaction.transaction
            );
        }
        let revisions: BTreeSet<u64> = report.committed.iter().map(|c| c.revision).collect();
        assert_eq!(revisions.len(), report.committed.len(), "{backend:?}");
        assert!(
            transactions(&mut session, "Rejected").contains(&rejected_by.to_string()),
            "{backend:?}"
        );
        assert_eq!(
            count_of(&mut session, organization()),
            1 + 1799,
            "{backend:?}"
        );
    }
}

/// Chunks respect the operation and byte caps, whether the caps are the kernel's or lower, and a
/// group is never split across two chunks.
#[test]
fn chunks_respect_the_operation_and_byte_caps() {
    let world = World::new(Backend::File);
    let mut session = world.seeded();
    let mut transport = RecordingTransport::record(&mut session);
    let groups: Vec<Vec<Operation>> = (0..400)
        .map(|n| {
            (0..(1 + n % 3))
                .map(|k| organization_node(&format!("c-{n}-{k}")).into())
                .collect()
        })
        .collect();
    let operations: usize = groups.iter().map(Vec::len).sum();
    let (cap_operations, cap_bytes) = (150, 24 * 1024);

    let report = Batcher::new(operator())
        .with_limits(cap_operations, cap_bytes)
        .commit(&mut transport, &groups)
        .unwrap();

    assert!(report.rejected.is_empty(), "{report:?}");
    assert_eq!(committed_groups(&report), (0..400).collect::<Vec<_>>());
    let batches: BTreeSet<usize> = report.committed.iter().map(|c| c.batch).collect();
    assert!(batches.len() > 1, "{batches:?}");
    assert_eq!(batches, (0..batches.len()).collect());
    for committed in &report.committed {
        let carried: usize = committed.groups.iter().map(|&g| groups[g].len()).sum();
        assert!(carried <= cap_operations, "{carried} operations");
    }
    let proposals: Vec<usize> = transport
        .recording()
        .exchanges
        .iter()
        .filter(|exchange| exchange.request.verb() == "propose")
        .map(|exchange| exchange.request.stdin.as_deref().unwrap_or_default().len())
        .collect();
    assert_eq!(
        proposals.len(),
        report.committed.len(),
        "nothing was bisected"
    );
    assert!(
        proposals.iter().all(|&bytes| bytes <= cap_bytes),
        "{proposals:?}"
    );
    assert_eq!(
        count_of(&mut transport, organization()),
        1 + operations,
        "every operation landed"
    );
}

/// A transport that lets a second process commit just before the first `commit` it forwards, so
/// that commit finds the head moved.
struct Interfering<'a, T> {
    inner: T,
    world: &'a World,
    first_commit: Option<TransactionId>,
}

impl<T: Transport> Transport for Interfering<'_, T> {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        if request.verb() == "commit" && self.first_commit.is_none() {
            self.first_commit = Some(id(&request.argv[1]));
            self.world.commit_elsewhere("interloper");
        }
        self.inner.request(request)
    }
}

/// A transport that fails the second `commit` it is asked, as a dead session would.
struct FailingSecondCommit<T> {
    inner: T,
    commits: usize,
}

impl<T: Transport> Transport for FailingSecondCommit<T> {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        if request.verb() == "commit" {
            self.commits += 1;
            if self.commits == 2 {
                return Err(TransportError::Replay {
                    verb: "commit".to_owned(),
                    detail: "the test fails the second commit".to_owned(),
                });
            }
        }
        self.inner.request(request)
    }
}

/// A batch that stops part-way returns what it committed before it stopped, and why it stopped.
#[test]
fn a_batch_that_stops_reports_what_it_committed() {
    let world = World::new(Backend::File);
    let mut session = world.seeded();
    let mut failing = FailingSecondCommit {
        inner: &mut session,
        commits: 0,
    };
    let groups: Vec<Vec<Operation>> = (0..3)
        .map(|n| vec![organization_node(&format!("s-{n}")).into()])
        .collect();

    let error: BatchError = Batcher::new(operator())
        .with_limits(1, 1 << 20)
        .commit(&mut failing, &groups)
        .unwrap_err();

    assert!(
        matches!(
            *error.cause,
            CallError::Transport(TransportError::Replay { .. })
        ),
        "{error}"
    );
    assert_eq!(error.report.committed.len(), 1, "{error:?}");
    assert_eq!(error.report.committed[0].groups, [0]);
    assert!(error.report.rejected.is_empty());
    assert!(transactions(&mut session, "Committed")
        .contains(&error.report.committed[0].transaction.to_string()));
}

/// Acceptance: a forced `Stale` is committed on retry under a new id.
#[test]
fn a_forced_stale_is_committed_on_retry_under_a_new_id() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut session = world.seeded();
        let mut interfering = Interfering {
            inner: &mut session,
            world: &world,
            first_commit: None,
        };
        let node = organization_node("patient");
        let created: NodeId = node.id;
        let groups = vec![vec![node.into()]];

        let report = Batcher::new(operator())
            .commit(&mut interfering, &groups)
            .unwrap();

        let first = interfering.first_commit.expect("a commit was sent");
        assert!(report.rejected.is_empty(), "{backend:?}: {report:?}");
        let [committed] = report.committed.as_slice() else {
            panic!("{backend:?}: {report:?}")
        };
        assert_eq!(committed.stale, [first], "{backend:?}");
        assert_ne!(committed.transaction, first, "{backend:?}: a new id");
        assert_eq!(committed.groups, [0]);
        assert!(transactions(&mut session, "Stale").contains(&first.to_string()));
        assert!(
            transactions(&mut session, "Committed").contains(&committed.transaction.to_string())
        );
        let snapshot = ok(&mut session, Request::new(["snapshot"]));
        assert!(
            snapshot["graph"]["graph"]["nodes"]
                .get(created.to_string())
                .is_some(),
            "{backend:?}: the node landed"
        );
    }
}
