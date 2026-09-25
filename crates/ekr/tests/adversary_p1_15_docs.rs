//! Adversary pass 1 on `story:cli-user-documentation`: `docs/cli.md` read as the specification it
//! claims to be, and run against the binary through its own worked example.
//!
//! Each case takes the page's worked-example files, changes one thing a reader following the page
//! could write, and asserts what the page says happens.

use std::path::Path;
use std::process::Output;

use serde_json::Value;

fn page() -> String {
    let root = std::path::PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("Cargo supplies the manifest directory"),
    );
    let path = root.join("../../docs/cli.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()))
}

/// The body of the fenced block whose info string carries `file=<name>`.
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

struct World {
    directory: tempfile::TempDir,
}

impl World {
    /// A fresh file store holding the worked example's host and payload files.
    fn new(page: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        for name in ["host.json", "wrote.txt", "published.txt"] {
            std::fs::write(directory.path().join(name), example_file(page, name)).unwrap();
        }
        World { directory }
    }

    fn write(&self, name: &str, body: &str) {
        std::fs::write(self.directory.path().join(name), body).unwrap();
    }

    fn run(&self, verb: &[&str]) -> Output {
        let at: &Path = self.directory.path();
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

/// docs/cli.md § Lifecycles and named operations: an operation's `name` is "the operation's name;
/// equal to its map key. `!Invoke` calls it by this name". The page's ontology rules do not refuse a
/// `name` that differs from its key, and neither does the binary — so a seed with one is accepted,
/// and the page then promises that `!Invoke` finds the operation by `name`.
///
/// After the fix, either the seed below is refused (the page's "equal to its map key" enforced, and
/// listed among the ontology rules), or `!Invoke` by `name` validates; the page must say which.
#[test]
fn invoke_calls_a_named_operation_by_the_name_the_page_says_it_uses() {
    let page = page();
    let world = World::new(&page);
    let seed = example_file(&page, "seed.yaml");
    let renamed = seed.replacen("        name: publish\n", "        name: publish_book\n", 1);
    assert_ne!(
        renamed, seed,
        "the worked seed declares the publish operation"
    );
    world.write("seed.yaml", &renamed);

    let seeded = world.run(&["seed", "seed.yaml"]);
    if seeded.status.code() != Some(0) {
        // The page's "equal to its map key" is enforced: the first branch of the fix.
        let stderr = String::from_utf8_lossy(&seeded.stderr);
        assert!(stderr.contains("seed-ontology"), "{stderr}");
        return;
    }

    let invoke = example_file(&page, "publish.yaml");
    // Keep only the `!Invoke` step of the worked transaction, calling the operation by its `name`.
    let document = format!(
        "format: ekr.transaction-document/1\ntransaction:\n  id: 00000000-0000-4000-a000-000000000791\n  proposer: 00000000-0000-4000-a000-000000000011\n  operations:\n{}  evidence: []\n",
        invoke
            .split("  - !")
            .find(|op| op.starts_with("Invoke"))
            .map(|op| format!("  - !{op}"))
            .unwrap()
            .replace("operation: publish\n", "operation: publish_book\n")
    );
    world.write("invoke.yaml", &document);
    let proposed = world.ok(&["propose", "invoke.yaml"]);
    let id = proposed["transaction_id"].as_str().unwrap().to_owned();
    let validated = world.ok(&["validate", &id]);
    if page.contains("`!Invoke` finds an operation by its **map key**, never by `name`") {
        // The third branch: the page says `!Invoke` resolves by key and the seed does not hold
        // `name` to it. Then calling by `name` is refused, and calling by key validates.
        assert_eq!(validated["kind"], "Rejected");
        assert!(
            codes(&validated).contains(&"operation-not-declared".to_owned()),
            "{:?}",
            codes(&validated)
        );
        world.write(
            "by-key.yaml",
            &document
                .replace("000000000791", "000000000792")
                .replace("operation: publish_book\n", "operation: publish\n"),
        );
        let proposed = world.ok(&["propose", "by-key.yaml"]);
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        let by_key = world.ok(&["validate", &id]);
        assert_eq!(by_key["kind"], "Validated", "{:?}", codes(&by_key));
        return;
    }
    assert_eq!(
        validated["kind"],
        "Validated",
        "docs/cli.md says `!Invoke` calls an operation by its `name`; the binary refused {:?}",
        codes(&validated)
    );
}

/// docs/cli.md § Assertions, `assessment`: "`Proposed`. Committing makes it `Accepted` by the
/// validator; any other value is rejected as `assertion-states-its-own-verdict`", and § Common
/// refusals lists that code as a validation issue (`Rejected`, exit 0). A reader who writes the
/// other value the page names, `Accepted`, must get that outcome.
///
/// After the fix, either `assessment: Accepted` proposes and validates to `Rejected` with
/// `assertion-states-its-own-verdict`, or the page says it is refused at `propose` as
/// `ekr.kernel.StructurallyInvalid` and names the form that reaches the validator.
#[test]
fn an_assessment_the_page_calls_other_than_proposed_is_rejected_with_the_code_it_names() {
    let page = page();
    let world = World::new(&page);
    world.write("seed.yaml", &example_file(&page, "seed.yaml"));
    world.ok(&["seed", "seed.yaml"]);

    let wrote = example_file(&page, "wrote.yaml");
    let accepted = wrote.replacen("assessment: Proposed", "assessment: Accepted", 1);
    assert_ne!(accepted, wrote);
    world.write("accepted.yaml", &accepted);

    let proposed = world.run(&["propose", "accepted.yaml"]);
    if page.contains(
        "a bare word such as `assessment: Accepted` is not a valid assessment at all and \
         `ekr propose` refuses the document as `ekr.kernel.StructurallyInvalid` (exit 2",
    ) {
        // The second branch the case names: refused at propose, and the page names the form that
        // reaches the validator, which must then be rejected with the code the page gives.
        assert_eq!(proposed.status.code(), Some(2));
        assert!(
            String::from_utf8_lossy(&proposed.stderr).contains("ekr.kernel.StructurallyInvalid")
        );
        let tagged = wrote.replacen(
            "assessment: Proposed",
            "assessment: !Accepted {validators: [00000000-0000-4000-a000-000000000012]}",
            1,
        );
        world.write("tagged.yaml", &tagged);
        let proposed = world.ok(&["propose", "tagged.yaml"]);
        let validated = world.ok(&["validate", proposed["transaction_id"].as_str().unwrap()]);
        assert_eq!(validated["kind"], "Rejected");
        assert!(
            codes(&validated).contains(&"assertion-states-its-own-verdict".to_owned()),
            "{:?}",
            codes(&validated)
        );
        return;
    }
    assert_eq!(
        proposed.status.code(),
        Some(0),
        "docs/cli.md says an assessment other than Proposed is rejected by validation as \
         assertion-states-its-own-verdict; propose refused it instead: {}",
        String::from_utf8_lossy(&proposed.stderr)
    );
    let id: Value = serde_json::from_slice(&proposed.stdout).unwrap();
    let validated = world.ok(&["validate", id["transaction_id"].as_str().unwrap()]);
    assert_eq!(validated["kind"], "Rejected");
    assert!(
        codes(&validated).contains(&"assertion-states-its-own-verdict".to_owned()),
        "{:?}",
        codes(&validated)
    );
}
