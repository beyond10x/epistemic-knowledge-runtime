//! Adversary, `task:proposed-node-carries-reference-aliases`, pass 1.
//!
//! Two consumers act on one `ProposeNew` outcome at the same head. One commits; the other's commit
//! is `Stale`, and it follows the recovery `docs/cli.md` § `ekr commit` names verbatim: "propose
//! the transaction again under a new id and validate it against the new head". Before correction
//! 1 nothing refused the second `CreateNode` of the same type with the same aliases, so the
//! reference that resolved at the first commit was `Ambiguous` from then on, and no operation can
//! change an alias to repair it. Now the recovery is `Rejected` with `alias-already-exists`.
//!
//! The task's context names the duplicate node for one real-world entity as what this work exists
//! to prevent; this case asserts that the reference still resolves to one node after the
//! documented workflow has run.

use std::path::PathBuf;
use std::process::Output;

use ekr::host::CliHostConfigurationV1;
use ekr_graph::GraphSnapshot;
use ekr_integrate::{ResolutionOutcome, TypedReference};
use ekr_kernel::Runtime;
use serde_json::Value;

const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";

fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

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

    fn propose(&self, name: &str, document: &str) -> String {
        let path = self.file(name, document);
        self.ok(&["propose", &path])["transaction_id"]
            .as_str()
            .unwrap()
            .to_owned()
    }
}

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
fn two_consumers_of_one_propose_new_following_the_stale_recovery_leave_the_reference_resolved() {
    let host: Value = serde_json::from_str(&text(&["example", "ekr.cli-host/1"])).unwrap();
    let operator = host["context"]["operator"].as_str().unwrap().to_owned();
    let reference = TypedReference {
        type_id: ORGANIZATION.parse().unwrap(),
        aliases: vec!["Globex".to_owned()],
    };
    let world = World::seeded("file");
    let ResolutionOutcome::ProposeNew(proposed) = world.resolve(&reference) else {
        panic!("the example seed holds no Globex");
    };

    let first = world.propose(
        "first.yaml",
        &create_node(
            &minted("node"),
            &proposed.aliases,
            &operator,
            &minted("transaction"),
        ),
    );
    let second_node = minted("node");
    let second = world.propose(
        "second.yaml",
        &create_node(
            &second_node,
            &proposed.aliases,
            &operator,
            &minted("transaction"),
        ),
    );
    assert_eq!(world.ok(&["validate", &first])["kind"], "Validated");
    assert_eq!(world.ok(&["validate", &second])["kind"], "Validated");
    assert_eq!(world.ok(&["commit", &first])["kind"], "Committed");
    assert_eq!(world.ok(&["commit", &second])["kind"], "Stale");
    assert!(
        matches!(world.resolve(&reference), ResolutionOutcome::Resolved(_)),
        "after the first commit the reference resolves"
    );

    // docs/cli.md § ekr commit: "propose the transaction again under a new id and validate it
    // against the new head".
    let again = world.propose(
        "again.yaml",
        &create_node(
            &second_node,
            &proposed.aliases,
            &operator,
            &minted("transaction"),
        ),
    );
    // Correction 1: the recovery is refused, naming the alias the first node already holds, and
    // the reference still resolves to that one node.
    let validated = world.ok(&["validate", &again]);
    assert_eq!(validated["kind"], "Rejected", "{validated}");
    assert!(
        validated.to_string().contains("alias-already-exists"),
        "the refusal names the held alias: {validated}"
    );
    let outcome = world.resolve(&reference);
    assert!(
        matches!(outcome, ResolutionOutcome::Resolved(_)),
        "after the refused recovery the reference still resolves to one node: {outcome:?}"
    );
}
