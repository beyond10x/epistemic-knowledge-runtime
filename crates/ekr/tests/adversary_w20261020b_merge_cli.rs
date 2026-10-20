//! Adversary pass 1, wave 2026-10-20b, `story:merge-entity-applies`: the `ekr` binary end to end,
//! on both providers, over the example seed (`ekr example ekr-seed/2`). Every write goes through
//! `ekr propose`, `ekr validate` and `ekr commit` of a YAML transaction document.
//!
//! `ekr compare` answers "whether two stores hold the same knowledge, ignoring commit times and
//! transaction ids" (`docs/cli.md`, `crates/ekr/src/cli/compare.rs:1-7`): each entry is "the JSON of
//! the graph's own record without the fields a commit's clock or its transaction decides". A
//! supersession record gained a seventh field, `merge` (design § 111.2, ADR 0018 decision 1: an
//! absorbed node "is a node held in a record that carries merge lineage"), which neither a clock
//! nor a transaction id decides; `compare.rs:209-242` still lists the six fields of § 110.

use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const PERSON: &str = "00000000-0000-4000-8000-000000000201";
const ALICE: &str = "00000000-0000-4000-8000-000000000301";
const ALICE_EVIDENCE: &str = "00000000-0000-4000-8000-000000000401";
/// A Person recorded twice, known as `dup`.
const DUPLICATE: &str = "00000000-0000-4000-8000-00000000bd11";

fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in [
        "EKR_HOST",
        "EKR_STORE",
        "EKR_BACKEND",
        "EKR_FULL_REPLAY",
        "EKR_STAGE",
    ] {
        command.env_remove(var);
    }
    command
}

fn printed(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
    store: &'static str,
}

impl World {
    fn seeded(directory: tempfile::TempDir, backend: &'static str, store: &'static str) -> Self {
        let world = Self {
            directory,
            backend,
            store,
        };
        std::fs::write(
            world.file("host.json"),
            printed(&["example", "ekr.cli-host/1"]),
        )
        .unwrap();
        std::fs::write(world.file("seed.yaml"), printed(&["example", "ekr-seed/2"])).unwrap();
        let seed = world.file("seed.yaml");
        world.ok(&["seed", seed.to_str().unwrap()]);
        world
    }

    fn file(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn ok(&self, args: &[&str]) -> Value {
        let output = ekr()
            .arg("--host")
            .arg(self.file("host.json"))
            .arg("--store")
            .arg(self.file(self.store))
            .args(["--backend", self.backend])
            .args(args)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {args:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// Proposes, validates and commits the transaction document `yaml`, whose id is `id`.
    fn commit(&self, name: &str, id: &str, yaml: &str) {
        let path = self.file(name);
        std::fs::write(&path, yaml).unwrap();
        self.ok(&["propose", path.to_str().unwrap()]);
        let validated = self.ok(&["validate", id]);
        assert_eq!(
            validated["kind"], "Validated",
            "{}: {validated}",
            self.backend
        );
        let committed = self.ok(&["commit", id]);
        assert_eq!(
            committed["kind"], "Committed",
            "{}: {committed}",
            self.backend
        );
    }
}

/// The transaction document `id` holding `operations`, resting on `evidence`.
fn document(id: &str, operations: &str, evidence: &[&str]) -> String {
    let listed = if evidence.is_empty() {
        " []".to_owned()
    } else {
        evidence
            .iter()
            .map(|id| format!("\n  - {id}"))
            .collect::<String>()
    };
    format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {id}\n  proposer: {OPERATOR}\n  \
         operations:\n{operations}  evidence:{listed}\n"
    )
}

/// One store of the example seed with `dup`, a Person recorded twice, retired in favour of Alice:
/// superseded when `merged` is false, merged into her when it is true. Both rest on the same
/// reason and evidence, in revision 2.
fn retired(directory: &tempfile::TempDir, backend: &'static str, merged: bool) -> World {
    let store = if merged { "merged" } else { "superseded" };
    let world = World::seeded(
        tempfile::TempDir::new_in(directory.path()).unwrap(),
        backend,
        store,
    );
    world.commit(
        "duplicate.yaml",
        "00000000-0000-4000-8000-00000000bd21",
        &document(
            "00000000-0000-4000-8000-00000000bd21",
            &format!(
                "  - !CreateNode\n    id: {DUPLICATE}\n    root_id: {ROOT}\n    type_id: {PERSON}\n    \
                 canonical_name: Dup\n    properties: {{}}\n    aliases:\n    - dup\n"
            ),
            &[],
        ),
    );
    let kind = if merged {
        format!("  - !MergeEntity\n    absorbed: {DUPLICATE}\n    into: {ALICE}\n")
    } else {
        format!("  - !SupersedeNode\n    node: {DUPLICATE}\n    by: {ALICE}\n")
    };
    world.commit(
        "retire.yaml",
        "00000000-0000-4000-8000-00000000bd22",
        &document(
            "00000000-0000-4000-8000-00000000bd22",
            &format!("{kind}    reason: recorded twice\n    evidence:\n    - {ALICE_EVIDENCE}\n"),
            &[ALICE_EVIDENCE],
        ),
    );
    world
}

/// A store that merged `dup` into Alice and one that superseded `dup` by Alice hold different
/// knowledge: the first holds `dup` absorbed, in a record carrying the lineage a split that
/// reverts the merge reads (ADR 0018 decisions 1 and 4), and its knowledge root differs. So
/// `ekr compare` does not answer that they are equal.
#[test]
fn adv_a_merged_and_a_superseded_duplicate_do_not_compare_equal() {
    for backend in BACKENDS {
        let directory = tempfile::tempdir().unwrap();
        let superseded = retired(&directory, backend, false);
        let merged = retired(&directory, backend, true);
        let (left, right) = (superseded.ok(&["snapshot"]), merged.ok(&["snapshot"]));
        let record = |snapshot: &Value| snapshot["graph"]["graph"]["superseded"][DUPLICATE].clone();
        assert!(
            record(&left)["merge"].is_null() && record(&right)["merge"].is_object(),
            "{backend}: only the merged store's record carries a lineage: {} / {}",
            record(&left),
            record(&right)
        );
        assert!(left["root"].is_object(), "{backend}: {left}");
        assert_ne!(
            left["root"], right["root"],
            "{backend}: the two stores' roots differ"
        );
        let comparison = superseded.ok(&[
            "compare",
            "--other",
            merged.file("merged").to_str().unwrap(),
        ]);
        assert_eq!(
            comparison["equal"], false,
            "{backend}: one store merged dup into Alice and the other superseded it by her, their \
             roots differ ({} / {}), and ekr compare answers that they hold the same knowledge: \
             {comparison}",
            left["root"], right["root"]
        );
    }
}
