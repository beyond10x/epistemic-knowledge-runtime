//! `story:sdk-evidence-attachment`: per-item evidence travels with the assertions that cite it.
//!
//! A consumer cites an [`EvidenceItem`] through an [`EvidenceSet`], which hashes its bytes and
//! mints its id locally, and a [`Batcher`] committing with that set puts the item's
//! `!AddEvidence` into the group of the first assertion citing it that it submits. Every case
//! drives a real `ekr`, built once from this checkout (`ekr_path`), through one child
//! `ekr session`.

#![cfg(unix)]

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;

use ekr_sdk::batch::{BatchReport, Batcher, Rejection};
use ekr_sdk::binary::EkrBinary;
use ekr_sdk::document::{
    AgentId, Assertion, AssertionId, EvidenceId, GraphRootId, NodeDraft, Object, Operation,
    Predicate, PropertyId, Subject, Timestamp, TypeId, Value,
};
use ekr_sdk::evidence::{EvidenceItem, EvidenceSet};
use ekr_sdk::reply::Answer;
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{Request, Transport};
use serde_json::Value as Json;

const BACKENDS: [Backend; 2] = [Backend::File, Backend::Sqlite];
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const LEGAL_NAME: &str = "00000000-0000-4000-8000-000000000801";
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

    /// A new session on the store.
    fn session(&self) -> ProcessSession {
        let binary = EkrBinary::open(ekr_path()).unwrap();
        let options = SessionOptions {
            current_dir: Some(self.path().to_path_buf()),
            ..SessionOptions::default()
        };
        ProcessSession::start(&binary, self.store(), options).unwrap()
    }

    /// A session that has seeded the store.
    fn seeded(&self) -> ProcessSession {
        let mut session = self.session();
        let seeded = ok(&mut session, Request::new(["seed", "seed.yaml"]));
        assert_eq!(seeded["result"]["revision"], 0);
        session
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

/// Standard base64 with padding, as `ekr explain` prints an evidence link's `payload`.
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

/// Item `n`: one of five readers' messages, each observed at its own instant.
fn item(n: usize) -> EvidenceItem {
    EvidenceItem::new(
        format!("reader-{}", n % 5),
        Timestamp::from_millis(1_790_000_000_000 + i64::try_from(n).unwrap()),
        format!("Message {n}: the organisation's legal name was read off its letterhead.\n")
            .into_bytes(),
    )
}

/// One group: a new organisation and one assertion of its legal name, citing `evidence`.
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

/// The payload of every evidence link `ekr explain` prints for `assertion`, by evidence id. It
/// lists the evidence its assertion cites and every other id its transaction's `evidence` lists
/// (`docs/cli.md`, `ekr explain`), so each payload is also held to the bytes `evidence` minted
/// that id for.
fn explained_evidence(
    transport: &mut dyn Transport,
    assertion: AssertionId,
    evidence: &EvidenceSet,
) -> BTreeMap<String, String> {
    let explained = ok(
        transport,
        Request::new(["explain".to_owned(), assertion.to_string()]),
    );
    let links: BTreeMap<String, String> = explained["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|link| link["kind"] == "Evidence")
        .map(|link| {
            (
                link["id"].as_str().unwrap().to_owned(),
                link["payload"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    for (id, payload) in &links {
        let entry = evidence
            .entry(id.parse().unwrap())
            .expect("an id the set minted");
        assert_eq!(
            payload,
            &base64(&entry.payload),
            "{assertion}: evidence {id}"
        );
    }
    links
}

/// Every group index the report lists as committed, sorted.
fn committed_groups(report: &BatchReport) -> Vec<usize> {
    let mut groups: Vec<usize> = report
        .committed
        .iter()
        .flat_map(|committed| committed.groups.iter().copied())
        .collect();
    groups.sort_unstable();
    groups
}

/// Acceptance: 100 assertions citing 40 items produce exactly 40 evidence entries, and `explain`
/// on each assertion returns its item's bytes. The groups are packed into batches of 25
/// operations, so an item first cited in one batch is cited again, by its existing id, in later
/// ones.
#[test]
fn one_hundred_assertions_citing_forty_items_add_forty_entries_each_explained() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut session = world.seeded();
        let seeded = evidence_entries(&mut session);
        let mut evidence = EvidenceSet::new(operator());
        let mut groups = Vec::new();
        let mut cited = Vec::new();
        for n in 0..100 {
            let cites = evidence.cite(item(n % 40));
            let (operations, assertion) = group(&format!("org-{n}"), id(ORGANIZATION), cites);
            groups.push(operations);
            cited.push((assertion, n % 40, cites));
        }
        assert_eq!(
            evidence.len(),
            40,
            "{backend:?}: citing an item again reuses its id"
        );

        let report = Batcher::new(operator())
            .with_limits(25, 1 << 20)
            .commit_with_evidence(&mut session, &groups, &mut evidence)
            .unwrap();

        assert!(report.rejected.is_empty(), "{backend:?}: {report:?}");
        assert!(report.refused.is_none(), "{backend:?}: {report:?}");
        assert!(report.committed.len() > 1, "{backend:?}: several batches");
        assert_eq!(committed_groups(&report), (0..100).collect::<Vec<_>>());

        let held = evidence_entries(&mut session);
        let added: BTreeSet<&String> = held.keys().filter(|id| !seeded.contains_key(*id)).collect();
        assert_eq!(added.len(), 40, "{backend:?}: exactly 40 evidence entries");
        let minted: BTreeSet<String> = cited.iter().map(|(_, _, id)| id.to_string()).collect();
        assert_eq!(
            added.into_iter().cloned().collect::<BTreeSet<_>>(),
            minted,
            "{backend:?}"
        );

        for (assertion, n, cites) in &cited {
            let links = explained_evidence(&mut session, *assertion, &evidence);
            assert_eq!(
                links.get(&cites.to_string()),
                Some(&base64(&item(*n).bytes)),
                "{backend:?}: {assertion} returns its item's bytes"
            );
            assert!(evidence.is_committed(*cites), "{backend:?}");
        }
    }
}

/// Acceptance: a planted rejection never commits an assertion without its evidence. Group 3, the
/// first to cite item 3, is refused (its node's type is undeclared) and bisected out; groups 13
/// and 23 cite item 3 too and are committed with it. Item 10 is cited only by the refused group
/// and is never committed.
#[test]
fn a_planted_rejection_never_commits_an_assertion_without_its_evidence() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut session = world.seeded();
        let seeded = evidence_entries(&mut session);
        let mut evidence = EvidenceSet::new(operator());
        let mut groups = Vec::new();
        let mut cited = Vec::new();
        for n in 0..30 {
            let cites = evidence.cite(item(n % 10));
            let type_id = if n == 3 {
                id(UNDECLARED_TYPE)
            } else {
                id(ORGANIZATION)
            };
            let (mut operations, assertion) = group(&format!("org-{n}"), type_id, cites);
            if n == 3 {
                let orphan = evidence.cite(item(10));
                let Some(Operation::AddAssertion(claim)) = operations.get_mut(1) else {
                    panic!("the group's assertion")
                };
                claim.evidence.insert(orphan);
            }
            groups.push(operations);
            cited.push((assertion, n % 10, cites));
        }
        let orphan = evidence.cite(item(10));

        let report = Batcher::new(operator())
            .commit_with_evidence(&mut session, &groups, &mut evidence)
            .unwrap();

        let rejected: BTreeSet<usize> = report.rejected.iter().map(|r| r.group).collect();
        assert_eq!(rejected, BTreeSet::from([3]), "{backend:?}: {report:?}");
        assert!(
            report.rejected.iter().all(|r| matches!(
                &r.rejection,
                Rejection::Rejected { issues, .. } if issues.iter().any(|i| i.code == "unknown-type")
            )),
            "{backend:?}: {report:?}"
        );
        let others: Vec<usize> = (0..30).filter(|&n| n != 3).collect();
        assert_eq!(committed_groups(&report), others, "{backend:?}");

        let held = evidence_entries(&mut session);
        let added: BTreeSet<String> = held
            .keys()
            .filter(|id| !seeded.contains_key(*id))
            .cloned()
            .collect();
        let expected: BTreeSet<String> = (0..10).map(|n| cited[n].2.to_string()).collect();
        assert_eq!(
            added, expected,
            "{backend:?}: items 0-9, each once, and not item 10"
        );
        assert!(!evidence.is_committed(orphan), "{backend:?}");

        let heads = assertions(&mut session);
        for (assertion, n, cites) in &cited {
            if assertion == &cited[3].0 {
                assert!(!heads.contains_key(&assertion.to_string()), "{backend:?}");
                continue;
            }
            for cited_id in heads[&assertion.to_string()]["evidence"]
                .as_array()
                .unwrap()
            {
                assert!(
                    held.contains_key(cited_id.as_str().unwrap()),
                    "{backend:?}: {assertion} cites {cited_id}, which the head holds"
                );
            }
            let links = explained_evidence(&mut session, *assertion, &evidence);
            assert_eq!(
                links.get(&cites.to_string()),
                Some(&base64(&item(*n).bytes)),
                "{backend:?}: {assertion}"
            );
            assert!(
                !links.contains_key(&orphan.to_string()),
                "{backend:?}: {assertion}"
            );
        }
    }
}

/// A consumer that restarts adds no second entry for an item the store holds: the first run
/// commits items 0-9 through one set and closes its session; the second, in a new session,
/// rebuilds its set from the store with `EvidenceSet::from_store`, cites items 0-14 and adds only
/// items 10-14. The rebuilt set cites the first run's ids.
#[test]
fn two_runs_over_one_store_with_a_rebuilt_set_add_each_entry_once() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut first_session = world.seeded();
        let seeded = evidence_entries(&mut first_session);
        let mut first = EvidenceSet::new(operator());
        let mut groups = Vec::new();
        let mut first_ids = Vec::new();
        for n in 0..10 {
            let cites = first.cite(item(n));
            groups.push(group(&format!("first-{n}"), id(ORGANIZATION), cites).0);
            first_ids.push(cites);
        }
        let report = Batcher::new(operator())
            .commit_with_evidence(&mut first_session, &groups, &mut first)
            .unwrap();
        assert!(report.rejected.is_empty(), "{backend:?}: {report:?}");
        first_session.close().unwrap();

        let mut session = world.session();
        let mut second = EvidenceSet::from_store(&mut session, operator()).unwrap();
        let mut groups = Vec::new();
        let mut cited = Vec::new();
        for n in 0..15 {
            let cites = second.cite(item(n));
            let (operations, assertion) = group(&format!("second-{n}"), id(ORGANIZATION), cites);
            groups.push(operations);
            cited.push((assertion, n, cites));
        }
        assert_eq!(
            cited[..10].iter().map(|(_, _, id)| *id).collect::<Vec<_>>(),
            first_ids,
            "{backend:?}: the rebuilt set cites the entries the store holds"
        );
        let report = Batcher::new(operator())
            .commit_with_evidence(&mut session, &groups, &mut second)
            .unwrap();
        assert!(report.rejected.is_empty(), "{backend:?}: {report:?}");
        assert_eq!(committed_groups(&report), (0..15).collect::<Vec<_>>());

        let held = evidence_entries(&mut session);
        let added: BTreeSet<String> = held
            .keys()
            .filter(|id| !seeded.contains_key(*id))
            .cloned()
            .collect();
        let expected: BTreeSet<String> = cited.iter().map(|(_, _, id)| id.to_string()).collect();
        assert_eq!(added, expected, "{backend:?}: 15 entries, each once");
        for (assertion, n, cites) in &cited {
            assert!(second.is_committed(*cites), "{backend:?}");
            let explained = ok(
                &mut session,
                Request::new(["explain".to_owned(), assertion.to_string()]),
            );
            let payload = explained["links"]
                .as_array()
                .unwrap()
                .iter()
                .find(|link| link["kind"] == "Evidence" && link["id"] == cites.to_string())
                .map(|link| link["payload"].clone());
            assert_eq!(
                payload,
                Some(Json::String(base64(&item(*n).bytes))),
                "{backend:?}: {assertion}"
            );
        }
    }
}

/// Acceptance: the SDK's hashes for the same items equal `ekr hash`, and the entry it builds
/// carries that hash, its item's source identity, observed-at time and bytes, and the operator.
#[test]
fn the_sdk_s_hashes_for_the_same_items_equal_ekr_hash() {
    let mut items: Vec<EvidenceItem> = (0..12).map(item).collect();
    items.push(EvidenceItem::new("a reader", Timestamp::EPOCH, Vec::new()));
    items.push(EvidenceItem::new(
        "a reader",
        Timestamp::EPOCH,
        vec![0, 159, 146, 150, 255, b'\n'],
    ));
    items.push(EvidenceItem::new(
        "a reader",
        Timestamp::EPOCH,
        b"no newline".to_vec(),
    ));
    let mut evidence = EvidenceSet::new(operator());
    for item in &items {
        let printed = ekr_json(&["hash", "-"], &item.bytes);
        let expected = printed["content_hash"].as_str().unwrap();
        assert_eq!(item.content_hash().to_string(), expected, "{item:?}");

        let cites = evidence.cite(item.clone());
        let entry = evidence.entry(cites).expect("the set holds what it cited");
        assert_eq!(entry.evidence.id, cites);
        assert_eq!(
            entry.evidence.content_hash.to_string(),
            expected,
            "{item:?}"
        );
        assert_eq!(entry.payload, item.bytes);
        assert_eq!(entry.evidence.observed_at, item.observed_at);
        assert_eq!(entry.evidence.extracted_by, operator());
        assert_eq!(
            entry.evidence.source,
            ekr_sdk::document::EvidenceSource::human(item.source.clone())
        );
        assert_eq!(
            evidence.cite(item.clone()),
            cites,
            "the same item, the same id"
        );
    }
    assert_eq!(evidence.len(), items.len());
}
