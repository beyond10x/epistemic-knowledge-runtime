//! Adversary pass 2 on unit D `p5-01-cli` (story:schema-evolution-transactions, part D), against
//! ae6a3b5: D's round 1 correction with the kernel unit's correction (6538628) merged in.
//!
//! The kernel's correction made the transaction reader accept a `ModifyProperty` in the ownerless
//! P1 shape (the bare property declaration) and made profile v2 refuse it as
//! `modify-property-without-owner`. These cases hold `docs/cli.md` and `ekr schema` to that: an
//! agent that writes the P1 shape — the shape the 0.0.2 and 0.0.3 pages taught — meets a refusal
//! code and a reader/schema difference that the page and the printed description both promise to
//! name.

use std::path::Path;
use std::process::{Command, Output};

use ekr_kernel::TransactionDocument;
use serde_json::Value;

const TRANSACTION: &str = "ekr.transaction-document/1";

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
        command.env_remove(var);
    }
    command
}

fn stdout(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn page() -> String {
    let root = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    std::fs::read_to_string(Path::new(&root).join("../../docs/cli.md")).unwrap()
}

/// A `ModifyProperty` in the P1 shape, alone in a versioned transaction, against the example seed.
const P1_SHAPE: &str = "format: ekr.transaction-document/1
transaction:
  id: 00000000-0000-4000-8000-000000000611
  proposer: 00000000-0000-4000-8000-000000000101
  operations:
  - !ModifyProperty
    id: 00000000-0000-4000-8000-000000000802
    name: nickname
    value_type:
      value_kind: String
    cardinality: One
    required: false
    constraints: []
  evidence: []
  schema_version: 00000000-0000-4000-8000-000000000004
";

/// A store seeded from the example seed under the example host with the v2 pair, exactly as
/// `ekr guide` says to write it.
struct V2Store {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl V2Store {
    fn seeded(backend: &'static str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let host = stdout(&["example", "ekr.cli-host/1"])
            .replace("\"ekr.p1-deterministic/1\"", "\"ekr.p2-deterministic/1\"")
            .replace("\"ekr.p1-apply/1\"", "\"ekr.p2-apply/1\"");
        std::fs::write(directory.path().join("host.json"), host).unwrap();
        std::fs::write(
            directory.path().join("seed.yaml"),
            stdout(&["example", "ekr-seed/2"]),
        )
        .unwrap();
        let store = Self { directory, backend };
        let seeded = store.run(&["seed", "seed.yaml"]);
        assert_eq!(seeded.status.code(), Some(0), "{backend}: {seeded:?}");
        store
    }

    fn run(&self, args: &[&str]) -> Output {
        let store = match self.backend {
            "file" => "store",
            _ => "state.db",
        };
        ekr()
            .current_dir(self.directory.path())
            .args([
                "--host",
                "host.json",
                "--store",
                store,
                "--backend",
                self.backend,
            ])
            .args(args)
            .output()
            .unwrap()
    }

    fn ok(&self, args: &[&str]) -> Value {
        let output = self.run(args);
        assert_eq!(output.status.code(), Some(0), "{args:?}: {output:?}");
        serde_json::from_slice(&output.stdout).unwrap()
    }
}

/// The refusal table's first column, one name per row.
fn refusal_names(page: &str) -> Vec<String> {
    page.lines()
        .filter_map(|line| line.strip_prefix("| `"))
        .filter_map(|rest| rest.split_once("` |").map(|(name, _)| name.to_owned()))
        .collect()
}

/// `ekr validate` refuses the P1 `ModifyProperty` shape under profile v2 with the issue code
/// `modify-property-without-owner` (exit 0, `kind: Rejected`), on both providers. `docs/cli.md`
/// says its refusal table holds every refusal (`docs_cli.rs` claims "every code a schema change
/// is refused with is listed or unreachable"), and the code is neither: the suite's list of the
/// structural validator's schema-shape codes is a hand-written three, and the kernel added a
/// fourth.
#[test]
fn the_refusal_table_lists_modify_property_without_owner() {
    for backend in ["file", "sqlite"] {
        let store = V2Store::seeded(backend);
        std::fs::write(store.directory.path().join("p1.yaml"), P1_SHAPE).unwrap();
        let proposed = store.ok(&["propose", "p1.yaml"]);
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        let validated = store.ok(&["validate", &id]);
        assert_eq!(validated["kind"], "Rejected", "{backend}: {validated}");
        let codes: Vec<&str> = validated["issues"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|issue| issue["code"].as_str())
            .collect();
        assert_eq!(
            codes,
            ["modify-property-without-owner"],
            "{backend}: {validated}"
        );
    }
    let names = refusal_names(&page());
    assert!(
        names
            .iter()
            .any(|name| name == "modify-property-without-owner"),
        "docs/cli.md's refusal table has no `modify-property-without-owner` row, and `ekr \
         validate` returns it under profile v2 for a ModifyProperty written as the bare P1 \
         declaration"
    );
}

/// `docs/cli.md` § `ekr schema`: "The printed `description` names every place the schema and the
/// reader differ", with a table of them. After the kernel correction the reader accepts a
/// `ModifyProperty` written as the bare P1 declaration (`ekr propose` records it) and the printed
/// schema refuses it (`PropertyModification` requires `owner` and `property`). Neither the
/// description nor the page's table names that difference.
#[test]
fn the_schema_gap_for_the_p1_modify_property_shape_is_named() {
    TransactionDocument::parse(P1_SHAPE.as_bytes()).expect("the reader accepts the P1 shape");
    let printed: Value = serde_json::from_str(&stdout(&["schema", TRANSACTION])).unwrap();
    let validator = jsonschema::draft202012::new(&printed).unwrap();
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(P1_SHAPE).unwrap();
    let instance = serde_json::to_value(yaml).unwrap();
    assert!(
        !validator.is_valid(&instance),
        "precondition: the printed schema refuses the P1 shape"
    );

    let description = printed["description"].as_str().unwrap();
    let page = page();
    let section = page
        .split("### `ekr schema`")
        .nth(1)
        .and_then(|rest| rest.split("\n## ").next())
        .unwrap();
    let mut missing = Vec::new();
    if !description.contains("ModifyProperty") {
        missing.push("`ekr schema ekr.transaction-document/1` description");
    }
    if !section.contains("ModifyProperty") {
        missing.push("docs/cli.md § `ekr schema` table");
    }
    assert!(
        missing.is_empty(),
        "the reader accepts a ModifyProperty in the P1 shape and the schema refuses it; not named \
         in: {missing:?}"
    );
}
