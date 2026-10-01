//! Adversary pass on `task:resolver-queue-drops-shared-alias-answers`.
//!
//! The unit makes a `ProposeNew` answer drop every other cached key of the queued node's type
//! that shares one of its aliases. These cases drive a real `ekr`, built once from this checkout,
//! through one child `ekr session`, and compare every answer the `Resolver` gives with a model of
//! what `ekr resolve` answers once the queue is flushed, and every `ekr resolve` it asks with the
//! requests a cache that drops exactly those keys would need.

#![cfg(unix)]

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::OnceLock;

use ekr_sdk::batch::Batcher;
use ekr_sdk::binary::EkrBinary;
use ekr_sdk::document::{
    AgentId, GraphRootId, NodeDraft, NodeId, Operation, TransactionBuilder, TypeId, TypedReference,
};
use ekr_sdk::reply::{Answer, Reply};
use ekr_sdk::resolve::{Resolution, Resolver};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{RecordingTransport, Request, Transport, TransportError};
use serde_json::Value as Json;

const BACKENDS: [Backend; 2] = [Backend::File, Backend::Sqlite];
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const PERSON: &str = "00000000-0000-4000-8000-000000000201";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";

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

fn person() -> TypeId {
    id(PERSON)
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

fn resolves<T: Transport>(recording: &RecordingTransport<T>) -> usize {
    recording
        .recording()
        .exchanges
        .iter()
        .filter(|exchange| exchange.request.verb() == "resolve")
        .count()
}

type Key = (TypeId, Vec<String>);

fn key(reference: &TypedReference) -> Key {
    let aliases: BTreeSet<String> = reference
        .aliases
        .iter()
        .filter(|alias| !alias.is_empty())
        .cloned()
        .collect();
    (reference.type_id, aliases.into_iter().collect())
}

/// Every node of the two seeded types at the head: id → (type, aliases).
fn store_nodes(transport: &mut dyn Transport) -> BTreeMap<NodeId, (TypeId, Vec<String>)> {
    let snapshot = ok(transport, Request::new(["snapshot"]));
    let (organization, person) = (organization().to_string(), person().to_string());
    snapshot["graph"]["graph"]["nodes"]
        .as_object()
        .unwrap()
        .values()
        .filter(|node| node["type_id"] == organization || node["type_id"] == person)
        .map(|node| {
            let aliases = node["aliases"]
                .as_array()
                .unwrap()
                .iter()
                .map(|alias| alias.as_str().unwrap().to_owned())
                .collect();
            (
                id::<NodeId>(node["id"].as_str().unwrap()),
                (id::<TypeId>(node["type_id"].as_str().unwrap()), aliases),
            )
        })
        .collect()
}

/// The nodes of `nodes` that `ekr resolve` names as candidates for `key`.
fn candidates(nodes: &BTreeMap<NodeId, (TypeId, Vec<String>)>, key: &Key) -> BTreeSet<NodeId> {
    nodes
        .iter()
        .filter(|(_, (type_id, aliases))| {
            *type_id == key.0 && aliases.iter().any(|alias| key.1.contains(alias))
        })
        .map(|(node, _)| *node)
        .collect()
}

/// A fixed-seed generator: the walk is the same on every run.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self, below: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        usize::try_from((self.0 >> 33) % below as u64).unwrap()
    }
}

/// What the `Resolver` should hold: the nodes it queued and not yet flushed, and the cache that
/// drops exactly the keys `ekr resolve` stops answering the same way once a queued node is
/// committed.
#[derive(Default)]
struct Model {
    pending: BTreeMap<NodeId, (TypeId, Vec<String>)>,
    cache: BTreeMap<Key, NodeId>,
}

/// Property: over a fixed-seed walk of resolves (repeats, overlapping alias sets, the empty
/// alias, two types sharing aliases) and flushes, with no other process writing,
///
/// * every answer is the one `ekr resolve` gives against the store with the queue flushed: a
///   cached `Resolved(M)` is never a key the store would answer `Ambiguous` or with another node;
/// * the resolver asks `ekr resolve` exactly when a cache that drops only the keys sharing an
///   alias of a queued node's type would miss: no stale hit, and no extra request.
#[test]
fn a_fixed_seed_walk_of_resolves_and_flushes_answers_what_the_store_answers() {
    const ALIASES: [&str; 9] = ["a", "b", "c", "d", "e", "f", "g", "h", ""];
    for backend in BACKENDS {
        let mut exercised = 0_usize;
        for seed in [7_u64, 1_234_567, 99_991] {
            let world = World::new(backend);
            let mut session = world.seeded();
            let existing: Vec<Vec<Operation>> = vec![
                vec![NodeDraft::new(root(), organization(), "A")
                    .with_alias("a")
                    .into()],
                vec![NodeDraft::new(root(), organization(), "B")
                    .with_alias("b")
                    .into()],
                vec![NodeDraft::new(root(), person(), "A").with_alias("a").into()],
            ];
            let before = Batcher::new(operator())
                .commit(&mut session, &existing)
                .unwrap();
            assert!(before.rejected.is_empty(), "{before:?}");

            let mut transport = RecordingTransport::record(&mut session);
            let mut resolver = Resolver::new(root(), operator());
            let mut model = Model::default();
            let mut random = Lcg(seed);
            let mut trail: Vec<String> = Vec::new();
            // Keys the model dropped when a node was queued, and how many of them were asked
            // again: each is a step where a resolver that dropped nothing on queue would answer
            // from the cache.
            let mut dropped: BTreeSet<Key> = BTreeSet::new();
            let mut dropped_then_asked = 0_usize;
            let mut history: Vec<(TypeId, Vec<String>)> = Vec::new();

            for step in 0..100 {
                let asked = resolves(&transport);
                if random.next(6) == 0 {
                    trail.push("flush".to_owned());
                    let flushed = resolver.flush(&mut transport).unwrap();
                    assert!(flushed.replaced.is_empty(), "{flushed:?}");
                    assert!(flushed.report.rejected.is_empty(), "{flushed:?}");
                    assert_eq!(
                        resolves(&transport),
                        asked,
                        "{backend:?} seed {seed} step {step}: a flush with no foreign commit \
                         asks nothing: {trail:?}"
                    );
                    let store = store_nodes(&mut transport);
                    model.pending.retain(|node, _| !store.contains_key(node));
                    assert!(model.pending.is_empty(), "{:?}", model.pending);
                    continue;
                }
                // Half the resolves ask a reference asked before, so cached keys are asked again.
                let (type_id, aliases) = if !history.is_empty() && random.next(2) == 0 {
                    history[random.next(history.len())].clone()
                } else {
                    let type_id = if random.next(3) == 0 {
                        person()
                    } else {
                        organization()
                    };
                    let count = 1 + random.next(3);
                    let mut aliases: Vec<String> = (0..count)
                        .map(|_| ALIASES[random.next(ALIASES.len())].to_owned())
                        .collect();
                    if aliases.iter().all(String::is_empty) {
                        aliases.push(ALIASES[random.next(8)].to_owned());
                    }
                    history.push((type_id, aliases.clone()));
                    (type_id, aliases)
                };
                let reference = TypedReference::new(type_id, aliases.clone());
                let key = key(&reference);
                let kind = if type_id == person() { "person" } else { "org" };
                trail.push(format!("{kind} {aliases:?}"));
                let hit = model.cache.contains_key(&key);
                if dropped.remove(&key) && !hit {
                    dropped_then_asked += 1;
                }

                let answer = resolver.resolve(&mut transport, &reference).unwrap();

                assert_eq!(
                    resolves(&transport) - asked,
                    usize::from(!hit),
                    "{backend:?} seed {seed} step {step}: {answer:?}; a cache holding exactly \
                     the keys no queued node shares an alias with would {}: {trail:?}",
                    if hit { "hit" } else { "miss" }
                );
                let store = store_nodes(&mut transport);
                model.pending.retain(|node, _| !store.contains_key(node));
                let mut flushed_view = store.clone();
                flushed_view.extend(model.pending.clone());
                let expected = candidates(&flushed_view, &key);
                match answer {
                    Resolution::Resolved(node) => {
                        assert_eq!(
                            expected,
                            BTreeSet::from([node]),
                            "{backend:?} seed {seed} step {step}: Resolved({node}) where the \
                             flushed store has these candidates: {trail:?}"
                        );
                        model.cache.insert(key, node);
                    }
                    Resolution::Queued(node) if !model.pending.contains_key(&node) => {
                        assert!(
                            expected.is_empty(),
                            "{backend:?} seed {seed} step {step}: queued {node} where the \
                             flushed store holds {expected:?}: {trail:?}"
                        );
                        model.cache.retain(|cached, _| {
                            let shares =
                                cached.0 == key.0 && cached.1.iter().any(|a| key.1.contains(a));
                            if shares {
                                dropped.insert(cached.clone());
                            }
                            !shares
                        });
                        model.pending.insert(node, key.clone());
                        model.cache.insert(key, node);
                    }
                    Resolution::Queued(node) => {
                        assert_eq!(
                            expected,
                            BTreeSet::from([node]),
                            "{backend:?} seed {seed} step {step}: {trail:?}"
                        );
                    }
                    Resolution::Ambiguous(found) => {
                        assert_eq!(
                            found.iter().copied().collect::<BTreeSet<_>>(),
                            expected,
                            "{backend:?} seed {seed} step {step}: {trail:?}"
                        );
                    }
                }
            }
            eprintln!("{backend:?} seed {seed}: {dropped_then_asked} dropped keys asked again");
            exercised += dropped_then_asked;
        }
        assert!(
            exercised > 0,
            "{backend:?}: no walk asked a key a queued node dropped, so none exercises the defect"
        );
    }
}

/// A queued node holding two aliases drops a cached key that shares only the second of them.
/// The unit's own test queues a node with one alias, so a fix that dropped keys for the first
/// alias alone would pass it.
#[test]
fn a_node_queued_under_two_aliases_drops_a_key_sharing_only_its_second_alias() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut session = world.seeded();
        let m = NodeDraft::new(root(), organization(), "M").with_alias("X");
        let m_id = m.id;
        let report = Batcher::new(operator())
            .commit(&mut session, &[vec![m.into()]])
            .unwrap();
        assert!(report.rejected.is_empty(), "{report:?}");

        let mut transport = RecordingTransport::record(&mut session);
        let mut resolver = Resolver::new(root(), operator());
        let zed_x = TypedReference::new(organization(), ["zed", "X"]);
        assert_eq!(
            resolver.resolve(&mut transport, &zed_x).unwrap(),
            Resolution::Resolved(m_id)
        );
        let Resolution::Queued(n_id) = resolver
            .resolve(
                &mut transport,
                &TypedReference::new(organization(), ["zed", "ada"]),
            )
            .unwrap()
        else {
            panic!("{backend:?}: nothing holds ada or zed")
        };
        let mut both = [m_id, n_id];
        both.sort();
        assert_eq!(
            resolver.resolve(&mut transport, &zed_x).unwrap(),
            Resolution::Ambiguous(both.to_vec()),
            "{backend:?}: zed is the queued node's second alias"
        );
    }
}

/// Fails the `fail`-th `resolve` request (counted from 1) without forwarding it.
struct FailsResolve<T> {
    inner: T,
    fail: usize,
    seen: usize,
}

impl<T: Transport> Transport for FailsResolve<T> {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        if request.verb() == "resolve" {
            self.seen += 1;
            if self.seen == self.fail {
                return Err(TransportError::Cancelled {
                    verb: "resolve".to_owned(),
                });
            }
        }
        self.inner.request(request)
    }
}

/// The recheck list and `reconcile`'s `ProposeNew` arm, which queues again without dropping any
/// key: a foreign commit makes a flush resolve both queued nodes again, the second resolve fails,
/// and the second node waits in the recheck list. A key sharing its alias, resolved while it
/// waits, and one cached before the next flush re-queues it, answer what the store answers.
#[test]
fn a_node_waiting_in_the_recheck_list_leaves_no_stale_answer_for_its_aliases() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut session = world.seeded();
        let m = NodeDraft::new(root(), organization(), "M").with_alias("X");
        let m_id = m.id;
        let report = Batcher::new(operator())
            .commit(&mut session, &[vec![m.into()]])
            .unwrap();
        assert!(report.rejected.is_empty(), "{report:?}");

        let mut resolver = Resolver::new(root(), operator());
        let x = TypedReference::new(organization(), ["X"]);
        let bob_x = TypedReference::new(organization(), ["bob", "X"]);
        assert_eq!(
            resolver.resolve(&mut session, &x).unwrap(),
            Resolution::Resolved(m_id)
        );
        let Resolution::Queued(_) = resolver
            .resolve(&mut session, &TypedReference::new(organization(), ["ada"]))
            .unwrap()
        else {
            panic!("{backend:?}: ada is new")
        };
        let Resolution::Queued(bob) = resolver
            .resolve(&mut session, &TypedReference::new(organization(), ["bob"]))
            .unwrap()
        else {
            panic!("{backend:?}: bob is new")
        };
        world.committed_elsewhere(organization(), "zed");
        let mut failing = FailsResolve {
            inner: &mut session,
            fail: 2,
            seen: 0,
        };
        resolver.flush(&mut failing).unwrap_err();

        // bob waits in the recheck list; X is resolved while it waits.
        assert_eq!(
            resolver.resolve(&mut session, &x).unwrap(),
            Resolution::Resolved(m_id),
            "{backend:?}"
        );
        let mut both = [m_id, bob];
        both.sort();
        assert_eq!(
            resolver.resolve(&mut session, &bob_x).unwrap(),
            Resolution::Ambiguous(both.to_vec()),
            "{backend:?}: bob, re-queued from the recheck list, is committed first"
        );
        let flushed = resolver.flush(&mut session).unwrap();
        assert!(flushed.replaced.is_empty(), "{backend:?}: {flushed:?}");
        let store = ok(
            &mut session,
            Request::new(["resolve", "-"]).with_stdin(bob_x.to_yaml().unwrap()),
        );
        assert_eq!(store["kind"], "Ambiguous", "{backend:?}: {store}");
    }
}
