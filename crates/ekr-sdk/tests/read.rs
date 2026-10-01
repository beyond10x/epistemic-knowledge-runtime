//! `story:sdk-read-helpers`: the SDK's typed reads over `ekr session` and one-shot `ekr`.
//!
//! * Every document the `ekr-views` conformance fixture stores render — each fixture built
//!   through the real kernel by `crates/ekr-views/tests/support/fixtures.rs`, at every revision —
//!   reads into its typed reader, and the typed value writes back exactly the document it was
//!   read from. A field a views format gains without an SDK update is left out of that write, so
//!   it fails here by its JSON pointer.
//! * The `expand` iterator pages a 5,000-node fixture store through a real session with nothing
//!   lost or repeated.
//! * A reader in one session sees another session's commits on its next call.
//! * `head`, `ontology`, `snapshot`, `transactions` and `explain` read the same typed value
//!   through a session and as one-shot `ekr` processes, and each is exactly the document `ekr`
//!   printed.
//!
//! Every case that runs `ekr` drives the binary built from this checkout (`ekr_path`), by path.

#![cfg(unix)]

#[path = "../../ekr-views/tests/support/fixtures.rs"]
#[allow(dead_code, clippy::all, clippy::pedantic)]
mod fixtures;

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ekr_core::{AssertionId, EdgeId, NodeId, RevisionNumber, Timestamp, TypeId};
use ekr_sdk::binary::EkrBinary;
use ekr_sdk::read::{
    Bucket, ChangeKind, Changes, ChangesMeta, DetailMeta, DetailNode, ExpandPages, ExpandQuery,
    ExplainedEvidence, Explanation, ExplanationLink, GraphChange, Head, ListedTransaction,
    MatchField, MatchTier, MatchesMeta, ModifiedProperty, NamedType, NodeDetail, NodeMatch,
    NodeMatches, NodeSummary, OneShotReader, Ontology, OntologyCardinality, OntologyEdgeType,
    OntologyNodeType, OntologyProperty, OntologyValueType, Overview, OverviewMeta,
    OverviewRevision, OverviewRoles, OverviewSchema, OverviewTimeline, ReadError, Reader,
    RecordedTime, ReferencingAssertion, Root, SchemaMember, SchemaVersionChange, Since, Slice,
    SliceEdge, SliceMeta, SliceNode, Snapshot, SnapshotAssertion, SnapshotEdge, SnapshotEvidence,
    SnapshotGraph, SnapshotGraphDocument, SnapshotNode, SnapshotPredicate, SnapshotRoot,
    SnapshotSubject, Timeline, TimelineBucket, TimelineCell, TimelineEvent, TimelineMeta,
    TimelineQuery, TimelineRow, TimelineRowType, TimelineStep, TransactionState, Transactions,
    TypeCount, TypeTiming, ValidTime, ViewAssertion, ViewAssessment, ViewEdge, ViewEdgeType,
    ViewLifecycle, ViewNodeType, ViewOntology, ViewProperty, ViewValue, WidenedEnd,
};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{RecordingTransport, Request, Transport};
use ekr_views::{
    BucketWidth, ChangesRequest, ExpandRequest, Index, OverviewRequest, SearchRequest, SinceKind,
    TimelineRequest,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{json, Value};

const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const ALICE: &str = "00000000-0000-4000-8000-000000000301";
const SEEDED_ASSERTION: &str = "00000000-0000-4000-8000-000000000510";
const GLOBEX: (&str, &str) = (
    "00000000-0000-4000-8000-000000000902",
    "00000000-0000-4000-8000-000000000901",
);

// ---- the binary and a store under the example host ---------------------------------------------

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
            .stderr(std::process::Stdio::inherit())
            .output()
            .expect("running cargo build -p ekr");
        assert!(output.status.success(), "cargo build -p ekr failed");
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|message| {
                message["reason"] == "compiler-artifact" && message["target"]["name"] == "ekr"
            })
            .find_map(|message| message["executable"].as_str().map(PathBuf::from))
            .expect("cargo named the ekr executable it built")
    })
}

fn binary() -> EkrBinary {
    EkrBinary::open(ekr_path()).expect("the built ekr meets the SDK's minimum")
}

/// Exit 0 and stdout, from the built binary run directly.
fn ekr_output(args: &[&std::ffi::OsStr]) -> Vec<u8> {
    let output = std::process::Command::new(ekr_path())
        .args(args)
        .env_clear()
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn ekr_text(args: &[&str]) -> String {
    let args: Vec<&std::ffi::OsStr> = args.iter().map(std::ffi::OsStr::new).collect();
    String::from_utf8(ekr_output(&args)).unwrap()
}

/// A directory holding the example host and seed and a store path on the file provider.
struct World {
    directory: tempfile::TempDir,
}

impl World {
    fn new() -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        world.file("host.json", &ekr_text(&["example", "ekr.cli-host/1"]));
        world.file("seed.yaml", &ekr_text(&["example", "ekr-seed/2"]));
        world.file(
            "create.yaml",
            &format!(
                "format: ekr.transaction-document/2\ntransaction:\n  id: {}\n  proposer: \
                 {OPERATOR}\n  operations:\n  - !CreateNode\n    id: {}\n    root_id: {ROOT}\n    \
                 type_id: {ORGANIZATION}\n    canonical_name: Globex\n    properties: {{}}\n    \
                 aliases: [Globex]\n  evidence: []\n",
                GLOBEX.0, GLOBEX.1
            ),
        );
        world
    }

    fn file(&self, name: &str, contents: &str) {
        std::fs::write(self.directory.path().join(name), contents).unwrap();
    }

    fn store(&self) -> StoreConfig {
        StoreConfig {
            host: self.directory.path().join("host.json"),
            store: self.directory.path().join("store"),
            backend: Backend::File,
        }
    }

    fn options(&self) -> SessionOptions {
        SessionOptions {
            current_dir: Some(self.directory.path().to_path_buf()),
            ..SessionOptions::default()
        }
    }

    fn session(&self) -> ProcessSession {
        ProcessSession::start(&binary(), self.store(), self.options()).unwrap()
    }

    /// A session that has seeded the store from `seed.yaml`.
    fn seeded(&self) -> ProcessSession {
        let mut session = self.session();
        assert_eq!(
            ok(&mut session, &["seed", "seed.yaml"])["result"]["revision"],
            0
        );
        session
    }

    /// Proposes, validates and commits `create.yaml` through `transport`: revision 1, Globex.
    fn commit_globex(transport: &mut dyn Transport) {
        ok(transport, &["propose", "create.yaml"]);
        assert_eq!(ok(transport, &["validate", GLOBEX.0])["kind"], "Validated");
        assert_eq!(ok(transport, &["commit", GLOBEX.0])["kind"], "Committed");
    }

    /// `ekr <verb>` under this world's store, run directly: its stdout as JSON.
    fn one_shot(&self, verb: &[&str]) -> Value {
        let store = self.store();
        let mut args: Vec<&std::ffi::OsStr> = vec![
            "--host".as_ref(),
            store.host.as_os_str(),
            "--store".as_ref(),
            store.store.as_os_str(),
            "--backend".as_ref(),
            "file".as_ref(),
        ];
        args.extend(verb.iter().map(std::ffi::OsStr::new));
        serde_json::from_slice(&ekr_output(&args)).unwrap()
    }
}

/// The document of an exit-0 answer.
fn ok(transport: &mut dyn Transport, argv: &[&str]) -> Value {
    let reply = transport
        .request(&Request::new(argv.iter().copied()))
        .unwrap();
    assert_eq!(reply.exit, 0, "{argv:?}: {}", reply.stderr);
    reply.document.unwrap()
}

// ---- exactness: read, and written back unchanged ----------------------------------------------

/// Every leaf of `value` by its JSON pointer.
fn leaves(value: &Value, at: &str, into: &mut BTreeMap<String, Value>) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (key, item) in map {
                let key = key.replace('~', "~0").replace('/', "~1");
                leaves(item, &format!("{at}/{key}"), into);
            }
        }
        Value::Array(items) if !items.is_empty() => {
            for (index, item) in items.iter().enumerate() {
                leaves(item, &format!("{at}/{index}"), into);
            }
        }
        leaf => {
            into.insert(at.to_owned(), leaf.clone());
        }
    }
}

/// `document` read as `T`, or the pointers at which `T` written back differs from it — a field
/// `T` does not know among them — or why it does not read.
fn exactly<T: DeserializeOwned + Serialize>(document: &Value) -> Result<T, Vec<String>> {
    let typed: T =
        serde_json::from_value(document.clone()).map_err(|error| vec![error.to_string()])?;
    let back = serde_json::to_value(&typed).map_err(|error| vec![error.to_string()])?;
    let (mut left, mut right) = (BTreeMap::new(), BTreeMap::new());
    leaves(document, "", &mut left);
    leaves(&back, "", &mut right);
    let drift: Vec<String> = left
        .keys()
        .chain(right.keys())
        .filter(|at| left.get(*at) != right.get(*at))
        .cloned()
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect();
    if drift.is_empty() {
        Ok(typed)
    } else {
        Err(drift)
    }
}

/// [`exactly`], or a failure naming `what` and the drift.
fn exact<T: DeserializeOwned + Serialize>(what: &str, bytes: &[u8]) -> T {
    let document: Value = serde_json::from_slice(bytes).unwrap();
    exactly::<T>(&document).unwrap_or_else(|drift| {
        panic!(
            "{what}: not read exactly as {}: {drift:?}",
            std::any::type_name::<T>()
        )
    })
}

// ---- what each typed document says, checked on every one read ---------------------------------

fn overview_holds(overview: &Overview) {
    let meta: &OverviewMeta = &overview.meta;
    assert_eq!(meta.format, "ekr.graph-overview/1");
    let counted = |types: &[TypeCount]| types.iter().map(|t| t.count).sum::<u64>();
    assert_eq!(counted(&overview.node_types), meta.node_count);
    assert_eq!(counted(&overview.edge_types), meta.edge_count);
    assert!(overview.top.len() as u64 <= meta.limit);
    let top: &[NodeSummary] = &overview.top;
    assert!(top.windows(2).all(|pair| pair[0].degree >= pair[1].degree));
    let ontology: &ViewOntology = &overview.ontology;
    let properties: BTreeSet<&str> = ontology
        .properties
        .iter()
        .map(|property: &ViewProperty| property.id.as_str())
        .collect();
    for node_type in &ontology.node_types {
        let node_type: &ViewNodeType = node_type;
        assert!(node_type
            .properties
            .iter()
            .all(|id| properties.contains(id.as_str())));
        node_type.assertions.iter().for_each(assertion_holds);
    }
    for edge_type in &ontology.edge_types {
        let edge_type: &ViewEdgeType = edge_type;
        assert!(!edge_type.source_types.is_empty() && !edge_type.target_types.is_empty());
        edge_type.assertions.iter().for_each(assertion_holds);
    }
    let schema: &OverviewSchema = &overview.schema;
    let versions: &[SchemaVersionChange] = &schema.versions;
    assert!(!versions.is_empty());
    for version in versions {
        for member in version.added.iter().chain(&version.removed) {
            let member: &SchemaMember = member;
            assert!(!member.kind.is_empty());
        }
        for end in &version.widened {
            let end: &WidenedEnd = end;
            assert!(["Source", "Target"].contains(&end.side.as_str()));
            assert!(!end.node_types.is_empty());
        }
        for property in &version.modified {
            let property: &ModifiedProperty = property;
            assert!(!property.changed.is_empty() && !property.name.is_empty());
        }
    }
    let revisions: &[OverviewRevision] = &schema.revisions;
    assert_eq!(
        revisions.last().map(|revision| revision.number),
        Some(meta.revision)
    );
    let roles: &OverviewRoles = &overview.roles;
    assert!(roles
        .types
        .iter()
        .all(|timing: &TypeTiming| timing.timestamped <= timing.nodes));
    let timeline: &OverviewTimeline = &overview.timeline;
    let buckets: &[TimelineBucket] = &timeline.buckets;
    assert!(buckets.iter().all(|bucket| bucket.assertions > 0));
}

fn assertion_holds(assertion: &ViewAssertion) {
    let assessment: &ViewAssessment = &assertion.assessment;
    let lifecycle: &ViewLifecycle = &assertion.lifecycle;
    assert!(!assessment.kind.is_empty() && !lifecycle.kind.is_empty());
    assert!(assertion.object_value.is_some() || assertion.object_ref.is_some());
    if let Some(value) = &assertion.object_value {
        value_holds(value);
    }
}

fn value_holds(value: &ViewValue) {
    match value {
        ViewValue::List(items) => items.iter().for_each(value_holds),
        ViewValue::Record(fields) => fields.values().for_each(value_holds),
        _ => {}
    }
}

fn matches_hold(matches: &NodeMatches, limit: u64) {
    let meta: &MatchesMeta = &matches.meta;
    assert_eq!(meta.format, "ekr.node-matches/1");
    assert!(matches.matches.len() as u64 <= limit.min(meta.total));
    for found in &matches.matches {
        let found: &NodeMatch = found;
        assert_eq!(found.alias.is_some(), found.field == MatchField::Alias);
        let _: MatchTier = found.tier;
    }
}

fn detail_holds(detail: &NodeDetail, node: NodeId) {
    let meta: &DetailMeta = &detail.meta;
    assert_eq!(meta.format, "ekr.node-detail/1");
    let described: &DetailNode = &detail.node;
    assert_eq!(described.id, node);
    detail.assertions.iter().for_each(assertion_holds);
    for referencing in &detail.referencing {
        let referencing: &ReferencingAssertion = referencing;
        assertion_holds(&referencing.assertion);
    }
    for edge in &detail.edges {
        let edge: &ViewEdge = edge;
        assert!(edge.source == node || edge.target == node);
    }
}

fn slice_holds(slice: &Slice, limit: u64) {
    let meta: &SliceMeta = &slice.meta;
    assert_eq!(meta.format, "ekr.graph-slice/1");
    assert!(slice.nodes.len() as u64 <= limit);
    let nodes: &[SliceNode] = &slice.nodes;
    assert!(nodes.iter().all(|node| node.distance <= meta.depth));
    let edges: &[SliceEdge] = &slice.edges;
    assert!(edges.len() as u64 <= meta.edge_total);
}

fn timeline_holds(timeline: &Timeline) {
    let meta: &TimelineMeta = &timeline.meta;
    assert_eq!(meta.format, "ekr.graph-timeline/1");
    assert!(timeline.rows.len() as u64 <= meta.limit);
    let _: &[TimelineRowType] = &timeline.row_types;
    for row in &timeline.rows {
        let row: &TimelineRow = row;
        let cells: &[TimelineCell] = &row.cells;
        assert_eq!(cells.iter().map(|cell| cell.events).sum::<u64>(), row.total);
    }
    for event in &timeline.events {
        let event: &TimelineEvent = event;
        assert_eq!(event.path.len() as u64, event.distance);
        let _: Option<&TimelineStep> = event.path.first();
    }
}

fn changes_hold(changes: &Changes) {
    let meta: &ChangesMeta = &changes.meta;
    assert_eq!(meta.format, "ekr.graph-changes/1");
    assert!(changes.changes.len() as u64 <= meta.limit);
    let listed: &[GraphChange] = &changes.changes;
    assert!(listed
        .windows(2)
        .all(|pair| pair[0].revision <= pair[1].revision));
    for change in listed {
        let kind: ChangeKind = change.change;
        assert_ne!(kind, ChangeKind::Other, "every kind ekr writes is modelled");
        assert_eq!(
            change.subject.is_some(),
            matches!(
                kind,
                ChangeKind::AssertionAdded
                    | ChangeKind::AssertionSuperseded
                    | ChangeKind::AssertionRetracted
            )
        );
        let added = kind == ChangeKind::EvidenceAdded;
        assert_eq!(change.locator.is_some(), added);
        assert_eq!(change.content_hash.is_some(), added);
        assert!(!added || change.evidence.is_empty());
    }
}

// ---- every conformance fixture document --------------------------------------------------------

const FIXTURES: [&str; 14] = [
    "seed-only",
    "seeded-evidence",
    "edge-assertion",
    "retracted-assertion",
    "schema-evolution",
    "store",
    "hub",
    "timeline",
    "growth",
    "subjects",
    "changes",
    "quality",
    "schema-changes",
    "property-redeclared",
];

/// Every document each conformance fixture store renders, at every revision: the overview at
/// the largest limit, a search matching every node, every node described, every node's
/// neighbourhood paged out at depth 0 and the first node's at depth 2, the timeline of every row
/// type with and without a subject and bucket, and every change after each kind of since, paged
/// out — each read exactly by its typed reader. The snapshot of every revision too.
#[test]
fn every_conformance_fixture_document_reads_exactly_into_its_typed_reader() {
    // The fixtures read `tests/fixtures/seed-empty.yaml` under the running crate's manifest
    // directory: this crate's copy is the views crate's, byte for byte.
    let seed = |krate: &str| {
        std::fs::read(
            workspace_root().join(format!("crates/{krate}/tests/fixtures/seed-empty.yaml")),
        )
        .unwrap()
    };
    assert_eq!(seed("ekr-sdk"), seed("ekr-views"));
    let mut read: BTreeMap<&str, usize> = BTreeMap::new();
    for name in FIXTURES {
        let directory = tempfile::tempdir().unwrap();
        let runtime = fixtures::open(directory.path(), fixtures::Provider::File);
        fixtures::Fixture::named(name)
            .unwrap_or_else(|| panic!("no fixture {name}"))
            .build(&runtime);
        let head = runtime.head().unwrap().unwrap().revision.get();
        for revision in 0..=head {
            let at = format!("{name}@{revision}");
            let index = Index::load(&runtime, Some(RevisionNumber::new(revision))).unwrap();

            let overview = index
                .overview(&OverviewRequest::new(Some(OverviewRequest::MAX_LIMIT)).unwrap())
                .unwrap();
            let typed = exact::<Overview>(&at, &overview.bytes);
            overview_holds(&typed);
            for version in &typed.schema.versions {
                *read.entry("widened").or_default() += version.widened.len();
                *read.entry("modified").or_default() += version.modified.len();
            }
            *read.entry("overview").or_default() += 1;

            let search = SearchRequest::new(String::new(), SearchRequest::MAX_LIMIT).unwrap();
            let found = index.search(&search).unwrap();
            matches_hold(&exact::<NodeMatches>(&at, &found.bytes), 100);
            *read.entry("search").or_default() += 1;

            let mut nodes: Vec<NodeId> = index.loaded().graph.nodes.keys().copied().collect();
            nodes.sort_unstable();
            for node in &nodes {
                let detail = index.describe(*node).unwrap();
                detail_holds(&exact::<NodeDetail>(&at, &detail.bytes), *node);
                *read.entry("describe").or_default() += 1;
            }

            let mut expansions = vec![(nodes.clone(), 0)];
            if let Some(first) = nodes.first() {
                expansions.push((vec![*first], 2));
            }
            for (seeds, depth) in expansions {
                let mut after = 0;
                loop {
                    let request =
                        ExpandRequest::new(seeds.clone(), depth, 97, Some(131), Some(after))
                            .unwrap();
                    let page = index.expand(&request).unwrap();
                    let slice = exact::<Slice>(&at, &page.bytes);
                    slice_holds(&slice, 97);
                    *read.entry("expand").or_default() += 1;
                    match slice.next {
                        Some(next) => after = i64::try_from(next).unwrap(),
                        None => break,
                    }
                }
            }

            let mut row_types: Vec<Option<TypeId>> = vec![None];
            row_types.extend(
                index
                    .loaded()
                    .graph
                    .ontology
                    .to_document()
                    .node_types
                    .iter()
                    .map(|t| Some(t.id)),
            );
            for row_type in row_types {
                let request = TimelineRequest::new(row_type, 3, 500, None, None).unwrap();
                let timeline = exact::<Timeline>(&at, &index.timeline(&request).unwrap().bytes);
                timeline_holds(&timeline);
                *read.entry("timeline").or_default() += 1;
                if let Some(row) = timeline.rows.first() {
                    for bucket in [BucketWidth::Day, BucketWidth::Week] {
                        let request =
                            TimelineRequest::new(row_type, 2, 1, Some(bucket), Some(row.id))
                                .unwrap();
                        let subject = index.timeline(&request).unwrap();
                        timeline_holds(&exact::<Timeline>(&at, &subject.bytes));
                        *read.entry("timeline").or_default() += 1;
                    }
                }
            }

            for (kind, since) in [
                (SinceKind::Revision, 0),
                (SinceKind::ValidTime, i64::MIN),
                (SinceKind::TransactionTime, i64::MIN),
            ] {
                let mut after = 0;
                loop {
                    let request = ChangesRequest::new(kind, since, Some(3), Some(after)).unwrap();
                    let page = index.changes(&runtime, &request).unwrap();
                    let changes = exact::<Changes>(&at, &page.bytes);
                    changes_hold(&changes);
                    *read.entry("changes").or_default() += 1;
                    *read.entry("evidence added").or_default() += changes
                        .changes
                        .iter()
                        .filter(|change| change.change == ChangeKind::EvidenceAdded)
                        .count();
                    match changes.next {
                        Some(next) => after = i64::try_from(next).unwrap(),
                        None => break,
                    }
                }
            }

            let snapshot = runtime
                .read(Some(RevisionNumber::new(revision)))
                .unwrap()
                .snapshot(None)
                .unwrap();
            let snapshot = serde_json::to_vec(&snapshot).unwrap();
            snapshot_holds(&exact::<Snapshot>(&at, &snapshot));
            *read.entry("snapshot").or_default() += 1;
        }
    }
    for (what, least) in [
        ("overview", 20),
        // `schema-changes`: two widened ends at revisions 2, 3 and 4; one modified property at
        // 3 and two at 4.
        ("widened", 6),
        ("modified", 3),
        ("search", 20),
        ("describe", 600),
        ("expand", 40),
        ("timeline", 40),
        ("changes", 60),
        ("evidence added", 2),
        ("snapshot", 20),
    ] {
        assert!(
            read.get(what).copied().unwrap_or(0) >= least,
            "{what}: {read:?}"
        );
    }
}

fn snapshot_holds(snapshot: &Snapshot) {
    let root: &Root = &snapshot.root;
    let document: &SnapshotGraphDocument = &snapshot.graph;
    assert_eq!(document.format, "ekr.graph-document/2");
    let graph: &SnapshotGraph = &document.graph;
    let graph_root: &SnapshotRoot = &graph.root;
    assert_eq!(graph_root.space, "Canonical");
    assert_eq!(root.parent.is_none(), root.revision == 0);
    assert_eq!(
        snapshot.matching_assertions.is_some(),
        snapshot.valid_at.is_some()
    );
    for (id, node) in &graph.nodes {
        let node: &SnapshotNode = node;
        assert_eq!(node.id, *id);
    }
    for (id, edge) in &graph.edges {
        let edge: &SnapshotEdge = edge;
        assert_eq!(edge.id, *id);
        assert!(graph.nodes.contains_key(&edge.source) && graph.nodes.contains_key(&edge.target));
    }
    for (id, assertion) in &graph.assertions {
        let assertion: &SnapshotAssertion = assertion;
        assert_eq!(assertion.id, *id);
        let valid: ValidTime = assertion.valid_time;
        let recorded: RecordedTime = assertion.transaction_time;
        assert!(valid
            .from
            .zip(valid.to)
            .is_none_or(|(from, to)| from.millis() <= to.millis()));
        assert!(recorded
            .recorded_to
            .is_none_or(|to| recorded.recorded_from.millis() <= to.millis()));
        assert!(assertion
            .evidence
            .iter()
            .all(|cited| graph.evidence.contains_key(cited)));
    }
    for (id, evidence) in &graph.evidence {
        let evidence: &SnapshotEvidence = evidence;
        assert_eq!(evidence.id, *id);
    }
}

/// A views format that gains a field — at its top or anywhere below — no longer reads exactly:
/// the check names the field by its pointer, so the fixture case above fails on it.
#[test]
fn a_field_added_to_a_views_format_without_an_sdk_update_fails_the_exact_read() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = fixtures::open(directory.path(), fixtures::Provider::File);
    fixtures::Fixture::named("changes").unwrap().build(&runtime);
    let index = Index::load(&runtime, None).unwrap();
    let node = *index.loaded().graph.nodes.keys().next().unwrap();
    let documents: Vec<(&str, Vec<u8>)> = vec![
        (
            "overview",
            index
                .overview(&OverviewRequest::new(None).unwrap())
                .unwrap()
                .bytes,
        ),
        (
            "search",
            index
                .search(&SearchRequest::new(String::new(), 20).unwrap())
                .unwrap()
                .bytes,
        ),
        ("describe", index.describe(node).unwrap().bytes),
        (
            "expand",
            index
                .expand(&ExpandRequest::new(vec![node], 1, 10, None, None).unwrap())
                .unwrap()
                .bytes,
        ),
        (
            "timeline",
            index
                .timeline(&TimelineRequest::new(None, 1, 10, None, None).unwrap())
                .unwrap()
                .bytes,
        ),
        (
            "changes",
            index
                .changes(
                    &runtime,
                    &ChangesRequest::new(SinceKind::Revision, 0, None, None).unwrap(),
                )
                .unwrap()
                .bytes,
        ),
    ];
    for (what, bytes) in documents {
        let document: Value = serde_json::from_slice(&bytes).unwrap();
        let reads = |document: &Value| match what {
            "overview" => exactly::<Overview>(document).map(drop),
            "search" => exactly::<NodeMatches>(document).map(drop),
            "describe" => exactly::<NodeDetail>(document).map(drop),
            "expand" => exactly::<Slice>(document).map(drop),
            "timeline" => exactly::<Timeline>(document).map(drop),
            _ => exactly::<Changes>(document).map(drop),
        };
        assert_eq!(reads(&document), Ok(()), "{what}");
        let mut top = document.clone();
        top["added_by_the_engine"] = json!(1);
        assert_eq!(
            reads(&top),
            Err(vec!["/added_by_the_engine".to_owned()]),
            "{what}"
        );
        let mut nested = document.clone();
        nested["meta"]["added_by_the_engine"] = json!("x");
        assert_eq!(
            reads(&nested),
            Err(vec!["/meta/added_by_the_engine".to_owned()]),
            "{what}"
        );
    }
}

// ---- a kind a newer ekr adds -------------------------------------------------------------------

/// `task:sdk-read-enums-tolerate-new-kinds`: a changes document whose second change is of a kind
/// this SDK does not model reads, with that change's kind as [`ChangeKind::Other`] and every
/// other change exactly as it read before; the exact read still names the drift by its pointer.
#[test]
fn a_change_kind_the_sdk_does_not_model_reads_as_other_with_every_other_change_intact() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = fixtures::open(directory.path(), fixtures::Provider::File);
    fixtures::Fixture::named("changes").unwrap().build(&runtime);
    let index = Index::load(&runtime, None).unwrap();
    let bytes = index
        .changes(
            &runtime,
            &ChangesRequest::new(SinceKind::Revision, 0, None, None).unwrap(),
        )
        .unwrap()
        .bytes;
    let document: Value = serde_json::from_slice(&bytes).unwrap();
    let known: Changes = exact("changes", &bytes);
    assert!(known.changes.len() >= 3, "{document}");

    let mut newer = document.clone();
    newer["changes"][1]["change"] = json!("PropertyReworded");
    newer["changes"][1]["reworded_to"] = json!("a field only that kind carries");
    let read: Changes = serde_json::from_value(newer.clone())
        .unwrap_or_else(|error| panic!("an unknown change kind fails the whole read: {error}"));
    assert_eq!(read.changes[1].change, ChangeKind::Other);
    assert_eq!(read.changes.len(), known.changes.len());
    for (at, (read, known)) in read.changes.iter().zip(&known.changes).enumerate() {
        if at != 1 {
            assert_eq!(read, known, "change {at}");
        }
    }
    assert_eq!(
        exactly::<Changes>(&newer).map(drop),
        Err(vec![
            "/changes/1/change".to_owned(),
            "/changes/1/reworded_to".to_owned()
        ])
    );
}

/// Every other closed enum of the read models reads a kind a newer `ekr` adds as its `Other`,
/// and the exact read reports the drift at that kind's pointer.
#[test]
fn every_closed_read_enum_reads_a_kind_a_newer_ekr_adds_as_other() {
    fn other<T>(what: &str, document: Value, expected: &T, pointer: &str)
    where
        T: DeserializeOwned + Serialize + PartialEq + std::fmt::Debug,
    {
        let read: T = serde_json::from_value(document.clone())
            .unwrap_or_else(|error| panic!("{what}: a new kind fails the read: {error}"));
        assert_eq!(&read, expected, "{what}");
        let drift = exactly::<T>(&document).map(drop).unwrap_err();
        assert!(
            drift.iter().any(|at| at == pointer),
            "{what}: {drift:?} does not name {pointer}"
        );
    }
    other("a match tier", json!("Phonetic"), &MatchTier::Other, "");
    other("a match field", json!("Summary"), &MatchField::Other, "");
    other(
        "a transaction state",
        json!("Quarantined"),
        &TransactionState::Other,
        "",
    );
    other(
        "a view value",
        json!({"kind": "Geo", "value": [52.5, 13.4]}),
        &ViewValue::Other,
        "/kind",
    );
    other(
        "an ontology value type",
        json!({"value_kind": "Geo", "parameters": {"datum": "WGS84"}}),
        &OntologyValueType::Other,
        "/value_kind",
    );
    other(
        "an explanation link",
        json!({"kind": "Attestation", "by": OPERATOR}),
        &ExplanationLink::Other,
        "/kind",
    );
    other(
        "an ontology cardinality",
        json!("AtLeastOne"),
        &OntologyCardinality::Other,
        "",
    );
    other(
        "a snapshot subject",
        json!({"Assertion": SEEDED_ASSERTION}),
        &SnapshotSubject::Other,
        "",
    );
    other(
        "a snapshot predicate",
        json!({"Qualifier": ORGANIZATION}),
        &SnapshotPredicate::Other,
        "",
    );
    assert_eq!(TransactionState::Other.as_str(), "Other");
    // A known externally tagged kind whose id is wrong is still refused.
    assert!(serde_json::from_value::<SnapshotSubject>(json!({"Node": 5})).is_err());
    assert!(serde_json::from_value::<SnapshotPredicate>(json!("Property")).is_err());
    assert_eq!(
        serde_json::from_value::<SnapshotSubject>(json!({"Node": ALICE})).unwrap(),
        SnapshotSubject::Node(ALICE.parse().unwrap())
    );

    // A new kind nested in a known one is the known one holding `Other`; a known kind whose
    // content is wrong is still refused, not read as `Other`.
    assert_eq!(
        serde_json::from_value::<ViewValue>(
            json!({"kind": "List", "value": [{"kind": "Geo", "value": 1}]})
        )
        .unwrap(),
        ViewValue::List(vec![ViewValue::Other])
    );
    assert_eq!(
        serde_json::from_value::<OntologyValueType>(
            json!({"value_kind": "List", "parameters": {"value_kind": "Geo", "parameters": 1}})
        )
        .unwrap(),
        OntologyValueType::List(Box::new(OntologyValueType::Other))
    );
    assert!(
        serde_json::from_value::<ViewValue>(json!({"kind": "Integer", "value": "seven"})).is_err()
    );
    assert!(serde_json::from_value::<OntologyValueType>(
        json!({"value_kind": "NodeRef", "parameters": {"allowed_types": 7}})
    )
    .is_err());
}

// ---- through a real session --------------------------------------------------------------------

/// A host document for the views fixtures' tenant, context and anchor.
fn fixture_host(path: &Path) {
    let host = json!({
        "format": "ekr.cli-host/1",
        "tenant": "views",
        "context": fixtures::context(),
        "authority": fixtures::anchor(),
    });
    std::fs::write(path, serde_json::to_vec_pretty(&host).unwrap()).unwrap();
}

/// The iterator pages 5,000 nodes and 5,000 edges — every node a seed at depth 0 — in pages of
/// at most 700 nodes and 900 edges: every node and every edge exactly once, each edge after both
/// its ends, every page of the revision the first page read.
#[test]
fn the_expand_iterator_pages_a_5000_node_fixture_with_nothing_lost_or_repeated() {
    const NODES: u64 = 5_000;
    let directory = tempfile::tempdir().unwrap();
    let store = directory.path().join("store");
    let runtime = fixtures::open(&store, fixtures::Provider::File);
    fixtures::build_large(&runtime, NODES, 5_000);
    drop(runtime);
    fixture_host(&directory.path().join("host.json"));
    let config = StoreConfig {
        host: directory.path().join("host.json"),
        store,
        backend: Backend::File,
    };
    let mut session = ProcessSession::start(&binary(), config, SessionOptions::default()).unwrap();
    let mut reader = Reader::new(&mut session);
    let overview = reader.overview(None, Some(1)).unwrap();
    assert_eq!(overview.meta.node_count, NODES);
    let edge_count = overview.meta.edge_count;

    let seeds: Vec<NodeId> = (0..NODES)
        .map(|n| fixtures::id::<NodeId>(0x60_0000_0000 + n))
        .collect();
    let mut query = ExpandQuery::new(seeds.clone(), 0, 700);
    query.edges = Some(900);
    let pages: ExpandPages<'_, &mut ProcessSession> = reader.expand(query);
    let (mut nodes, mut edges) = (BTreeSet::new(), BTreeSet::new());
    let (mut revisions, mut count) = (BTreeSet::new(), 0);
    for page in pages {
        let page: Slice = page.unwrap();
        count += 1;
        assert!(page.nodes.len() <= 700 && page.edges.len() <= 900);
        assert_eq!(page.meta.node_total, NODES);
        assert_eq!(page.meta.edge_total, edge_count);
        revisions.insert(page.meta.revision);
        for node in &page.nodes {
            assert!(nodes.insert(node.id), "{} repeated", node.id);
        }
        for edge in &page.edges {
            assert!(edges.insert(edge.id), "{} repeated", edge.id);
            assert!(nodes.contains(&edge.source) && nodes.contains(&edge.target));
        }
    }
    assert!(count >= 8, "{count} pages");
    assert_eq!(nodes, seeds.into_iter().collect::<BTreeSet<NodeId>>());
    assert_eq!(edges.len() as u64, edge_count);
    assert_eq!(revisions.len(), 1);
    let _: EdgeId = *edges.iter().next().unwrap();
}

/// Session A's reader answers, on its next call after session B committed, from B's commit: the
/// head, the overview, a search, the node's detail and the changes since revision 0.
#[test]
fn a_reader_in_one_session_sees_another_sessions_commits_on_its_next_call() {
    let world = World::new();
    let mut first = world.seeded();
    let mut reader = Reader::new(&mut first);
    let before = reader.overview(None, None).unwrap();
    assert_eq!(before.meta.revision, 0);
    assert!(reader
        .search("Globex", None, None)
        .unwrap()
        .matches
        .is_empty());
    let since = reader
        .changes(Since::Revision(0), None, None, None)
        .unwrap();
    assert!(since.changes.is_empty());

    let mut second = world.session();
    World::commit_globex(&mut second);
    drop(second);

    let node: NodeId = GLOBEX.1.parse().unwrap();
    assert_eq!(reader.head().unwrap().revision, 1);
    let after = reader.overview(None, None).unwrap();
    assert_eq!(after.meta.revision, 1);
    assert_eq!(after.meta.node_count, before.meta.node_count + 1);
    let found = reader.search("Globex", None, None).unwrap();
    assert_eq!(
        found.matches.iter().map(|m| m.id).collect::<Vec<_>>(),
        [node]
    );
    assert_eq!(reader.describe(node, None).unwrap().node.name, "Globex");
    let since = reader
        .changes(Since::Revision(0), None, None, None)
        .unwrap();
    assert_eq!(since.changes.len(), 1);
    assert_eq!(since.changes[0].change, ChangeKind::NodeCreated);
    assert_eq!(since.changes[0].id, GLOBEX.1);
    assert!(matches!(
        reader.describe(node, Some(0)),
        Err(ReadError::Refused { ref refusal, .. }) if refusal.code == "ekr.views.NodeNotFound"
    ));
}

/// Every views read of a real session is the document the session printed, read exactly; a
/// refusal is typed by its name.
#[test]
fn every_views_read_of_a_real_session_is_the_document_it_printed() {
    let world = World::new();
    let mut session = world.seeded();
    World::commit_globex(&mut session);
    let alice: NodeId = ALICE.parse().unwrap();
    let person: TypeId = "00000000-0000-4000-8000-000000000201".parse().unwrap();
    let mut reader = Reader::new(RecordingTransport::record(&mut session));
    overview_holds(&reader.overview(Some(0), Some(2)).unwrap());
    matches_hold(&reader.search("-", Some(5), None).unwrap(), 5);
    matches_hold(&reader.search("", Some(5), Some(0)).unwrap(), 5);
    detail_holds(&reader.describe(alice, None).unwrap(), alice);
    let page = reader
        .expand_page(&ExpandQuery::new(vec![alice], 1, 10), 0)
        .unwrap();
    slice_holds(&page, 10);
    let mut timeline = TimelineQuery::new(2, 5);
    timeline.row_type = Some(person);
    timeline.bucket = Some(Bucket::Week);
    timeline.subject = Some(alice);
    timeline.revision = Some(1);
    timeline_holds(&reader.timeline(&timeline).unwrap());
    timeline_holds(&reader.timeline(&TimelineQuery::new(1, 10)).unwrap());
    for since in [
        Since::Revision(0),
        Since::Valid(Timestamp::from_millis(0)),
        Since::Recorded(Timestamp::from_millis(-1)),
    ] {
        changes_hold(&reader.changes(since, Some(1), Some(1), Some(0)).unwrap());
    }
    let refused = reader.overview(Some(9), None).unwrap_err();
    assert!(
        matches!(&refused, ReadError::Refused { verb, refusal }
            if verb == "overview" && refusal.code == "ekr.views.RevisionNotFound"),
        "{refused}"
    );

    let recording = reader.into_inner().into_recording();
    assert_eq!(recording.exchanges.len(), 11);
    for exchange in &recording.exchanges {
        let what = format!("{:?}", exchange.request.argv);
        let Some(document) = &exchange.reply.document else {
            assert_eq!(exchange.reply.exit, 2, "{what}");
            continue;
        };
        let drift = match exchange.request.verb() {
            "overview" => exactly::<Overview>(document).map(drop),
            "search" => exactly::<NodeMatches>(document).map(drop),
            "describe" => exactly::<NodeDetail>(document).map(drop),
            "expand" => exactly::<Slice>(document).map(drop),
            "timeline" => exactly::<Timeline>(document).map(drop),
            "changes" => exactly::<Changes>(document).map(drop),
            other => panic!("the reader sent {other}"),
        };
        assert_eq!(drift, Ok(()), "{what}");
    }
}

/// `head`, `ontology`, `snapshot`, `transactions` and `explain` read one typed value through a
/// session and as one-shot `ekr` processes, and it is exactly what `ekr` printed.
#[test]
fn the_five_kernel_reads_are_one_typed_value_through_a_session_and_one_shot() {
    let world = World::new();
    let mut session = world.seeded();
    World::commit_globex(&mut session);
    let assertion: AssertionId = SEEDED_ASSERTION.parse().unwrap();
    let mut one_shot = OneShotReader::new(&binary(), world.store(), world.options());
    let mut reader = Reader::new(&mut session);

    let head: Head = reader.head().unwrap();
    assert_eq!(head, one_shot.head().unwrap());
    assert_eq!(head.revision, 1);
    assert_eq!(
        exactly::<Head>(&world.one_shot(&["head"])),
        Ok(head.clone())
    );

    for at in [None, Some(0)] {
        let ontology: Ontology = reader.ontology(at).unwrap();
        assert_eq!(ontology, one_shot.ontology(at).unwrap());
        let mut verb = vec!["ontology"];
        let revision = at.map(|at: u64| at.to_string());
        if let Some(revision) = &revision {
            verb.extend(["--at", revision]);
        }
        assert_eq!(
            exactly::<Ontology>(&world.one_shot(&verb)),
            Ok(ontology.clone())
        );
        let node_types: &[OntologyNodeType] = &ontology.node_types;
        assert!(node_types.iter().any(|t| t.name == "Organization"));
        for property in node_types.iter().flat_map(|t| &t.properties) {
            let property: &OntologyProperty = property;
            let kind: &OntologyValueType = &property.value_type;
            assert!(!property.name.is_empty(), "{kind:?}");
        }
        let edge_types: &[OntologyEdgeType] = &ontology.edge_types;
        for end in edge_types.iter().flat_map(|t| &t.source_types) {
            let end: &NamedType = end;
            assert!(end.name.is_some());
        }
    }

    let valid_at = Timestamp::from_millis(1_600_000_000_000);
    for (at, valid) in [(None, None), (Some(0), Some(valid_at))] {
        let snapshot: Snapshot = reader.snapshot(at, valid).unwrap();
        assert_eq!(snapshot, one_shot.snapshot(at, valid).unwrap());
        snapshot_holds(&snapshot);
        let mut verb = vec!["snapshot".to_owned()];
        if let Some(at) = at {
            verb.extend(["--at".to_owned(), at.to_string()]);
        }
        if let Some(valid) = valid {
            verb.extend(["--valid-at".to_owned(), valid.millis().to_string()]);
        }
        let verb: Vec<&str> = verb.iter().map(String::as_str).collect();
        assert_eq!(
            exactly::<Snapshot>(&world.one_shot(&verb)),
            Ok(snapshot.clone())
        );
        assert_eq!(snapshot.valid_at, valid);
    }

    for state in [
        None,
        Some(TransactionState::Committed),
        Some(TransactionState::Proposed),
    ] {
        let transactions: Transactions = reader.transactions(state).unwrap();
        assert_eq!(transactions, one_shot.transactions(state).unwrap());
        let listed: &[ListedTransaction] = &transactions.0;
        assert_eq!(
            listed.len(),
            usize::from(state != Some(TransactionState::Proposed))
        );
    }
    assert_eq!(
        exactly::<Transactions>(&world.one_shot(&["transactions"])),
        Ok(reader.transactions(None).unwrap())
    );

    let explanation: Explanation = reader.explain(assertion).unwrap();
    assert_eq!(explanation, one_shot.explain(assertion).unwrap());
    assert_eq!(
        exactly::<Explanation>(&world.one_shot(&["explain", SEEDED_ASSERTION])),
        Ok(explanation.clone())
    );
    assert_eq!(explanation.format, "ekr.explanation/2");
    let documents: Explanation = reader.explain_documents(assertion).unwrap();
    assert_eq!(documents, one_shot.explain_documents(assertion).unwrap());
    assert_eq!(
        exactly::<Explanation>(&world.one_shot(&["explain", SEEDED_ASSERTION, "--documents"])),
        Ok(documents.clone())
    );
    let evidence = |explanation: &Explanation| -> Vec<ExplainedEvidence> {
        explanation
            .links
            .iter()
            .filter_map(|link| match link {
                ExplanationLink::Evidence(evidence) => Some(evidence.clone()),
                _ => None,
            })
            .collect()
    };
    // By reference, the evidence link is the entry alone; with the documents, its bytes too.
    let referenced = evidence(&explanation);
    assert_eq!(referenced.len(), 1);
    assert_eq!(
        (&referenced[0].payload, &referenced[0].text),
        (&None, &None)
    );
    let whole = evidence(&documents);
    assert_eq!(whole.len(), 1);
    assert_eq!(whole[0].evidence, referenced[0].evidence);
    assert!(whole[0].payload.is_some());
    assert_eq!(whole[0].text.as_deref(), Some("Alice is CEO of Acme."));
    assert!(matches!(
        explanation.links.first(),
        Some(ExplanationLink::Assertion(first)) if first.id == assertion
    ));

    let unknown: AssertionId = "00000000-0000-4000-8000-000000000999".parse().unwrap();
    for refused in [reader.explain(unknown), one_shot.explain(unknown)] {
        assert!(
            matches!(&refused, Err(ReadError::Refused { refusal, .. })
                if refusal.code == "ekr.kernel.AssertionNotFound"),
            "{refused:?}"
        );
    }
}
