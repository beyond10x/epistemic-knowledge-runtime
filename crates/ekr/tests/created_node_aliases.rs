//! `task:proposed-node-carries-reference-aliases`: a node committed from a `ProposeNew` outcome
//! carries the reference's aliases, and resolving the same reference against the next revision
//! returns `Resolved` with that node.
//!
//! Seed, propose, validate, commit and snapshot are fresh `ekr` processes on both providers, under
//! the example host and seed. The resolution is `ekr_integrate::resolve` over the verified
//! snapshot a fresh [`Runtime`] reads from the same store under the same host: the call `ekr
//! resolve` makes, which is not on this branch yet.

use std::path::PathBuf;
use std::process::Output;

use ekr::host::CliHostConfigurationV1;
use ekr_core::NodeId;
use ekr_graph::GraphSnapshot;
use ekr_integrate::{ResolutionOutcome, ResolvedReference, TypedReference};
use ekr_kernel::Runtime;
use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

/// Exit 0 and stdout as text.
fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// One provider in its own directory, seeded from `ekr example ekr-seed/2`.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn seeded(backend: &'static str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        world.file("host.json", &text(&["example", "ekr.cli-host/1"]));
        let seed = world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        assert_eq!(world.ok(&["seed", &seed])["result"]["revision"], 0);
        world
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn file(&self, name: &str, contents: &str) -> String {
        let path = self.directory.path().join(name);
        std::fs::write(&path, contents).unwrap();
        path.display().to_string()
    }

    fn run(&self, verb: &[&str]) -> Output {
        ekr()
            .arg("--host")
            .arg(self.directory.path().join("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(verb)
            .output()
            .unwrap()
    }

    /// Exit 0 and one JSON result on stdout.
    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// `reference` resolved against the head a fresh runtime verifies from the store.
    fn resolve(&self, reference: &TypedReference) -> ResolutionOutcome {
        let host = CliHostConfigurationV1::from_json(
            &std::fs::read(self.directory.path().join("host.json")).unwrap(),
        )
        .unwrap();
        let runtime = match self.backend {
            "file" => Runtime::file(&self.store(), &host.tenant, host.context, host.authority),
            _ => Runtime::sqlite(&self.store(), &host.tenant, host.context, host.authority),
        }
        .unwrap();
        let graph = runtime.snapshot().unwrap();
        ekr_integrate::resolve(GraphSnapshot::of(&graph), reference)
    }
}

/// A `CreateNode` of `node` with `aliases`, as the one operation of an operator's transaction.
fn create_node(node: &str, aliases: &[String], operator: &str, transaction: &str) -> String {
    let aliases = serde_json::to_string(aliases).unwrap();
    format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {transaction}\n  proposer: \
         {operator}\n  operations:\n  - !CreateNode\n    id: {node}\n    root_id: {ROOT}\n    \
         type_id: {ORGANIZATION}\n    canonical_name: Globex\n    properties: {{}}\n    \
         aliases: {aliases}\n  evidence: []\n"
    )
}

fn minted(kind: &str) -> String {
    let minted: Value = serde_json::from_str(&text(&["mint", kind])).unwrap();
    minted["id"].as_str().unwrap().to_owned()
}

#[test]
fn a_node_created_from_propose_new_carries_its_aliases_and_resolves_at_the_next_revision() {
    let host: Value = serde_json::from_str(&text(&["example", "ekr.cli-host/1"])).unwrap();
    let operator = host["context"]["operator"].as_str().unwrap().to_owned();
    let reference = TypedReference {
        type_id: ORGANIZATION.parse().unwrap(),
        aliases: vec!["Globex Corporation".to_owned(), "Globex".to_owned()],
    };
    for backend in BACKENDS {
        let world = World::seeded(backend);
        let ResolutionOutcome::ProposeNew(proposed) = world.resolve(&reference) else {
            panic!("{backend}: the seed holds no Globex, so the reference proposes a new node");
        };
        assert_eq!(
            proposed.aliases,
            ["Globex", "Globex Corporation"],
            "{backend}"
        );

        let node = minted("node");
        let document = world.file(
            "create.yaml",
            &create_node(&node, &proposed.aliases, &operator, &minted("transaction")),
        );
        let id = world.ok(&["propose", &document])["transaction_id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(
            world.ok(&["validate", &id])["kind"],
            "Validated",
            "{backend}"
        );
        let committed = world.ok(&["commit", &id]);
        assert_eq!(committed["kind"], "Committed", "{backend}: {committed}");
        assert_eq!(committed["result"]["revision"], 1, "{backend}");

        let snapshot = world.ok(&["snapshot"]);
        assert_eq!(
            snapshot["graph"]["graph"]["nodes"][&node]["aliases"],
            serde_json::json!(["Globex", "Globex Corporation"]),
            "{backend}: the committed node carries the reference's aliases: {snapshot}"
        );

        let node_id: NodeId = node.parse().unwrap();
        assert_eq!(
            world.resolve(&reference),
            ResolutionOutcome::Resolved(ResolvedReference { node_id }),
            "{backend}: the same reference, against revision 1"
        );
        for alias in &proposed.aliases {
            let one = TypedReference {
                type_id: reference.type_id,
                aliases: vec![alias.clone()],
            };
            assert_eq!(
                world.resolve(&one),
                ResolutionOutcome::Resolved(ResolvedReference { node_id }),
                "{backend}: {alias} alone"
            );
        }
    }
}

#[test]
fn a_create_node_without_aliases_still_commits_under_both_formats_and_carries_none() {
    let host: Value = serde_json::from_str(&text(&["example", "ekr.cli-host/1"])).unwrap();
    let operator = host["context"]["operator"].as_str().unwrap().to_owned();
    for backend in BACKENDS {
        for format in ["ekr.transaction-document/1", "ekr.transaction-document/2"] {
            let world = World::seeded(backend);
            let node = minted("node");
            let document = create_node(&node, &[], &operator, &minted("transaction"))
                .replace("    aliases: []\n", "")
                .replacen("ekr.transaction-document/2", format, 1);
            assert!(!document.contains("aliases"), "{document}");
            let path = world.file("create.yaml", &document);
            let id = world.ok(&["propose", &path])["transaction_id"]
                .as_str()
                .unwrap()
                .to_owned();
            assert_eq!(world.ok(&["validate", &id])["kind"], "Validated");
            assert_eq!(world.ok(&["commit", &id])["kind"], "Committed");
            let snapshot = world.ok(&["snapshot"]);
            assert_eq!(
                snapshot["graph"]["graph"]["nodes"][&node]["aliases"],
                serde_json::json!([]),
                "{backend} {format}: {snapshot}"
            );
        }
    }
}
