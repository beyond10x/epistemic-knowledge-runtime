//! Adversary pass 1 on `story:sdk-resolve-and-batch`.
//!
//! Two kinds of case. The `scripted_*` cases drive the `Batcher` through a scripted transport that
//! answers `propose`, `validate` and `commit` the way `docs/cli.md` declares them, so a bound can
//! be counted exactly. The other cases drive a real `ekr`, built once from this checkout, through
//! one child `ekr session`, with a "second process" run one-shot against the same store.
//!
//! Cases marked `#[ignore = "defect: ..."]` fail on the implementation they were written against;
//! each names the defect.

#![cfg(unix)]

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;
use std::time::Duration;

use ekr_sdk::batch::{BatchReport, Batcher, CallError, Rejection};
use ekr_sdk::binary::EkrBinary;
use ekr_sdk::document::{
    AgentId, GraphRootId, NodeDraft, NodeId, Operation, TransactionBuilder, TransactionId, TypeId,
    TypedReference,
};
use ekr_sdk::reply::{Answer, Reply};
use ekr_sdk::resolve::{Resolution, Resolver};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{RecordingTransport, Request, Transport, TransportError};
use serde_json::{json, Value as Json};

const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const PERSON: &str = "00000000-0000-4000-8000-000000000201";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";

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

fn person() -> TypeId {
    id(PERSON)
}

// ---------------------------------------------------------------------------------------------
// Scripted transport
// ---------------------------------------------------------------------------------------------

/// Answers `propose` with a proposal record, `validate` with `Rejected` when the document last
/// proposed holds one of `bad` and `Validated` otherwise, and `commit` with `Committed` (or
/// `Stale`, while `stale` is set). Counts every verb.
#[derive(Default)]
struct Scripted {
    bad: Vec<String>,
    stale: bool,
    last_proposed: String,
    proposes: usize,
    validates: usize,
    commits: usize,
    revision: u64,
}

fn outcome(document: Json) -> Reply {
    Reply {
        exit: 0,
        document: Some(document),
        stderr: String::new(),
    }
}

impl Transport for Scripted {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        Ok(match request.verb() {
            "propose" => {
                self.proposes += 1;
                self.last_proposed = request.stdin.clone().unwrap_or_default();
                outcome(json!({"kind": "Proposal", "state": "Proposed"}))
            }
            "validate" => {
                self.validates += 1;
                if self
                    .bad
                    .iter()
                    .any(|marker| self.last_proposed.contains(marker))
                {
                    outcome(json!({
                        "kind": "Rejected",
                        "issues": [{
                            "validator": "Type",
                            "code": "unknown-type",
                            "message": "planted",
                        }],
                    }))
                } else {
                    outcome(json!({"kind": "Validated"}))
                }
            }
            "commit" => {
                self.commits += 1;
                if self.stale {
                    outcome(json!({"kind": "Stale"}))
                } else {
                    self.revision += 1;
                    outcome(json!({"kind": "Committed", "result": {"revision": self.revision}}))
                }
            }
            verb => panic!("the scripted transport does not answer {verb}"),
        })
    }
}

/// `n` single-`CreateNode` groups; the groups in `bad` carry a marker the transport rejects.
fn scripted_groups(n: usize, bad: &BTreeSet<usize>) -> (Vec<Vec<Operation>>, Vec<String>) {
    let (root, organization) = (root(), organization());
    let groups = (0..n)
        .map(|g| {
            let name = if bad.contains(&g) {
                format!("PLANTED<{g}>")
            } else {
                format!("fine<{g}>")
            };
            vec![NodeDraft::new(root, organization, name).into()]
        })
        .collect();
    let markers = bad.iter().map(|g| format!("PLANTED<{g}>")).collect();
    (groups, markers)
}

/// Every group the report lists as committed and as rejected, each list sorted.
fn partition(report: &BatchReport) -> (Vec<usize>, Vec<usize>) {
    let mut committed: Vec<usize> = report
        .committed
        .iter()
        .flat_map(|c| c.groups.iter().copied())
        .collect();
    committed.sort_unstable();
    let mut rejected: Vec<usize> = report.rejected.iter().map(|r| r.group).collect();
    rejected.sort_unstable();
    (committed, rejected)
}

fn ceil_log2(n: usize) -> usize {
    (usize::BITS - (n - 1).leading_zeros()) as usize
}

/// The `Stale` bound is the documented one: the first commit and eight retries, each under a new
/// id, each proposed and validated again, and then `StaleRetries` naming all nine ids. A change of
/// `STALE_RETRIES` or of the comparison against it is caught here and by no case of the unit.
#[test]
fn scripted_stale_is_retried_eight_times_then_stops_with_every_id() {
    let mut transport = Scripted {
        stale: true,
        ..Scripted::default()
    };
    let groups = vec![vec![
        NodeDraft::new(root(), organization(), "patient").into()
    ]];

    let error = Batcher::new(operator())
        .commit(&mut transport, &groups)
        .unwrap_err();

    let CallError::StaleRetries { transactions } = *error.cause else {
        panic!("{:?}", error.cause)
    };
    assert_eq!(transactions.len(), 9, "the first commit and eight retries");
    let distinct: BTreeSet<_> = transactions.iter().collect();
    assert_eq!(distinct.len(), 9, "each retry under a newly minted id");
    assert_eq!(
        (transport.proposes, transport.validates, transport.commits),
        (9, 9, 9)
    );
    assert!(error.report.committed.is_empty());
    assert!(error.report.rejected.is_empty());
}

/// Bisection terminates, commits every group but the bad ones exactly once, reports every bad one
/// exactly once, and costs at most `1 + 2·k·⌈log₂ n⌉` validations for `k` bad groups among `n`.
/// Several fixed patterns stand in for a property test.
#[test]
fn scripted_bisection_partitions_the_groups_and_is_logarithmic_in_one_bad_group() {
    let n = 2000;
    let patterns: Vec<BTreeSet<usize>> = vec![
        BTreeSet::new(),
        BTreeSet::from([0]),
        BTreeSet::from([n - 1]),
        BTreeSet::from([777]),
        BTreeSet::from([0, n - 1]),
        BTreeSet::from([999, 1000]),
        (0..n).step_by(401).collect(),
    ];
    for bad in patterns {
        let (groups, markers) = scripted_groups(n, &bad);
        let mut transport = Scripted {
            bad: markers,
            ..Scripted::default()
        };

        let report = Batcher::new(operator())
            .commit(&mut transport, &groups)
            .unwrap();

        let (committed, rejected) = partition(&report);
        let expected_committed: Vec<usize> = (0..n).filter(|g| !bad.contains(g)).collect();
        assert_eq!(
            committed, expected_committed,
            "{bad:?}: each good group once"
        );
        assert_eq!(
            rejected,
            bad.iter().copied().collect::<Vec<_>>(),
            "{bad:?}: each bad group once"
        );
        assert_eq!(transport.commits, report.committed.len(), "{bad:?}");
        let bound = 1 + 2 * bad.len() * ceil_log2(n);
        assert!(
            transport.validates <= bound,
            "{bad:?}: {} validations, bound {bound}",
            transport.validates
        );
    }
}

/// Every group refused: every one is reported, and the cost is the whole tree, `2n − 1`
/// validations, each a transaction `ekr` retains as `Rejected`. Pinned so a change is visible.
#[test]
fn scripted_every_group_bad_costs_two_n_minus_one_validations() {
    let n = 64;
    let bad: BTreeSet<usize> = (0..n).collect();
    let (groups, markers) = scripted_groups(n, &bad);
    let mut transport = Scripted {
        bad: markers,
        ..Scripted::default()
    };

    let report = Batcher::new(operator())
        .commit(&mut transport, &groups)
        .unwrap();

    let (committed, rejected) = partition(&report);
    assert!(committed.is_empty());
    assert_eq!(rejected, (0..n).collect::<Vec<_>>());
    assert_eq!(transport.validates, 2 * n - 1);
    assert_eq!(transport.commits, 0);
}

/// A single group past the operation cap, and one past the byte cap, each travel alone and are
/// rejected as `Document` without a request; their neighbours commit in their own batches.
#[test]
fn scripted_a_group_past_a_cap_is_rejected_alone_and_its_neighbours_commit() {
    let (root, organization) = (root(), organization());
    let small =
        |name: &str| -> Vec<Operation> { vec![NodeDraft::new(root, organization, name).into()] };
    let past_operations: Vec<Operation> = (0..10_001)
        .map(|n| NodeDraft::new(root, organization, format!("o-{n}")).into())
        .collect();
    let long = "x".repeat(60_000);
    let past_bytes: Vec<Operation> = (0..150)
        .map(|_| NodeDraft::new(root, organization, long.as_str()).into())
        .collect();
    let groups = vec![
        small("before"),
        past_operations,
        small("between"),
        past_bytes,
        small("after"),
    ];
    let mut transport = Scripted::default();

    let report = Batcher::new(operator())
        .commit(&mut transport, &groups)
        .unwrap();

    let (committed, _) = partition(&report);
    assert_eq!(committed, [0, 2, 4]);
    let rejected_groups: BTreeMap<usize, usize> =
        report
            .rejected
            .iter()
            .fold(BTreeMap::new(), |mut counts, r| {
                *counts.entry(r.group).or_default() += 1;
                counts
            });
    assert_eq!(rejected_groups, BTreeMap::from([(1, 10_001), (3, 150)]));
    assert!(report
        .rejected
        .iter()
        .all(|r| matches!(r.rejection, Rejection::Document(_))));
    let batches: Vec<(usize, usize)> = report
        .rejected
        .iter()
        .map(|r| (r.group, r.batch))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(batches, [(1, 1), (3, 3)], "each alone in its own batch");
    assert_eq!(
        transport.proposes, 3,
        "nothing is sent for a group past a cap"
    );
}

// ---------------------------------------------------------------------------------------------
// Real `ekr`
// ---------------------------------------------------------------------------------------------

/// The workspace root, found at run time from the directory holding `Cargo.lock`.
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

fn ekr_text(args: &[&str]) -> String {
    let output = std::process::Command::new(ekr_path())
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

struct World {
    directory: tempfile::TempDir,
}

impl World {
    fn new() -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
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
        StoreConfig {
            host: self.path().join("host.json"),
            store: self.path().join("store"),
            backend: Backend::File,
        }
    }

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

    /// A node of `type_id` known by `alias`, committed by a second process: its id.
    fn committed_elsewhere(&self, type_id: TypeId, alias: &str) -> NodeId {
        let node = NodeDraft::new(root(), type_id, alias).with_alias(alias);
        let created = node.id;
        let document = TransactionBuilder::new(operator())
            .push(node.into())
            .build()
            .unwrap();
        let transaction = document.transaction.id.to_string();
        self.second_process(&["propose", "-"], &document.to_yaml().unwrap());
        let validated = self.second_process(&["validate", &transaction], "");
        assert_eq!(validated["kind"], "Validated", "{validated}");
        let committed = self.second_process(&["commit", &transaction], "");
        assert_eq!(committed["kind"], "Committed", "{committed}");
        created
    }
}

fn ok(transport: &mut dyn Transport, request: Request) -> Json {
    let reply = transport.request(&request).unwrap();
    match reply.answer() {
        Answer::Outcome(outcome) => outcome.document().clone(),
        other => panic!("{:?}: {other:?}", request.argv),
    }
}

fn transactions(transport: &mut dyn Transport, state: &str) -> BTreeSet<String> {
    ok(transport, Request::new(["transactions", "--state", state]))
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["transaction_id"].as_str().unwrap().to_owned())
        .collect()
}

/// Every node of `type_id` at the head, by id, with its aliases.
fn nodes_of(transport: &mut dyn Transport, type_id: TypeId) -> BTreeMap<String, Vec<String>> {
    let snapshot = ok(transport, Request::new(["snapshot"]));
    snapshot["graph"]["graph"]["nodes"]
        .as_object()
        .unwrap()
        .values()
        .filter(|node| node["type_id"] == type_id.to_string())
        .map(|node| {
            let aliases = node["aliases"]
                .as_array()
                .unwrap()
                .iter()
                .map(|alias| alias.as_str().unwrap().to_owned())
                .collect();
            (node["id"].as_str().unwrap().to_owned(), aliases)
        })
        .collect()
}

/// Forwards every request; the `lose`-th `commit` (counted from 1) is forwarded and applied, and
/// its reply is then lost, as a `TimedOut` or a cancel during a slow commit loses it.
struct LosesCommitReply<T> {
    inner: T,
    lose: usize,
    commits: usize,
}

impl<T: Transport> Transport for LosesCommitReply<T> {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        let reply = self.inner.request(request)?;
        if request.verb() == "commit" {
            self.commits += 1;
            if self.commits == self.lose {
                return Err(TransportError::TimedOut {
                    verb: "commit".to_owned(),
                    timeout: Duration::from_secs(300),
                    stderr_tail: String::new(),
                });
            }
        }
        Ok(reply)
    }
}

/// `docs/sdk.md` § Batches: "`commit` returns a `BatchError`. Its `report` lists what was
/// committed and rejected until then." A commit whose reply is lost after `ekr` applied it is
/// committed, and neither the report nor the error names it, so a consumer that retries what the
/// report does not list commits it twice.
#[test]
fn a_commit_whose_reply_is_lost_is_named_by_the_batch_error() {
    let world = World::new();
    let mut session = world.seeded();
    let before = transactions(&mut session, "Committed");
    let groups: Vec<Vec<Operation>> = (0..3)
        .map(|n| {
            let alias = format!("lost-{n}");
            vec![NodeDraft::new(root(), organization(), &alias)
                .with_alias(alias)
                .into()]
        })
        .collect();
    let mut losing = LosesCommitReply {
        inner: &mut session,
        lose: 2,
        commits: 0,
    };

    let error = Batcher::new(operator())
        .with_limits(1, 1 << 20)
        .commit(&mut losing, &groups)
        .unwrap_err();

    assert!(matches!(
        *error.cause,
        CallError::Transport(TransportError::TimedOut { .. })
    ));
    let listed: BTreeSet<String> = error
        .report
        .committed
        .iter()
        .map(|c| c.transaction.to_string())
        .collect();
    let told = format!("{error:?}");
    let landed: Vec<String> = transactions(&mut session, "Committed")
        .difference(&before)
        .cloned()
        .collect();
    assert_eq!(landed.len(), 2, "two commits reached the store");
    for transaction in &landed {
        // `TransactionId`'s `Debug` is its integer, its `Display` the UUID: accept either.
        let debug = format!("{:?}", id::<TransactionId>(transaction));
        assert!(
            listed.contains(transaction)
                || told.contains(transaction.as_str())
                || told.contains(&debug),
            "{transaction} is committed and the BatchError does not name it: {error:?}"
        );
    }
}

/// `docs/sdk.md` § Resolve before you create: "The queued id is never created, and
/// `Flushed::replaced` maps it to the new resolution." After a flush whose commit reply was lost,
/// the next flush finds the queued node itself at the head and reports it as replaced by itself.
#[test]
fn a_queued_node_committed_under_a_lost_reply_is_not_replaced_by_itself() {
    let world = World::new();
    let mut session = world.seeded();
    let mut resolver = Resolver::new(root(), operator());
    let alpha = TypedReference::new(organization(), ["alpha"]);
    let Resolution::Queued(queued) = resolver.resolve(&mut session, &alpha).unwrap() else {
        panic!("alpha is new")
    };
    let mut losing = LosesCommitReply {
        inner: &mut session,
        lose: 1,
        commits: 0,
    };
    resolver.flush(&mut losing).unwrap_err();

    let flushed = resolver.flush(&mut session).unwrap();

    let organizations = nodes_of(&mut session, organization());
    assert!(
        organizations.contains_key(&queued.to_string()),
        "the queued id was created"
    );
    assert_eq!(
        organizations
            .values()
            .filter(|aliases| aliases.contains(&"alpha".to_owned()))
            .count(),
        1,
        "no duplicate"
    );
    assert_ne!(
        flushed.replaced.get(&queued),
        Some(&Resolution::Resolved(queued)),
        "{flushed:?}"
    );
}

/// A foreign commit that names none of the queued aliases drops the cache, and each queued
/// reference is resolved again, stays queued and is committed by the same flush. No case of the
/// unit reaches `reconcile`'s `ProposeNew` arm: both of its foreign commits take the queued alias.
#[test]
fn an_unrelated_foreign_commit_keeps_queued_nodes_queued_and_commits_them() {
    let world = World::new();
    let mut session = world.seeded();
    let mut transport = RecordingTransport::record(&mut session);
    let mut resolver = Resolver::new(root(), operator());
    let alpha = TypedReference::new(organization(), ["alpha"]);
    let beta = TypedReference::new(person(), ["beta"]);
    let Resolution::Queued(x) = resolver.resolve(&mut transport, &alpha).unwrap() else {
        panic!("alpha is new")
    };
    let Resolution::Queued(y) = resolver.resolve(&mut transport, &beta).unwrap() else {
        panic!("beta is new")
    };
    world.committed_elsewhere(organization(), "zeta");

    let flushed = resolver.flush(&mut transport).unwrap();

    assert!(flushed.replaced.is_empty(), "{flushed:?}");
    assert!(flushed.report.rejected.is_empty(), "{flushed:?}");
    let committed: Vec<usize> = flushed
        .report
        .committed
        .iter()
        .flat_map(|c| c.groups.iter().copied())
        .collect();
    assert_eq!(committed, [0, 1], "{flushed:?}");
    assert!(nodes_of(&mut transport, organization()).contains_key(&x.to_string()));
    assert!(nodes_of(&mut transport, person()).contains_key(&y.to_string()));
    assert_eq!(
        resolver.resolve(&mut transport, &alpha).unwrap(),
        Resolution::Resolved(x),
        "cached again after the reconcile"
    );
    let resolves = transport
        .recording()
        .exchanges
        .iter()
        .filter(|e| e.request.verb() == "resolve")
        .count();
    assert_eq!(
        resolves, 4,
        "two first asks, two reconciles, the last resolve cached"
    );
}

/// One alias on two types is two keys and two nodes: resolving the second does not flush the
/// first, and one flush commits both.
#[test]
fn one_alias_on_two_types_queues_two_nodes_without_a_flush() {
    let world = World::new();
    let mut session = world.seeded();
    let mut transport = RecordingTransport::record(&mut session);
    let mut resolver = Resolver::new(root(), operator());
    let as_organization = TypedReference::new(organization(), ["mercury"]);
    let as_person = TypedReference::new(person(), ["mercury"]);
    let Resolution::Queued(o) = resolver.resolve(&mut transport, &as_organization).unwrap() else {
        panic!("new")
    };
    let Resolution::Queued(p) = resolver.resolve(&mut transport, &as_person).unwrap() else {
        panic!("new")
    };
    let verbs: Vec<&str> = transport
        .recording()
        .exchanges
        .iter()
        .map(|e| e.request.verb())
        .collect();
    assert_eq!(verbs, ["head", "resolve", "resolve"], "no flush between");

    let flushed = resolver.flush(&mut transport).unwrap();

    assert!(flushed.report.rejected.is_empty(), "{flushed:?}");
    assert!(nodes_of(&mut transport, organization()).contains_key(&o.to_string()));
    assert!(nodes_of(&mut transport, person()).contains_key(&p.to_string()));
}

/// Lets a second process give `alias` to a node of `type_id` just before the first `validate`,
/// and fails the first `commit` without forwarding it.
struct RaceThenFail<'a, T> {
    inner: T,
    world: &'a World,
    alias: &'static str,
    elsewhere: Option<NodeId>,
    commits: usize,
}

impl<T: Transport> Transport for RaceThenFail<'_, T> {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        if request.verb() == "validate" && self.elsewhere.is_none() {
            self.elsewhere = Some(self.world.committed_elsewhere(organization(), self.alias));
        }
        if request.verb() == "commit" {
            self.commits += 1;
            if self.commits == 1 {
                return Err(TransportError::Cancelled {
                    verb: "commit".to_owned(),
                });
            }
        }
        self.inner.request(request)
    }
}

/// `docs/sdk.md`: a commit rejected with `alias-already-exists` because a node arrived between the
/// head check and the commit is handled as a foreign node is, and `Flushed::replaced` maps the
/// queued id to it; `flush` keeps a node that was not committed queued. When a later request of
/// the same flush fails, the refused node is neither replaced nor queued: it is dropped.
#[test]
fn a_flush_that_fails_after_an_alias_race_still_replaces_the_refused_node() {
    let world = World::new();
    let mut session = world.seeded();
    let mut resolver = Resolver::new(root(), operator());
    let Resolution::Queued(alpha) = resolver
        .resolve(
            &mut session,
            &TypedReference::new(organization(), ["alpha"]),
        )
        .unwrap()
    else {
        panic!("alpha is new")
    };
    let Resolution::Queued(beta) = resolver
        .resolve(&mut session, &TypedReference::new(organization(), ["beta"]))
        .unwrap()
    else {
        panic!("beta is new")
    };
    let mut racing = RaceThenFail {
        inner: &mut session,
        world: &world,
        alias: "alpha",
        elsewhere: None,
        commits: 0,
    };
    resolver.flush(&mut racing).unwrap_err();
    let elsewhere = racing.elsewhere.expect("the race ran");

    let flushed = resolver.flush(&mut session).unwrap();

    assert!(
        flushed.report.rejected.iter().any(|r| matches!(
            &r.rejection,
            Rejection::Rejected { issues, .. } if issues.iter().any(|i| i.code == "alias-already-exists")
        )),
        "the race was reported: {flushed:?}"
    );
    assert!(
        nodes_of(&mut session, organization()).contains_key(&beta.to_string()),
        "beta stayed queued and was committed"
    );
    assert_eq!(
        flushed.replaced.get(&alpha),
        Some(&Resolution::Resolved(elsewhere)),
        "{flushed:?}"
    );
}
