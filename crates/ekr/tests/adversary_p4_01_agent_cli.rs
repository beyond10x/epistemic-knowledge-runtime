//! Adversary pass 1 on `story:agent-discoverable-cli`: the printed agent text read as the
//! specification it claims to be, and driven against the binary that prints it.
//!
//! Every case uses only strings the binary printed: `ekr example`, `ekr operations <Kind>`,
//! `ekr mint`, `ekr guide`.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Output;

use ekr_kernel::{GraphOperation, TransactionDocument};
use serde_json::Value;

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
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

fn minted(kind: &str) -> String {
    let out: Value = serde_json::from_str(&text(&["mint", kind])).unwrap();
    out["id"].as_str().unwrap().to_owned()
}

/// The kinds `ekr operations` lists, the kind name first on each line.
fn listed_kinds() -> Vec<String> {
    text(&["operations"])
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split_whitespace().next().unwrap().to_owned())
        .collect()
}

/// The example operation of `ekr operations <Kind>`: every line after the `Example` heading.
fn example_operation(kind: &str) -> String {
    let out = text(&["operations", kind]);
    let at = out
        .lines()
        .position(|line| line.starts_with("Example"))
        .unwrap_or_else(|| panic!("{kind}: no Example section"));
    let mut example = out.lines().skip(at + 1).collect::<Vec<_>>().join("\n");
    example.push('\n');
    example
}

/// A complete transaction document around one printed operation. The evidence list is the
/// evidence the operation cites, as the guide says it must be.
fn document(operation: &str) -> String {
    let host: Value = serde_json::from_str(&text(&["example", "ekr.cli-host/1"])).unwrap();
    let operator = host["context"]["operator"].as_str().unwrap().to_owned();
    let mut text = format!(
        "format: ekr.transaction-document/1\ntransaction:\n  id: {}\n  proposer: {operator}\n  operations:\n",
        minted("transaction")
    );
    for line in operation.lines() {
        text.push_str("  ");
        text.push_str(line);
        text.push('\n');
    }
    let parsed = TransactionDocument::parse(text.as_bytes())
        .ok()
        .and_then(|d| d.transaction().operations.first().cloned());
    let evidence: Vec<String> = match parsed {
        Some(GraphOperation::AddAssertion(assertion)) => {
            assertion.evidence.iter().map(ToString::to_string).collect()
        }
        _ => Vec::new(),
    };
    text.push_str(&format!("  evidence: [{}]\n", evidence.join(", ")));
    text
}

/// A file store seeded from `ekr example ekr-seed/2` under `ekr example ekr.cli-host/1`.
struct World {
    directory: tempfile::TempDir,
    host: PathBuf,
}

impl World {
    fn seeded() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let host = directory.path().join("host.json");
        std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
        let world = Self { directory, host };
        let seed = world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        world.ok(&["seed", &seed]);
        world
    }

    fn file(&self, name: &str, contents: &str) -> String {
        let path = self.directory.path().join(name);
        std::fs::write(&path, contents).unwrap();
        path.display().to_string()
    }

    fn run(&self, verb: &[&str]) -> Output {
        let store = self.directory.path().join("store");
        ekr()
            .arg("--host")
            .arg(&self.host)
            .arg("--store")
            .arg(store)
            .args(["--backend", "file"])
            .args(verb)
            .output()
            .unwrap()
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{verb:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// Proposes and validates one printed operation; the validation record.
    fn validate(&self, name: &str, operation: &str) -> Value {
        let path = self.file(name, &document(operation));
        let proposed = self.ok(&["propose", &path]);
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        self.ok(&["validate", &id])
    }
}

/// Every issue `code` anywhere in a validation record.
fn codes(value: &Value, into: &mut BTreeSet<String>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(code)) = map.get("code") {
                into.insert(code.clone());
            }
            map.values().for_each(|v| codes(v, into));
        }
        Value::Array(items) => items.iter().for_each(|v| codes(v, into)),
        _ => {}
    }
}

fn issue_codes(validated: &Value) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    codes(validated, &mut found);
    found
}

/// `ekr operations <Kind>` presents every kind as something an agent can write. A kind whose
/// every proposal the kernel rejects as `unsupported-operation` must say so on its page, or the
/// agent writes a document the binary itself always refuses.
///
/// After the fix: each such kind's page (and its `ekr operations` line) names that the kind is
/// not applied in this release.
#[test]
fn a_kind_the_kernel_never_applies_says_so_where_it_is_documented() {
    let world = World::seeded();
    // The CreateNode example first, so the ids the other examples reuse (Globex, 0304) exist.
    let node = world.validate("create-node.yaml", &example_operation("CreateNode"));
    assert_eq!(node["kind"], "Validated", "{node}");
    let id = node["transaction_id"]
        .as_str()
        .or_else(|| node["record"]["transaction_id"].as_str())
        .map(ToOwned::to_owned);
    if let Some(id) = id {
        world.ok(&["commit", &id]);
    } else {
        // The record names the transaction elsewhere; find it through the listing.
        let listed = world.ok(&["transactions", "--state", "Validated"]);
        let id = listed[0]["transaction_id"].as_str().unwrap().to_owned();
        world.ok(&["commit", &id]);
    }
    assert_eq!(world.ok(&["head"])["revision"], 1);

    let listing = text(&["operations"]);
    let mut undocumented = Vec::new();
    let mut unsupported = Vec::new();
    for kind in listed_kinds() {
        if kind == "CreateNode" {
            continue;
        }
        let validated = world.validate(&format!("{kind}.yaml"), &example_operation(&kind));
        assert!(
            validated["kind"] == "Validated" || validated["kind"] == "Rejected",
            "{kind}: {validated}"
        );
        if issue_codes(&validated).contains("unsupported-operation") {
            unsupported.push(kind.clone());
            let page = text(&["operations", &kind]).to_lowercase();
            let line = listing
                .lines()
                .find(|l| l.split_whitespace().next() == Some(kind.as_str()))
                .unwrap()
                .to_lowercase();
            let says = |t: &str| t.contains("not supported") || t.contains("unsupported");
            if !says(&page) || !says(&line) {
                undocumented.push(kind);
            }
        }
    }
    assert!(
        undocumented.is_empty(),
        "the kernel rejects every proposal of {unsupported:?} as unsupported-operation, and \
         `ekr operations` documents {undocumented:?} as writable without saying so"
    );
}

/// Every `ekr operations <Kind>` page says `Example (ids from ekr example ekr-seed/2)`. Read as
/// that claim: an example validated against a store seeded from `ekr example ekr-seed/2` finds
/// every id it names that must already exist, and every property and operation it uses declared.
///
/// After the fix: no example draws an `unresolved-*`, `undeclared-*` or `*-not-declared` issue
/// against the printed seed (either the seed declares what the examples use, or the heading
/// stops claiming the ids come from it).
#[test]
fn every_example_operation_names_only_what_the_printed_seed_holds() {
    let mut dangling = Vec::new();
    for kind in listed_kinds() {
        let page = text(&["operations", &kind]);
        assert!(
            page.contains("ids from ekr example ekr-seed/2"),
            "{kind}: the page no longer makes the claim this case holds it to"
        );
        let world = World::seeded();
        let validated = world.validate(&format!("{kind}.yaml"), &example_operation(&kind));
        let missing: Vec<String> = issue_codes(&validated)
            .into_iter()
            .filter(|c| {
                c.starts_with("unresolved-")
                    || c.starts_with("undeclared-")
                    || c.ends_with("-not-declared")
            })
            .collect();
        if !missing.is_empty() {
            dangling.push(format!("{kind}: {missing:?}"));
        }
    }
    assert!(
        dangling.is_empty(),
        "examples naming ids or declarations the printed seed does not hold: {dangling:#?}"
    );
}

/// The guide: "guide, operations, example, mint and hash need none of them" (--host, --store,
/// --backend). An agent whose environment carries a store configuration it has not finished
/// writing (an empty EKR_STORE) or a mistyped EKR_BACKEND must still be able to read the guide
/// and mint ids.
///
/// After the fix: each config-free verb exits 0 whatever `EKR_*` holds.
#[test]
fn verbs_that_need_no_configuration_ignore_the_configuration_environment() {
    let mut refused = Vec::new();
    for (var, value) in [
        ("EKR_BACKEND", "bogus"),
        ("EKR_STORE", ""),
        ("EKR_HOST", ""),
    ] {
        for verb in [
            vec!["guide"],
            vec!["operations"],
            vec!["example", "ekr-seed/2"],
            vec!["mint", "node"],
            vec!["hash", "-"],
        ] {
            let output = ekr().env(var, value).args(&verb).output().unwrap();
            if output.status.code() != Some(0) {
                refused.push(format!(
                    "{var}={value:?} ekr {}: exit {:?}: {}",
                    verb.join(" "),
                    output.status.code(),
                    String::from_utf8_lossy(&output.stderr)
                        .lines()
                        .next()
                        .unwrap_or("")
                ));
            }
        }
    }
    assert!(refused.is_empty(), "{refused:#?}");
}

/// A store verb without its configuration names the flag and its variable. An empty variable is
/// no configuration either, and is the one case where the variable is the only thing the agent
/// set, so the message must name it.
///
/// After the fix: `EKR_STORE= ekr head` exits 2 and stderr names `EKR_STORE`.
#[test]
fn an_empty_configuration_variable_is_refused_by_its_name() {
    let world = World::seeded();
    let output = ekr()
        .env("EKR_HOST", &world.host)
        .env("EKR_BACKEND", "file")
        .env("EKR_STORE", "")
        .arg("head")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(
        stderr.contains("EKR_STORE"),
        "stderr does not name EKR_STORE: {stderr}"
    );
}
