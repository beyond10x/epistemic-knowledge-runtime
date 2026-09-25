//! Adversary pass 2 on `story:cli-user-documentation`: `docs/cli.md` read as the specification it
//! claims to be.
//!
//! Case 1 follows the page's own retraction, supersession and root rules from the worked example,
//! makes the mistakes a reader of those rules can make, and asserts that the refusal the binary
//! returns is one the page's refusal table names, so that the reader can look it up.
//!
//! Case 2 holds the refusal table's `where` column for validation-issue rows to the validators:
//! a row the page files as a validation issue must name a code a validator raises. The page's own
//! guard (`docs_cli.rs`) runs only the rows that are not validation issues, and checks the others
//! only against all runtime source, so a named refusal relabelled `validation issue | 0` drops out
//! of every check.

use std::path::{Path, PathBuf};
use std::process::Output;

use serde_json::Value;

/// The workspace root, from `CARGO_MANIFEST_DIR` by lexical parents (as `docs_cli.rs` does).
fn root() -> PathBuf {
    PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the manifest dir"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/ekr is two levels below the root")
        .to_path_buf()
}

fn page() -> String {
    let path = root().join("docs/cli.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

fn example_file(page: &str, name: &str) -> String {
    let wanted = format!("file={name}");
    let mut body: Option<String> = None;
    for line in page.lines() {
        match body.as_mut() {
            None => {
                if line.starts_with("```") && line.split_whitespace().any(|w| w == wanted) {
                    body = Some(String::new());
                }
            }
            Some(text) => {
                if line == "```" {
                    return body.unwrap();
                }
                text.push_str(line);
                text.push('\n');
            }
        }
    }
    panic!("docs/cli.md has no block with {wanted}")
}

/// `(name, where, exit)` for every row of the page's `## Common refusals` table.
fn refusal_rows(page: &str) -> Vec<(String, String, String)> {
    let start = page
        .find("\n## Common refusals\n")
        .expect("the page has a Common refusals section");
    let rest = &page[start + 1..];
    let end = rest[3..].find("\n## ").map_or(rest.len(), |at| at + 3);
    rest[..end]
        .lines()
        .filter(|line| line.starts_with("| `"))
        .map(|line| {
            let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
            (
                cells[0].trim_matches('`').to_owned(),
                cells[1].to_owned(),
                cells[2].to_owned(),
            )
        })
        .collect()
}

struct World {
    directory: tempfile::TempDir,
}

impl World {
    fn new(page: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        for name in ["host.json", "seed.yaml", "wrote.yaml"] {
            std::fs::write(directory.path().join(name), example_file(page, name)).unwrap();
        }
        World { directory }
    }

    fn write(&self, name: &str, body: &str) {
        std::fs::write(self.directory.path().join(name), body).unwrap();
    }

    fn run(&self, verb: &[&str]) -> Output {
        let at = self.directory.path();
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
        for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
            command.env_remove(var);
        }
        command
            .current_dir(at)
            .arg("--host")
            .arg(at.join("host.json"))
            .arg("--store")
            .arg(at.join("store"))
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

    /// Proposes `body` as `name` and validates it; returns the validation outcome.
    fn validate(&self, name: &str, body: &str) -> Value {
        self.write(name, body);
        let proposed = self.ok(&["propose", name]);
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        self.ok(&["validate", &id])
    }

    fn commit(&self, name: &str, body: &str) {
        let validated = self.validate(name, body);
        assert_eq!(validated["kind"], "Validated", "{validated}");
        let id = validated["transaction_id"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| {
                body.lines()
                    .find_map(|l| l.strip_prefix("  id: "))
                    .unwrap()
                    .split_whitespace()
                    .next()
                    .unwrap()
                    .to_owned()
            });
        let committed = self.ok(&["commit", &id]);
        assert_eq!(committed["kind"], "Committed", "{committed}");
    }
}

fn codes(validation: &Value) -> Vec<String> {
    validation["issues"]
        .as_array()
        .map(|issues| {
            issues
                .iter()
                .map(|i| i["code"].as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

const OPERATOR: &str = "00000000-0000-4000-a000-000000000011";
const ROOT: &str = "00000000-0000-4000-a000-000000000002";
const WROTE_ASSERTION: &str = "00000000-0000-4000-a000-000000000501";

fn retract(transaction: &str) -> String {
    format!(
        "format: ekr.transaction-document/1\ntransaction:\n  id: {transaction}\n  proposer: {OPERATOR}\n  operations:\n  - !RetractAssertion\n    assertion: {WROTE_ASSERTION}\n    reason: filed against the wrong author\n  evidence: []\n"
    )
}

/// docs/cli.md § Operation kinds: `RetractAssertion` "withdraws an accepted, active assertion";
/// `SupersedeAssertion` takes `effective_from` "(the replacement's `valid_time.from`)"; an
/// assertion's `root_id` is "the graph root id". § Common refusals: "each issue carries the `code`"
/// and the table gives "what it means" and "what to fix" for each.
///
/// A reader who retracts an assertion twice, supersedes with an `effective_from` that is not the
/// replacement's `from`, or writes a `root_id` that is not the graph root follows the page's rules
/// wrongly in the most ordinary way. Each is a `Rejected` validation. After the fix, the code each
/// returns is a row of the page's refusal table, so the reader finds what to fix.
#[test]
fn every_refusal_a_reader_of_the_retraction_and_supersession_rules_meets_is_in_the_table() {
    let page = page();
    let named: Vec<String> = refusal_rows(&page).into_iter().map(|row| row.0).collect();
    assert!(named.len() >= 20, "the refusal table parsed: {named:?}");
    let world = World::new(&page);
    world.ok(&["seed", "seed.yaml"]);
    world.commit("wrote.yaml", &example_file(&page, "wrote.yaml"));

    let wrote = example_file(&page, "wrote.yaml");
    let add = |id: &str, from: &str, root: &str| {
        wrote
            .split("  - !CreateEdge")
            .next()
            .unwrap()
            .replace(WROTE_ASSERTION, id)
            .replacen(&format!("root_id: {ROOT}"), &format!("root_id: {root}"), 1)
            .replace("from: 1554076800000", &format!("from: {from}"))
    };
    let mut met: Vec<(&str, Vec<String>)> = Vec::new();

    // Supersede with an `effective_from` one millisecond after the replacement's `from`.
    let supersede = format!(
        "{}  - !SupersedeAssertion\n    assertion: {WROTE_ASSERTION}\n    by: 00000000-0000-4000-a000-000000000581\n    effective_from: 1600000000001\n  evidence:\n  - 00000000-0000-4000-a000-000000000401\n",
        add(
            "00000000-0000-4000-a000-000000000581",
            "1600000000000",
            ROOT,
        )
        .replace("000000000701", "000000000781")
    );
    assert!(supersede.contains("!SupersedeAssertion"), "{supersede}");
    let outcome = world.validate("supersede.yaml", &supersede);
    assert_eq!(outcome["kind"], "Rejected", "{outcome}");
    met.push((
        "effective_from is not the replacement's from",
        codes(&outcome),
    ));

    // An assertion whose `root_id` is not the graph root.
    let rooted = add(
        "00000000-0000-4000-a000-000000000582",
        "1554076800000",
        "00000000-0000-4000-a000-000000000009",
    )
    .replace("000000000701", "000000000782");
    let rooted = format!("{rooted}  evidence:\n  - 00000000-0000-4000-a000-000000000401\n");
    let outcome = world.validate("rooted.yaml", &rooted);
    assert_eq!(outcome["kind"], "Rejected", "{outcome}");
    met.push(("root_id is not the graph root", codes(&outcome)));

    // Retract the worked assertion, then retract it again.
    world.commit(
        "retract.yaml",
        &retract("00000000-0000-4000-a000-000000000783"),
    );
    let outcome = world.validate(
        "retract-again.yaml",
        &retract("00000000-0000-4000-a000-000000000784"),
    );
    assert_eq!(outcome["kind"], "Rejected", "{outcome}");
    met.push(("retracting a retracted assertion", codes(&outcome)));

    let unnamed: Vec<(&str, Vec<String>)> = met
        .into_iter()
        .map(|(what, codes)| {
            (
                what,
                codes
                    .into_iter()
                    .filter(|code| !named.contains(code))
                    .collect::<Vec<_>>(),
            )
        })
        .filter(|(_, codes)| !codes.is_empty())
        .collect();
    assert!(
        unnamed.is_empty(),
        "docs/cli.md § Common refusals does not name the code a reader meets: {unnamed:?}"
    );
}

/// docs/cli.md § Common refusals: "A validation issue, exit 0. `ekr validate` records
/// `\"kind\": \"Rejected\"`, and each issue carries the `code`." A row filed as `validation issue`
/// therefore names a code a validator raises. After a fix to `docs_cli.rs` that holds the same
/// thing, relabelling a named refusal (for example `seed-unsupported-source`) as a validation issue
/// fails a check; today only this case fails on it.
#[test]
fn every_row_filed_as_a_validation_issue_names_a_code_a_validator_raises() {
    let page = page();
    let validators = root().join("crates/ekr-kernel/src/validate");
    let mut source = String::new();
    for entry in std::fs::read_dir(&validators).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "rs") {
            source.push_str(&std::fs::read_to_string(&path).unwrap());
        }
    }
    let rows = refusal_rows(&page);
    let issue_rows: Vec<&(String, String, String)> = rows
        .iter()
        .filter(|row| row.1 == "validation issue")
        .collect();
    assert!(issue_rows.len() >= 20, "{issue_rows:?}");
    let not_raised: Vec<&str> = issue_rows
        .iter()
        .map(|row| row.0.as_str())
        .filter(|code| !source.contains(&format!("\"{code}\"")))
        .collect();
    assert!(
        not_raised.is_empty(),
        "docs/cli.md files these as validation issues; no validator raises them: {not_raised:?}"
    );
}
