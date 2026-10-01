//! Adversary pass 1 on `story:sdk-evidence-attachment`.
//!
//! Two kinds of case. The `scripted_*` cases drive the `Batcher` through a scripted transport that
//! answers `propose`, `validate` and `commit` the way `docs/cli.md` declares them and records every
//! document proposed, so what each transaction carried can be counted exactly. The other cases
//! drive a real `ekr`, built once from this checkout (`ekr_path`), through one child
//! `ekr session`, with a second process run one-shot against the same store where a case needs
//! the head to move.
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
    AgentId, Assertion, AssertionId, Confidence, EvidenceId, GraphRootId, NodeDraft, NodeId,
    Object, Operation, Predicate, PropertyId, Subject, Timestamp, TransactionBuilder, TypeId,
    Value,
};
use ekr_sdk::evidence::{EvidenceItem, EvidenceSet};
use ekr_sdk::reply::{Answer, Reply};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{Request, Transport, TransportError};
use serde_json::{json, Value as Json};

const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const LEGAL_NAME: &str = "00000000-0000-4000-8000-000000000801";
const UNDECLARED_TYPE: &str = "00000000-0000-4000-8000-000000000999";

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

// ---------------------------------------------------------------------------------------------
// Scripted transport
// ---------------------------------------------------------------------------------------------

/// Answers `propose` with a proposal record, `validate` with `Rejected` when the document last
/// proposed holds one of `bad` and `Validated` otherwise, and `commit` with `Committed`, or
/// `ekr.kernel.ProposalAttribution` naming another submitter at `propose` when `foreign` is set.
/// Records every document proposed.
#[derive(Default)]
struct Scripted {
    bad: Vec<String>,
    foreign: bool,
    proposed: Vec<String>,
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
                if self.foreign {
                    return Ok(Reply {
                        exit: 2,
                        document: None,
                        stderr: format!(
                            "ekr: ekr.kernel.ProposalAttribution: proposer does not match \
                             registered submitter {}\n",
                            AgentId::mint()
                        ),
                    });
                }
                self.proposed
                    .push(request.stdin.clone().unwrap_or_default());
                outcome(json!({"kind": "Proposal", "state": "Proposed"}))
            }
            "validate" => {
                let last = self.proposed.last().cloned().unwrap_or_default();
                if self.bad.iter().any(|marker| last.contains(marker)) {
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
                self.revision += 1;
                outcome(json!({"kind": "Committed", "result": {"revision": self.revision}}))
            }
            verb => panic!("the scripted transport does not answer {verb}"),
        })
    }
}

/// The operations of a proposed document, by counting the operation tags at the start of a list
/// item: `- !AddEvidence`, `- !AddAssertion`, `- !CreateNode`.
fn operation_count(yaml: &str) -> usize {
    yaml.lines()
        .map(str::trim_start)
        .filter(|line| {
            ["- !AddEvidence", "- !AddAssertion", "- !CreateNode"]
                .iter()
                .any(|tag| line.starts_with(tag))
        })
        .count()
}

/// How many `!AddEvidence` a proposed document carries.
fn evidence_additions(yaml: &str) -> usize {
    yaml.lines()
        .filter(|line| line.trim_start().starts_with("- !AddEvidence"))
        .count()
}

/// One assertion-only group citing `evidence`: no node, which the scripted transport does not
/// check. Its assertion id is the marker the scripted transport rejects it by.
fn claim(evidence: EvidenceId) -> (Vec<Operation>, String) {
    let claim = Assertion::new(
        root(),
        Subject::Node(NodeId::mint()),
        Predicate::Property(PropertyId::mint()),
        Object::Value(Value::String("scripted".to_owned())),
        operator(),
    )
    .citing(evidence);
    let marker = claim.id.to_string();
    (vec![claim.into()], marker)
}

fn scripted_item(n: u8) -> EvidenceItem {
    EvidenceItem::new(
        "a reader",
        Timestamp::from_millis(1_790_000_000_000),
        vec![n; 16],
    )
}

/// `Batcher::with_limits`: "This batcher packing at most `operations` operations … into one
/// batch." The plan counts an entry's `!AddEvidence` in the group that first cites it; when that
/// group is rejected, the next group citing the entry carries it into a batch the plan filled
/// without it. `submit` checks the byte limit of what it built (`within`) and not the operation
/// limit, so the transaction is proposed one operation past the cap.
///
/// Group 0 (rejected) cites item E and is packed alone: 2 operations. Group 1 cites E, which the
/// plan counts as introduced, and group 2 cites an id the set did not mint: batch 1 is planned at
/// 2 operations and proposed with 3.
#[test]
fn scripted_an_entry_moved_by_a_rejection_keeps_every_proposal_within_the_operation_cap() {
    let mut evidence = EvidenceSet::new(operator());
    let shared = evidence.cite(scripted_item(0));
    let (first, bad) = claim(shared);
    let (second, _) = claim(shared);
    let (third, _) = claim(EvidenceId::mint());
    let groups = vec![first, second, third];
    let mut transport = Scripted {
        bad: vec![bad],
        ..Scripted::default()
    };
    let cap = 2;

    let report = Batcher::new(operator())
        .with_limits(cap, 1 << 20)
        .commit_with_evidence(&mut transport, &groups, &mut evidence)
        .unwrap();

    let rejected: BTreeSet<usize> = report.rejected.iter().map(|r| r.group).collect();
    assert_eq!(rejected, BTreeSet::from([0]), "{report:?}");
    assert!(evidence.is_committed(shared));
    let counts: Vec<usize> = transport
        .proposed
        .iter()
        .map(|yaml| operation_count(yaml))
        .collect();
    assert!(
        counts.iter().all(|&count| count <= cap),
        "operations per proposed transaction: {counts:?}, cap {cap}"
    );
}

/// `EvidenceSet::mark_committed` (`docs/sdk.md` § Evidence items: "mark the entries its groups
/// cite … so no later run adds them again") is called by no case of the unit's suite: a
/// `mark_committed` that did nothing passes it. An entry marked committed is not added again.
#[test]
fn scripted_an_entry_marked_committed_is_not_added_again() {
    let mut evidence = EvidenceSet::new(operator());
    let marked = evidence.cite(scripted_item(1));
    let fresh = evidence.cite(scripted_item(2));
    assert!(!evidence.is_committed(marked));
    evidence.mark_committed(marked);
    assert!(evidence.is_committed(marked));
    let groups = vec![claim(marked).0, claim(fresh).0];
    let mut transport = Scripted::default();

    Batcher::new(operator())
        .commit_with_evidence(&mut transport, &groups, &mut evidence)
        .unwrap();

    assert_eq!(transport.proposed.len(), 1);
    let yaml = &transport.proposed[0];
    assert_eq!(evidence_additions(yaml), 1, "{yaml}");
    assert!(yaml.contains(&fresh.to_string()));
    let added = yaml
        .split("- !AddEvidence")
        .skip(1)
        .any(|entry| entry.contains(&format!("id: {marked}")));
    assert!(!added, "the marked entry was added again: {yaml}");
}

/// A refusal of the transaction as a whole stops the run and commits nothing, so no entry is
/// marked committed.
#[test]
fn scripted_a_batch_wide_refusal_marks_no_entry_committed() {
    let mut evidence = EvidenceSet::new(operator());
    let cited = evidence.cite(scripted_item(3));
    let groups = vec![claim(cited).0, claim(cited).0];
    let mut transport = Scripted {
        foreign: true,
        ..Scripted::default()
    };

    let report = Batcher::new(operator())
        .commit_with_evidence(&mut transport, &groups, &mut evidence)
        .unwrap();

    assert!(report.committed.is_empty());
    assert_eq!(report.refused.as_ref().unwrap().groups, [0, 1]);
    assert!(!evidence.is_committed(cited));
}

/// `docs/sdk.md` § Evidence items: "An item with the same source, observed-at time and bytes gets
/// the same id". Every item of the unit's suite differs in its observed-at time, so a key that
/// dropped the source, or the time, passes it. The same bytes from another source, or observed at
/// another time, are another entry with the same hash.
#[test]
fn the_same_bytes_from_another_source_or_time_are_another_entry() {
    let bytes = b"The letterhead reads: Acme Ltd.\n".to_vec();
    let at = Timestamp::from_millis(1_790_000_000_000);
    let later = Timestamp::from_millis(1_790_000_000_001);
    let mut evidence = EvidenceSet::new(operator());

    let first = evidence.cite(EvidenceItem::new("reader-a", at, bytes.clone()));
    let other_source = evidence.cite(EvidenceItem::new("reader-b", at, bytes.clone()));
    let other_time = evidence.cite(EvidenceItem::new("reader-a", later, bytes.clone()));
    let again = evidence.cite(EvidenceItem::new("reader-a", at, bytes.clone()));

    assert_eq!(again, first);
    let distinct: BTreeSet<EvidenceId> = [first, other_source, other_time].into();
    assert_eq!(distinct.len(), 3);
    assert_eq!(evidence.len(), 3);
    let hashes: BTreeSet<String> = [first, other_source, other_time]
        .iter()
        .map(|cited| {
            evidence
                .entry(*cited)
                .unwrap()
                .evidence
                .content_hash
                .to_string()
        })
        .collect();
    assert_eq!(hashes.len(), 1, "one hash for the same bytes");
    assert_eq!(
        evidence.entry(other_time).unwrap().evidence.observed_at,
        later
    );
    // `docs/sdk.md` § Evidence items: "confidence `Confidence::CERTAIN`", which no case of the
    // unit's suite reads.
    assert_eq!(
        evidence.entry(first).unwrap().evidence.confidence,
        Confidence::CERTAIN
    );
}

// ---------------------------------------------------------------------------------------------
// A real `ekr`
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

/// `ekr <args>` with `stdin`, from the built binary with no store: exit 0 and stdout as JSON.
fn ekr_json(args: &[&str], stdin: &[u8]) -> Json {
    let mut child = std::process::Command::new(ekr_path())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(stdin).unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn ekr_text(args: &[&str]) -> String {
    let output = std::process::Command::new(ekr_path())
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// A directory holding the example host and seed, and a file store.
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

    /// A node committed by a second process, which moves the head.
    fn commit_elsewhere(&self, alias: &str) {
        let node = NodeDraft::new(root(), id(ORGANIZATION), alias).with_alias(alias);
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

/// The evidence entries the head holds, by id.
fn evidence_entries(transport: &mut dyn Transport) -> BTreeMap<String, Json> {
    let snapshot = ok(transport, Request::new(["snapshot"]));
    snapshot["graph"]["graph"]["evidence"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(id, entry)| (id.clone(), entry.clone()))
        .collect()
}

/// The assertions the head holds, by id.
fn assertions(transport: &mut dyn Transport) -> BTreeMap<String, Json> {
    let snapshot = ok(transport, Request::new(["snapshot"]));
    snapshot["graph"]["graph"]["assertions"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(id, entry)| (id.clone(), entry.clone()))
        .collect()
}

/// Standard base64 with padding, as `ekr explain --documents` prints an evidence link's
/// `payload`.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let mut triple = [0_u8; 3];
        triple[..chunk.len()].copy_from_slice(chunk);
        let n = (u32::from(triple[0]) << 16) | (u32::from(triple[1]) << 8) | u32::from(triple[2]);
        for position in 0..4 {
            if position <= chunk.len() {
                out.push(char::from(
                    ALPHABET[((n >> (18 - 6 * position)) & 0x3f) as usize],
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The `payload` `ekr explain --documents` prints for `assertion`'s evidence link `evidence`.
fn explained_payload(
    transport: &mut dyn Transport,
    assertion: AssertionId,
    evidence: EvidenceId,
) -> Option<String> {
    let explained = ok(
        transport,
        Request::new([
            "explain".to_owned(),
            assertion.to_string(),
            "--documents".to_owned(),
        ]),
    );
    explained["links"]
        .as_array()
        .unwrap()
        .iter()
        .find(|link| link["kind"] == "Evidence" && link["id"] == evidence.to_string())
        .map(|link| link["payload"].as_str().unwrap().to_owned())
}

/// One group: a new organisation of `type_id` and one assertion of its legal name, citing
/// `evidence`.
fn group(name: &str, type_id: TypeId, evidence: EvidenceId) -> (Vec<Operation>, AssertionId) {
    let node = NodeDraft::new(root(), type_id, name).with_alias(name);
    let claim = Assertion::new(
        root(),
        Subject::Node(node.id),
        Predicate::Property(id::<PropertyId>(LEGAL_NAME)),
        Object::Value(Value::String(name.to_owned())),
        operator(),
    )
    .citing(evidence);
    let assertion = claim.id;
    (vec![node.into(), claim.into()], assertion)
}

fn item(text: &str) -> EvidenceItem {
    EvidenceItem::new(
        "a reader",
        Timestamp::from_millis(1_790_000_000_000),
        format!("{text}\n").into_bytes(),
    )
}

fn committed_groups(report: &BatchReport) -> BTreeSet<usize> {
    report
        .committed
        .iter()
        .flat_map(|committed| committed.groups.iter().copied())
        .collect()
}

/// Nine groups cite one item, packed three to a batch (each group is two operations, the first
/// citer three). The first, the middle, the last, all three, and every citer are rejected in turn.
/// Each run commits exactly one entry when any citer is committed and none when none is, every
/// committed assertion's evidence is held, and `explain` returns the item's bytes.
#[test]
fn rejected_citers_first_middle_last_and_all_never_strand_an_assertion() {
    let plans: [&[usize]; 5] = [&[0], &[4], &[8], &[0, 4, 8], &[0, 1, 2, 3, 4, 5, 6, 7, 8]];
    for rejected_plan in plans {
        let world = World::new();
        let mut session = world.seeded();
        let seeded = evidence_entries(&mut session);
        let mut evidence = EvidenceSet::new(operator());
        let shared = evidence.cite(item("the shared letter"));
        let mut groups = Vec::new();
        let mut claims = Vec::new();
        for n in 0..9 {
            let type_id = if rejected_plan.contains(&n) {
                id(UNDECLARED_TYPE)
            } else {
                id(ORGANIZATION)
            };
            let (operations, assertion) = group(&format!("citer-{n}"), type_id, shared);
            groups.push(operations);
            claims.push(assertion);
        }

        let report = Batcher::new(operator())
            .with_limits(7, 1 << 20)
            .commit_with_evidence(&mut session, &groups, &mut evidence)
            .unwrap();

        let rejected: BTreeSet<usize> = report.rejected.iter().map(|r| r.group).collect();
        let expected_rejected: BTreeSet<usize> = rejected_plan.iter().copied().collect();
        assert_eq!(rejected, expected_rejected, "{rejected_plan:?}: {report:?}");
        let expected_committed: BTreeSet<usize> =
            (0..9).filter(|n| !rejected_plan.contains(n)).collect();
        assert_eq!(
            committed_groups(&report),
            expected_committed,
            "{rejected_plan:?}"
        );
        assert!(report.committed.len() > 1 || expected_committed.len() <= 1);

        let held = evidence_entries(&mut session);
        let added: Vec<&String> = held.keys().filter(|id| !seeded.contains_key(*id)).collect();
        let any = !expected_committed.is_empty();
        assert_eq!(
            added.len(),
            usize::from(any),
            "{rejected_plan:?}: {added:?}"
        );
        assert_eq!(evidence.is_committed(shared), any, "{rejected_plan:?}");

        let heads = assertions(&mut session);
        for (n, assertion) in claims.iter().enumerate() {
            if rejected_plan.contains(&n) {
                assert!(!heads.contains_key(&assertion.to_string()));
                continue;
            }
            assert_eq!(
                explained_payload(&mut session, *assertion, shared),
                Some(base64(b"the shared letter\n")),
                "{rejected_plan:?}: citer {n}"
            );
        }
    }
}

/// One set reused across two runs: the second run's groups cite an entry the first committed and
/// one it never saw. Nothing is added twice and nothing is rejected.
#[test]
fn one_set_across_two_runs_adds_each_entry_once() {
    let world = World::new();
    let mut session = world.seeded();
    let seeded = evidence_entries(&mut session);
    let mut evidence = EvidenceSet::new(operator());
    let old = evidence.cite(item("the first letter"));
    let first: Vec<Vec<Operation>> = (0..3)
        .map(|n| group(&format!("first-{n}"), id(ORGANIZATION), old).0)
        .collect();
    let report = Batcher::new(operator())
        .commit_with_evidence(&mut session, &first, &mut evidence)
        .unwrap();
    assert!(report.rejected.is_empty(), "{report:?}");

    let new = evidence.cite(item("the second letter"));
    let second: Vec<Vec<Operation>> = (0..4)
        .map(|n| {
            let cited = if n % 2 == 0 { old } else { new };
            group(&format!("second-{n}"), id(ORGANIZATION), cited).0
        })
        .collect();
    let report = Batcher::new(operator())
        .with_limits(4, 1 << 20)
        .commit_with_evidence(&mut session, &second, &mut evidence)
        .unwrap();

    assert!(report.rejected.is_empty(), "{report:?}");
    let held = evidence_entries(&mut session);
    let added: BTreeSet<String> = held
        .keys()
        .filter(|id| !seeded.contains_key(*id))
        .cloned()
        .collect();
    assert_eq!(added, BTreeSet::from([old.to_string(), new.to_string()]));
}

/// A transport that lets a second process commit just before the first `commit` it forwards, so
/// that commit finds the head moved.
struct Interfering<'a, T> {
    inner: T,
    world: &'a World,
    interfered: bool,
}

impl<T: Transport> Transport for Interfering<'_, T> {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        if request.verb() == "commit" && !self.interfered {
            self.interfered = true;
            self.world.commit_elsewhere("interloper");
        }
        self.inner.request(request)
    }
}

/// A `Stale` commit applied nothing; the batch proposed again under a new id carries the same
/// `!AddEvidence`, is not refused as `identity-already-exists`, and the entry is held once.
#[test]
fn a_stale_retry_adds_its_entry_once() {
    let world = World::new();
    let mut session = world.seeded();
    let seeded = evidence_entries(&mut session);
    let mut evidence = EvidenceSet::new(operator());
    let cited = evidence.cite(item("a stale letter"));
    let groups: Vec<Vec<Operation>> = (0..3)
        .map(|n| group(&format!("stale-{n}"), id(ORGANIZATION), cited).0)
        .collect();
    let mut interfering = Interfering {
        inner: &mut session,
        world: &world,
        interfered: false,
    };

    let report = Batcher::new(operator())
        .commit_with_evidence(&mut interfering, &groups, &mut evidence)
        .unwrap();

    assert!(report.rejected.is_empty(), "{report:?}");
    assert_eq!(report.committed.len(), 1);
    assert_eq!(report.committed[0].stale.len(), 1, "{report:?}");
    let held = evidence_entries(&mut session);
    let added: Vec<&String> = held.keys().filter(|id| !seeded.contains_key(*id)).collect();
    assert_eq!(added, [&cited.to_string()]);
    assert!(evidence.is_committed(cited));
}

/// Forwards every request; the `lose`-th `commit` is forwarded and applied, and its reply lost.
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

/// `docs/sdk.md` § Evidence items, followed as written: a commit whose reply is lost leaves its
/// entries unmarked; `ekr transactions --state Committed` lists it, the consumer marks the entries
/// its groups cite, and sends the groups neither committed nor in that transaction again with the
/// same set. Every entry is held once and nothing is rejected.
#[test]
fn an_unknown_outcome_settled_as_documented_adds_each_entry_once() {
    let world = World::new();
    let mut session = world.seeded();
    let seeded = evidence_entries(&mut session);
    let mut evidence = EvidenceSet::new(operator());
    let items: Vec<EvidenceId> = (0..4)
        .map(|n| evidence.cite(item(&format!("letter {n}"))))
        .collect();
    let groups: Vec<Vec<Operation>> = (0..8)
        .map(|n| group(&format!("unknown-{n}"), id(ORGANIZATION), items[n % 4]).0)
        .collect();
    let mut losing = LosesCommitReply {
        inner: &mut session,
        lose: 2,
        commits: 0,
    };

    let error = Batcher::new(operator())
        .with_limits(3, 1 << 20)
        .commit_with_evidence(&mut losing, &groups, &mut evidence)
        .unwrap_err();

    assert!(matches!(
        *error.cause,
        CallError::Transport(TransportError::TimedOut { .. })
    ));
    let unknown = error.outcome_unknown.clone().expect("the lost commit");
    let cited: BTreeSet<EvidenceId> = unknown
        .groups
        .iter()
        .flat_map(|&group| match &groups[group][1] {
            Operation::AddAssertion(claim) => claim.evidence.iter().copied().collect::<Vec<_>>(),
            _ => Vec::new(),
        })
        .collect();
    assert!(
        cited.iter().any(|entry| !evidence.is_committed(*entry)),
        "the lost transaction added an entry the set does not know is committed"
    );
    assert!(transactions(&mut session, "Committed").contains(&unknown.transaction.to_string()));
    for entry in cited {
        evidence.mark_committed(entry);
    }
    let done: BTreeSet<usize> = committed_groups(&error.report)
        .into_iter()
        .chain(unknown.groups.iter().copied())
        .collect();
    let remaining: Vec<Vec<Operation>> = (0..8)
        .filter(|n| !done.contains(n))
        .map(|n| groups[n].clone())
        .collect();

    let report = Batcher::new(operator())
        .commit_with_evidence(&mut session, &remaining, &mut evidence)
        .unwrap();

    assert!(report.rejected.is_empty(), "{report:?}");
    let held = evidence_entries(&mut session);
    let added: BTreeSet<String> = held
        .keys()
        .filter(|id| !seeded.contains_key(*id))
        .cloned()
        .collect();
    let minted: BTreeSet<String> = items.iter().map(ToString::to_string).collect();
    assert_eq!(added, minted);
}

/// Payload boundaries through a real `ekr`: the empty item and a non-UTF-8 item of exactly
/// 16,384 bytes hash as `ekr hash` does, commit, and `explain` returns their bytes. An item of
/// 16,385 bytes hashes as `ekr hash` does too, and every group citing it is rejected as
/// `Rejection::Document` naming `sequence_elements`, while the other groups commit.
#[test]
fn payload_boundaries_commit_or_are_rejected_as_documents() {
    let world = World::new();
    let mut session = world.seeded();
    let seeded = evidence_entries(&mut session);
    let at = Timestamp::from_millis(1_790_000_000_000);
    let full: Vec<u8> = (0..16_384_usize)
        .map(|n| u8::try_from((n * 7 + 128) % 256).unwrap())
        .collect();
    assert!(std::str::from_utf8(&full).is_err());
    let mut over = full.clone();
    over.push(0xff);
    let items = [
        EvidenceItem::new("a reader", at, Vec::new()),
        EvidenceItem::new("a reader", at, full.clone()),
        EvidenceItem::new("a reader", at, over.clone()),
    ];
    for item in &items {
        let printed = ekr_json(&["hash", "-"], &item.bytes);
        assert_eq!(
            item.content_hash().to_string(),
            printed["content_hash"].as_str().unwrap(),
            "{} bytes",
            item.bytes.len()
        );
    }
    let mut evidence = EvidenceSet::new(operator());
    let ids: Vec<EvidenceId> = items
        .iter()
        .map(|item| evidence.cite(item.clone()))
        .collect();
    let mut groups = Vec::new();
    let mut claims = Vec::new();
    for (n, cited) in [ids[0], ids[2], ids[1], ids[2]].into_iter().enumerate() {
        let (operations, assertion) = group(&format!("bound-{n}"), id(ORGANIZATION), cited);
        groups.push(operations);
        claims.push(assertion);
    }

    let report = Batcher::new(operator())
        .commit_with_evidence(&mut session, &groups, &mut evidence)
        .unwrap();

    let rejected: BTreeSet<usize> = report.rejected.iter().map(|r| r.group).collect();
    assert_eq!(rejected, BTreeSet::from([1, 3]), "{report:?}");
    assert!(report.rejected.iter().all(|r| matches!(
        &r.rejection,
        Rejection::Document(reason) if reason.contains("sequence_elements")
    )));
    assert_eq!(committed_groups(&report), BTreeSet::from([0, 2]));
    assert!(!evidence.is_committed(ids[2]));
    let held = evidence_entries(&mut session);
    let added: BTreeSet<String> = held
        .keys()
        .filter(|id| !seeded.contains_key(*id))
        .cloned()
        .collect();
    assert_eq!(
        added,
        BTreeSet::from([ids[0].to_string(), ids[1].to_string()])
    );
    assert_eq!(
        explained_payload(&mut session, claims[0], ids[0]),
        Some(String::new())
    );
    assert_eq!(
        explained_payload(&mut session, claims[2], ids[1]),
        Some(base64(&full))
    );
}
