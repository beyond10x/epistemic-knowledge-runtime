//! Adversary pass on `story:sdk-read-helpers`: the SDK's typed reads against what `docs/sdk.md`
//! promises of them.
//!
//! * `ExpandPages` pins every page after the first to the first page's revision. The suite's own
//!   5,000-node case commits nothing between pages, so it stays green with the pin removed; the
//!   case here commits between pages.
//! * "A reader ignores a field it does not know, so a newer `ekr` does not break an older
//!   consumer." `Snapshot` and `Ontology` reuse SDK document types that deny unknown fields.
//!
//! Every case that runs `ekr` drives the binary built from this checkout, by path.

#![cfg(unix)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ekr_core::NodeId;
use ekr_sdk::binary::EkrBinary;
use ekr_sdk::read::{ExpandQuery, Ontology, Reader, Snapshot};
use ekr_sdk::reply::Reply;
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{Request, Transport, TransportError};
use serde_json::Value;

const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const ALICE: &str = "00000000-0000-4000-8000-000000000301";

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

/// A file store seeded from the example seed, under the example host.
struct World {
    directory: tempfile::TempDir,
}

impl World {
    fn new() -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
        };
        let host = world.run(&["example", "ekr.cli-host/1"], false);
        std::fs::write(world.directory.path().join("host.json"), host).unwrap();
        let seed = world.run(&["example", "ekr-seed/2"], false);
        std::fs::write(world.directory.path().join("seed.yaml"), seed).unwrap();
        world.run(&["seed", "seed.yaml"], true);
        world
    }

    fn store(&self) -> StoreConfig {
        StoreConfig {
            host: self.directory.path().join("host.json"),
            store: self.directory.path().join("store"),
            backend: Backend::File,
        }
    }

    /// `ekr [store options] <args>`, exit 0: its stdout.
    fn run(&self, args: &[&str], on_store: bool) -> Vec<u8> {
        let mut command = std::process::Command::new(ekr_path());
        command.env_clear().current_dir(self.directory.path());
        if on_store {
            let store = self.store();
            command
                .arg("--host")
                .arg(&store.host)
                .arg("--store")
                .arg(&store.store)
                .args(["--backend", "file"]);
        }
        let output = command.args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    /// Commits, from another process, a new unconnected Organization `name`: one revision more.
    fn commit_elsewhere(&self, node: u64, transaction: u64, name: &str) {
        let document = format!(
            "format: ekr.transaction-document/2\ntransaction:\n  id: \
             00000000-0000-4000-8000-{transaction:012}\n  proposer: {OPERATOR}\n  operations:\n  \
             - !CreateNode\n    id: 00000000-0000-4000-8000-{node:012}\n    root_id: {ROOT}\n    \
             type_id: {ORGANIZATION}\n    canonical_name: {name}\n    properties: {{}}\n    \
             aliases: []\n  evidence: []\n"
        );
        let file = format!("tx-{transaction}.yaml");
        std::fs::write(self.directory.path().join(&file), document).unwrap();
        let id = format!("00000000-0000-4000-8000-{transaction:012}");
        self.run(&["propose", &file], true);
        self.run(&["validate", &id], true);
        self.run(&["commit", &id], true);
    }

    fn session(&self) -> ProcessSession {
        let binary = EkrBinary::open(ekr_path()).unwrap();
        let options = SessionOptions {
            current_dir: Some(self.directory.path().to_path_buf()),
            ..SessionOptions::default()
        };
        ProcessSession::start(&binary, self.store(), options).unwrap()
    }
}

/// A transport that, after the first `expand` it passes on, lets another process commit.
struct CommitAfterFirstPage<'w> {
    inner: ProcessSession,
    world: &'w World,
    commit: Option<(u64, u64, &'static str)>,
}

impl Transport for CommitAfterFirstPage<'_> {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        let reply = self.inner.request(request)?;
        if request.verb() == "expand" {
            if let Some((node, transaction, name)) = self.commit.take() {
                self.world.commit_elsewhere(node, transaction, name);
            }
        }
        Ok(reply)
    }
}

/// `docs/sdk.md`: "Every page after the first reads the revision that the first page read, so a
/// commit between two pages neither drops nor repeats a node or an edge." Paged by hand with no
/// revision, the second page reads the commit made between the pages — so the scenario
/// discriminates; paged by `ExpandPages`, every page reads the first page's revision.
///
/// Green on this tree. With `self.query.revision = Some(slice.meta.revision)` removed from
/// `ExpandPages::next` (`crates/ekr-sdk/src/read/mod.rs`), the second half fails; the suite's own
/// `the_expand_iterator_pages_a_5000_node_fixture_with_nothing_lost_or_repeated` does not, since
/// nothing commits between its pages.
#[test]
fn expand_pages_read_the_first_pages_revision_after_a_commit_between_pages() {
    let world = World::new();
    let alice: NodeId = ALICE.parse().unwrap();
    let query = ExpandQuery::new(vec![alice], 1, 1);

    let mut unpinned = Reader::new(CommitAfterFirstPage {
        inner: world.session(),
        world: &world,
        commit: Some((0x903, 0x904, "Initech")),
    });
    let first = unpinned.expand_page(&query, 0).unwrap();
    let after = first
        .next
        .expect("Alice's neighbourhood takes two pages of one node");
    let second = unpinned.expand_page(&query, after).unwrap();
    assert_eq!(
        (first.meta.revision, second.meta.revision),
        (0, 1),
        "an unpinned second page reads the commit made after the first"
    );

    let mut pinned = Reader::new(CommitAfterFirstPage {
        inner: world.session(),
        world: &world,
        commit: Some((0x905, 0x906, "Umbrella")),
    });
    let revisions: Vec<u64> = pinned
        .expand(query)
        .map(|page| page.unwrap().meta.revision)
        .collect();
    assert!(revisions.len() >= 2, "{revisions:?}");
    assert!(
        revisions.iter().all(|revision| *revision == 1),
        "every page reads the first page's revision 1, not the revision 2 committed between \
         pages: {revisions:?}"
    );
}

/// Inserts `"added_by_a_newer_ekr": 1` into the object at `pointer`.
fn with_field(document: &Value, pointer: &str) -> Value {
    let mut document = document.clone();
    document
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("{pointer} is in the document"))
        .as_object_mut()
        .unwrap_or_else(|| panic!("{pointer} is an object"))
        .insert("added_by_a_newer_ekr".to_owned(), Value::from(1));
    document
}

/// `docs/sdk.md` § Typed reads: "A reader ignores a field it does not know, so a newer `ekr`
/// does not break an older consumer." That holds at the top of a `Snapshot` and an `Ontology`,
/// and fails below: `Snapshot` reads the graph root, an assertion's `valid_time` and
/// `transaction_time`, and `Ontology` a property definition and its value type, through
/// `ekr_sdk::document` types that are `#[serde(deny_unknown_fields)]`
/// (`crates/ekr-sdk/src/document/graph.rs` `TemporalRange`, `TransactionTime`;
/// `document/seed.rs` `GraphRoot`; `document/value.rs` `PropertyDefinition`, `ValueType`).
#[test]
#[ignore = "defect: Snapshot and Ontology refuse an unknown field inside nested records, against docs/sdk.md's forward-compatibility promise"]
fn a_snapshot_and_an_ontology_read_with_a_field_a_newer_ekr_added_anywhere() {
    let world = World::new();
    let snapshot: Value = serde_json::from_slice(&world.run(&["snapshot"], true)).unwrap();
    let ontology: Value = serde_json::from_slice(&world.run(&["ontology"], true)).unwrap();
    let assertion = snapshot["graph"]["graph"]["assertions"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap()
        .clone();
    let assertions = format!("/graph/graph/assertions/{assertion}");
    let organization = ontology["node_types"]
        .as_array()
        .unwrap()
        .iter()
        .position(|node_type| node_type["name"] == "Organization")
        .unwrap();

    let mut refused = Vec::new();
    for pointer in [
        String::new(),
        "/root".to_owned(),
        "/graph/graph/root".to_owned(),
        format!("{assertions}/valid_time"),
        format!("{assertions}/transaction_time"),
    ] {
        if let Err(error) = serde_json::from_value::<Snapshot>(with_field(&snapshot, &pointer)) {
            refused.push(format!("Snapshot {pointer:?}: {error}"));
        }
    }
    for pointer in [
        String::new(),
        format!("/node_types/{organization}/properties/0"),
        format!("/node_types/{organization}/properties/0/value_type"),
    ] {
        if let Err(error) = serde_json::from_value::<Ontology>(with_field(&ontology, &pointer)) {
            refused.push(format!("Ontology {pointer:?}: {error}"));
        }
    }
    assert!(
        refused.is_empty(),
        "a field a newer ekr adds breaks the read:\n{}",
        refused.join("\n")
    );
}
