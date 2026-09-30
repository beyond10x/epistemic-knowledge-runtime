//! `story:sdk-resolve-and-batch`: the SDK resolves-or-creates through a cache.
//!
//! Every case drives a real `ekr`, built once from this checkout (`ekr_path`), through one child
//! `ekr session`. A "second process" is the same binary run one-shot against the same store while
//! the session holds it.

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
    AgentId, AliasAddition, GraphRootId, NodeDraft, NodeId, Operation, PropertyId,
    PropertyMutation, TransactionBuilder, TypeId, TypedReference, Value,
};
use ekr_sdk::reply::Answer;
use ekr_sdk::resolve::{Flushed, Resolution, Resolver};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{RecordingTransport, Request, Transport};
use serde_json::Value as Json;

const BACKENDS: [Backend; 2] = [Backend::File, Backend::Sqlite];
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const PERSON: &str = "00000000-0000-4000-8000-000000000201";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const LEGAL_NAME: &str = "00000000-0000-4000-8000-000000000801";

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

fn person() -> TypeId {
    id(PERSON)
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

    /// `ekr <args>` as a second process over the same store, `stdin` on its standard input:
    /// the exit-0 document.
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

/// The document of an exit-0 answer.
fn ok(transport: &mut dyn Transport, request: Request) -> Json {
    let reply = transport.request(&request).unwrap();
    match reply.answer() {
        Answer::Outcome(outcome) => outcome.document().clone(),
        other => panic!("{:?}: {other:?}", request.argv),
    }
}

/// How many `resolve` requests a recording holds.
fn resolves<T: Transport>(recording: &RecordingTransport<T>) -> usize {
    recording
        .recording()
        .exchanges
        .iter()
        .filter(|exchange| exchange.request.verb() == "resolve")
        .count()
}

/// The node a resolution names, whether it was found or queued.
fn node_of(resolution: &Resolution) -> NodeId {
    match resolution {
        Resolution::Resolved(node) | Resolution::Queued(node) => *node,
        Resolution::Ambiguous(candidates) => panic!("ambiguous: {candidates:?}"),
    }
}

/// The key the story names: the exact type id and the sorted, distinct, non-empty aliases.
fn key(reference: &TypedReference) -> (TypeId, Vec<String>) {
    let aliases: BTreeSet<String> = reference
        .aliases
        .iter()
        .filter(|alias| !alias.is_empty())
        .cloned()
        .collect();
    (reference.type_id, aliases.into_iter().collect())
}

/// Every node at the head of `type_id`, by id, with its aliases.
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

/// An organization reference to `org-<n>`, written one of three equivalent ways: order, a
/// repeat and an empty alias do not change the key.
fn organization_reference(n: usize, variant: usize) -> TypedReference {
    let alias = format!("org-{n}");
    let aliases = match variant % 3 {
        0 => vec![alias],
        1 => vec![alias.clone(), alias],
        _ => vec![String::new(), alias],
    };
    TypedReference::new(organization(), aliases)
}

/// `org-5` under a second alias: a different key sharing an alias with `org-5`'s.
fn overlapping_reference(variant: usize) -> TypedReference {
    let aliases = if variant.is_multiple_of(2) {
        vec!["org-5-inc", "org-5"]
    } else {
        vec!["org-5", "org-5-inc", "org-5"]
    };
    TypedReference::new(organization(), aliases)
}

fn person_reference(n: usize, variant: usize) -> TypedReference {
    let aliases = if variant.is_multiple_of(2) {
        vec![format!("person-{n}")]
    } else {
        vec![format!("person-{n}"), format!("person-{n}")]
    };
    TypedReference::new(person(), aliases)
}

/// Acceptance: resolve requests equal the number of distinct references in a fixture with
/// repeats. 240 messages each name an organization and a person; five organizations exist before
/// the resolver starts, the rest are created through it. Between chunks the resolver flushes and
/// the consumer commits one `UpdateProperty` per message through a `Batcher`, which the resolver
/// observes, so its own commits never drop the cache.
#[test]
fn resolve_requests_equal_the_distinct_references_of_a_fixture_with_repeats() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut session = world.seeded();
        let batcher = Batcher::new(operator());

        let existing: Vec<Vec<Operation>> = (0..5)
            .map(|n| {
                let alias = format!("org-{n}");
                vec![NodeDraft::new(root(), organization(), &alias)
                    .with_alias(alias)
                    .into()]
            })
            .collect();
        let before = batcher.commit(&mut session, &existing).unwrap();
        assert!(before.rejected.is_empty(), "{backend:?}: {before:?}");

        let mut transport = RecordingTransport::record(&mut session);
        let mut resolver = Resolver::new(root(), operator());
        let mut distinct = BTreeSet::new();
        let mut nodes: BTreeMap<(TypeId, Vec<String>), NodeId> = BTreeMap::new();
        let mut calls = 0usize;
        for chunk in (0..240usize).collect::<Vec<_>>().chunks(40) {
            let mut groups = Vec::new();
            for &message in chunk {
                let organization = if message % 17 == 0 {
                    overlapping_reference(message)
                } else {
                    organization_reference((message * 7) % 20, message)
                };
                let person = person_reference((message * 3) % 10, message);
                let found = resolver.resolve(&mut transport, &organization).unwrap();
                let named = person.aliases[0].clone();
                let someone = resolver
                    .resolve_with(&mut transport, &person, || {
                        NodeDraft::new(root(), self::person(), format!("Person {named}"))
                    })
                    .unwrap();
                calls += 2;
                for (reference, resolution) in [(&organization, &found), (&person, &someone)] {
                    distinct.insert(key(reference));
                    let node = node_of(resolution);
                    let first = *nodes.entry(key(reference)).or_insert(node);
                    assert_eq!(first, node, "{backend:?}: one key, one node: {reference:?}");
                }
                groups.push(vec![PropertyMutation::new(
                    node_of(&found),
                    id::<PropertyId>(LEGAL_NAME),
                    vec![Value::String(format!("message {message}"))],
                )
                .into()]);
            }
            let flushed: Flushed = resolver.flush(&mut transport).unwrap();
            assert!(
                flushed.report.rejected.is_empty(),
                "{backend:?}: {flushed:?}"
            );
            assert!(flushed.replaced.is_empty(), "{backend:?}: {flushed:?}");
            let report = batcher.commit(&mut transport, &groups).unwrap();
            assert!(report.rejected.is_empty(), "{backend:?}: {report:?}");
            resolver.observe(&report, &groups);
        }

        assert_eq!(calls, 480);
        assert_eq!(
            distinct.len(),
            31,
            "{backend:?}: 20 + 1 organizations, 10 people"
        );
        assert_eq!(
            resolves(&transport),
            distinct.len(),
            "{backend:?}: resolve requests"
        );
        // `org-5` and its overlapping key name one node.
        assert_eq!(
            nodes[&key(&overlapping_reference(0))],
            nodes[&key(&organization_reference(5, 0))]
        );

        let organizations = nodes_of(&mut transport, organization());
        assert_eq!(
            organizations.len(),
            21,
            "{backend:?}: Acme and org-0..org-19"
        );
        for n in 0..20 {
            let alias = format!("org-{n}");
            let holders = organizations
                .values()
                .filter(|aliases| aliases.contains(&alias))
                .count();
            assert_eq!(holders, 1, "{backend:?}: {alias}");
        }
        assert_eq!(nodes_of(&mut transport, person()).len(), 12, "{backend:?}");
    }
}

/// Invalidation is per alias: after a consumer's `AddAlias` that the resolver observes, a key
/// holding that alias is resolved again (and is now `Ambiguous`, returned as a value), while keys
/// without it stay cached; `invalidate` drops exactly the keys holding one alias of one type.
#[test]
fn invalidation_is_per_alias_and_ambiguous_is_a_value() {
    let world = World::new(Backend::File);
    let mut session = world.seeded();
    let mut transport = RecordingTransport::record(&mut session);
    let mut resolver = Resolver::new(root(), operator());
    let batcher = Batcher::new(operator());

    let alpha = TypedReference::new(organization(), ["alpha"]);
    let beta = TypedReference::new(organization(), ["beta"]);
    let alpha_or_gamma = TypedReference::new(organization(), ["gamma", "alpha"]);
    let p = node_of(&resolver.resolve(&mut transport, &alpha).unwrap());
    let q = node_of(&resolver.resolve(&mut transport, &beta).unwrap());
    let flushed = resolver.flush(&mut transport).unwrap();
    assert_eq!(flushed.report.committed.len(), 1, "{flushed:?}");
    assert_eq!(
        resolver.resolve(&mut transport, &alpha_or_gamma).unwrap(),
        Resolution::Resolved(p)
    );
    assert_eq!(resolves(&transport), 3);

    let groups = vec![vec![AliasAddition::new(q, "gamma").into()]];
    let report = batcher.commit(&mut transport, &groups).unwrap();
    assert!(report.rejected.is_empty(), "{report:?}");
    resolver.observe(&report, &groups);
    resolver.flush(&mut transport).unwrap();

    let mut both = [p, q];
    both.sort();
    assert_eq!(
        resolver.resolve(&mut transport, &alpha_or_gamma).unwrap(),
        Resolution::Ambiguous(both.to_vec())
    );
    assert_eq!(
        resolves(&transport),
        4,
        "the key holding gamma was asked again"
    );
    assert_eq!(
        resolver.resolve(&mut transport, &alpha).unwrap(),
        Resolution::Resolved(p)
    );
    assert_eq!(resolves(&transport), 4, "alpha alone stays cached");

    resolver.invalidate(organization(), "alpha");
    assert_eq!(
        resolver.resolve(&mut transport, &alpha).unwrap(),
        Resolution::Resolved(p)
    );
    assert_eq!(resolves(&transport), 5, "alpha was invalidated");
    assert_eq!(
        resolver.resolve(&mut transport, &beta).unwrap(),
        Resolution::Resolved(q)
    );
    assert_eq!(resolves(&transport), 5, "beta was not");
}

/// `task:resolver-queue-drops-shared-alias-answers`: queuing a node drops every cached answer of
/// its type that shares an alias with it. `M` holds `X`; `{Ada, X}` resolves to `M` and is cached;
/// `["Ada"]` then queues `N`, and `{Ada, X}` must answer what `ekr resolve` answers once `N` is
/// flushed, `Ambiguous` with `M` and `N`, not `M` from the cache. A key of another type holding
/// `Ada`, and keys of the same type sharing no alias with `N`, are still the cache's.
#[test]
fn queuing_a_node_drops_cached_answers_that_share_its_aliases() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut session = world.seeded();
        let m = NodeDraft::new(root(), organization(), "M").with_alias("X");
        let o = NodeDraft::new(root(), organization(), "O").with_alias("Y");
        let p = NodeDraft::new(root(), person(), "P").with_alias("Ada");
        let (m_id, o_id, p_id) = (m.id, o.id, p.id);
        let existing = vec![vec![m.into()], vec![o.into()], vec![p.into()]];
        let before = Batcher::new(operator())
            .commit(&mut session, &existing)
            .unwrap();
        assert!(before.rejected.is_empty(), "{backend:?}: {before:?}");

        let mut transport = RecordingTransport::record(&mut session);
        let mut resolver = Resolver::new(root(), operator());
        let x = TypedReference::new(organization(), ["X"]);
        let y = TypedReference::new(organization(), ["Y"]);
        let ada_x = TypedReference::new(organization(), ["Ada", "X"]);
        let ada = TypedReference::new(organization(), ["Ada"]);
        let ada_person = TypedReference::new(person(), ["Ada"]);

        // 1 and 2, and the keys that must survive: another type holding `Ada`, and `Y`.
        assert_eq!(
            resolver.resolve(&mut transport, &x).unwrap(),
            Resolution::Resolved(m_id),
            "{backend:?}: step 1"
        );
        assert_eq!(
            resolver.resolve(&mut transport, &ada_x).unwrap(),
            Resolution::Resolved(m_id),
            "{backend:?}: step 2"
        );
        assert_eq!(
            resolver.resolve(&mut transport, &ada_person).unwrap(),
            Resolution::Resolved(p_id),
            "{backend:?}"
        );
        assert_eq!(
            resolver.resolve(&mut transport, &y).unwrap(),
            Resolution::Resolved(o_id),
            "{backend:?}"
        );
        assert_eq!(resolves(&transport), 4, "{backend:?}");

        // 3.
        let queued = resolver.resolve(&mut transport, &ada).unwrap();
        let Resolution::Queued(n_id) = queued else {
            panic!("{backend:?}: step 3: {queued:?}")
        };
        assert_eq!(resolves(&transport), 5, "{backend:?}");

        // A key of another type, and keys sharing no alias with `N`, are still the cache's.
        for (reference, node) in [(&ada_person, p_id), (&x, m_id), (&y, o_id)] {
            assert_eq!(
                resolver.resolve(&mut transport, reference).unwrap(),
                Resolution::Resolved(node),
                "{backend:?}: {reference:?}"
            );
        }
        assert_eq!(
            resolver.resolve(&mut transport, &ada).unwrap(),
            Resolution::Queued(n_id),
            "{backend:?}: the queued node's own key"
        );
        assert_eq!(
            resolves(&transport),
            5,
            "{backend:?}: no resolve asked for a key that shares no alias with N"
        );

        // 4.
        let mut both = [m_id, n_id];
        both.sort();
        assert_eq!(
            resolver.resolve(&mut transport, &ada_x).unwrap(),
            Resolution::Ambiguous(both.to_vec()),
            "{backend:?}: step 4"
        );
        assert_eq!(
            resolves(&transport),
            6,
            "{backend:?}: step 4 asked ekr resolve"
        );
        let flushed = resolver.flush(&mut transport).unwrap();
        assert_eq!(
            flushed.report.committed.len(),
            1,
            "{backend:?}: step 4 flushed N first: {flushed:?}"
        );
        assert!(flushed.replaced.is_empty(), "{backend:?}: {flushed:?}");
        let store = ok(
            &mut transport,
            Request::new(["resolve", "-"]).with_stdin(ada_x.to_yaml().unwrap()),
        );
        assert_eq!(store["kind"], "Ambiguous", "{backend:?}: {store}");
        assert_eq!(
            store["candidates"],
            serde_json::json!([both[0].to_string(), both[1].to_string()]),
            "{backend:?}: the store's answer is the resolver's"
        );
    }
}

/// Acceptance: an alias committed by a second process between two resolves produces no duplicate
/// node and no `alias-already-exists`. Once where the second resolve is a cache hit and the
/// explicit flush finds the foreign commit, once where the second resolve shares an alias with
/// the queued node and flushes first.
#[test]
fn an_alias_committed_by_a_second_process_between_two_resolves_makes_no_duplicate() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut session = world.seeded();
        let mut transport = RecordingTransport::record(&mut session);
        let mut resolver = Resolver::new(root(), operator());

        let acme = TypedReference::new(organization(), ["acme-two"]);
        let queued = resolver.resolve(&mut transport, &acme).unwrap();
        let Resolution::Queued(first) = queued else {
            panic!("{backend:?}: {queued:?}")
        };
        let elsewhere = world.committed_elsewhere(organization(), "acme-two");
        assert_eq!(
            resolver.resolve(&mut transport, &acme).unwrap(),
            Resolution::Queued(first),
            "{backend:?}: the second resolve is the cache's"
        );
        let flushed = resolver.flush(&mut transport).unwrap();
        assert_eq!(
            flushed.replaced,
            BTreeMap::from([(first, Resolution::Resolved(elsewhere))]),
            "{backend:?}: {flushed:?}"
        );
        assert!(
            flushed.report.rejected.is_empty(),
            "{backend:?}: {flushed:?}"
        );
        assert!(
            flushed.report.committed.is_empty(),
            "{backend:?}: {flushed:?}"
        );
        let asked = resolves(&transport);
        assert_eq!(
            resolver.resolve(&mut transport, &acme).unwrap(),
            Resolution::Resolved(elsewhere)
        );
        assert_eq!(resolves(&transport), asked, "{backend:?}: cached again");

        let initech = TypedReference::new(organization(), ["initech"]);
        let Resolution::Queued(second) = resolver.resolve(&mut transport, &initech).unwrap() else {
            panic!("{backend:?}: initech is new")
        };
        let other = world.committed_elsewhere(organization(), "initech");
        let wider = TypedReference::new(organization(), ["initech", "initech-llc"]);
        let sent = transport.recording().exchanges.len();
        assert_eq!(
            resolver.resolve(&mut transport, &wider).unwrap(),
            Resolution::Resolved(other),
            "{backend:?}"
        );
        let verbs: Vec<&str> = transport.recording().exchanges[sent..]
            .iter()
            .map(|exchange| exchange.request.verb())
            .collect();
        assert_eq!(
            verbs,
            ["head", "resolve", "resolve"],
            "{backend:?}: sharing an alias with a queued node flushes first: the head check \
             finds the foreign commit and resolves the queued reference again, then the wider one"
        );
        let flushed = resolver.flush(&mut transport).unwrap();
        assert_eq!(
            flushed.replaced,
            BTreeMap::from([(second, Resolution::Resolved(other))]),
            "{backend:?}: {flushed:?}"
        );
        assert!(
            flushed.report.rejected.is_empty(),
            "{backend:?}: {flushed:?}"
        );

        let rejected = ok(
            &mut transport,
            Request::new(["transactions", "--state", "Rejected"]),
        );
        assert_eq!(rejected, Json::Array(Vec::new()), "{backend:?}");
        let organizations = nodes_of(&mut transport, organization());
        for (alias, holder) in [("acme-two", elsewhere), ("initech", other)] {
            let holders: Vec<&String> = organizations
                .iter()
                .filter(|(_, aliases)| aliases.iter().any(|held| held == alias))
                .map(|(node, _)| node)
                .collect();
            assert_eq!(holders, [&holder.to_string()], "{backend:?}: {alias}");
        }
        assert!(!organizations.contains_key(&first.to_string()));
        assert!(!organizations.contains_key(&second.to_string()));
    }
}
