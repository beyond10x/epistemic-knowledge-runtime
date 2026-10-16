//! `story:explain-scoped-to-cited-evidence` (ekr-mcp-fixes-20261015 #7): `ekr explain --format
//! ekr.explanation/3` answers only the evidence the chain cites, counting the rest as
//! `omitted_evidence`, unless `--scope all`; with `--documents` it answers no proposal record and
//! no base64 payload beside a text; and the whole answer is at most 48 KiB, the longest evidence
//! text halved and marked `truncated` until it fits (`kernel.yaml`, `ekr.kernel.ExplanationV3`).
//! Without `--format` it prints `ekr.explanation/2` as before.
//!
//! Every case runs the built binary on both providers, under the example host and seed, after one
//! transaction that creates 40 Organizations, adds 40 evidence entries of 1,400 bytes of text
//! each, and asserts each Organization's `legal_name` citing its own entry: the transaction's
//! `evidence` lists all 40, so `ekr.explanation/2` of the first assertion lists 40 Evidence links,
//! and that assertion cites one.

use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const LEGAL_NAME: &str = "00000000-0000-4000-8000-000000000801";
const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const TRANSACTION: &str = "00000000-0000-4000-8000-00000000b000";
/// The entries, nodes and assertions are `<base> + k`, k from 1 to [`ENTRIES`].
const ENTRIES: u64 = 40;
const NODES: u64 = 0xb100;
const EVIDENCE: u64 = 0xb200;
const ASSERTIONS: u64 = 0xb300;
/// The bytes of each entry's text.
const TEXT_BYTES: usize = 1_400;
/// `ekr.explanation/3`'s cap.
const CAP: usize = 48 * 1024;

fn id(n: u64) -> String {
    format!("00000000-0000-4000-8000-{n:012x}")
}

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
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

/// Entry `k`'s text: ASCII, `TEXT_BYTES` long, its number first.
fn statement(k: u64) -> String {
    let mut text = format!("Statement {k:02} on the legal name of organization {k:02}.");
    while text.len() < TEXT_BYTES {
        text.push_str(" It is recorded here word for word.");
    }
    text.truncate(TEXT_BYTES);
    text
}

/// The transaction: 40 `!CreateNode`, 40 `!AddEvidence` and 40 `!AddAssertion`, assertion k on
/// node k citing entry k.
fn transaction() -> String {
    let mut document = format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {TRANSACTION}\n  proposer: \
         {OPERATOR}\n  operations:\n"
    );
    for k in 1..=ENTRIES {
        let node = id(NODES + k);
        document.push_str(&format!(
            "  - !CreateNode\n    id: {node}\n    root_id: {ROOT}\n    type_id: {ORGANIZATION}\n    \
             canonical_name: Organization {k:02}\n    properties: {{}}\n    aliases: \
             [Organization {k:02}]\n"
        ));
        let bytes = statement(k).into_bytes();
        let hash = ekr_core::ContentHash::of_bytes(&bytes);
        let payload: Vec<String> = bytes.iter().map(u8::to_string).collect();
        document.push_str(&format!(
            "  - !AddEvidence\n    evidence:\n      id: {}\n      source: !HumanStatement\n        \
             identity: Runtime operator\n      content_hash: {hash}\n      extracted_by: \
             {OPERATOR}\n      observed_at: 1773273600000\n      confidence: 10000\n    payload: \
             [{}]\n",
            id(EVIDENCE + k),
            payload.join(", ")
        ));
        document.push_str(&format!(
            "  - !AddAssertion\n    id: {}\n    root_id: {ROOT}\n    subject: !Node {node}\n    \
             predicate: !Property {LEGAL_NAME}\n    object: !Value\n      value_kind: String\n      \
             value: Organization {k:02} Ltd\n    evidence:\n    - {}\n    proposed_by: {OPERATOR}\n    \
             assessment: Proposed\n    lifecycle: Active\n    valid_time:\n      from: null\n      \
             to: null\n    transaction_time:\n      recorded_from: 0\n      recorded_to: null\n",
            id(ASSERTIONS + k),
            id(EVIDENCE + k)
        ));
    }
    document.push_str("  evidence:\n");
    for k in 1..=ENTRIES {
        document.push_str(&format!("  - {}\n", id(EVIDENCE + k)));
    }
    document
}

/// One provider in its own directory under the example host and seed, with the transaction
/// committed.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn committed(backend: &'static str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        world.file("host.json", &text(&["example", "ekr.cli-host/1"]));
        world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        world.ok(&["seed", "seed.yaml"]);
        world.file("batch.yaml", &transaction());
        let proposed: Value = serde_json::from_str(&world.ok(&["propose", "batch.yaml"])).unwrap();
        let transaction = proposed["transaction_id"].as_str().unwrap().to_owned();
        let validated: Value =
            serde_json::from_str(&world.ok(&["validate", &transaction])).unwrap();
        assert_eq!(validated["kind"], "Validated", "{backend}: {validated}");
        let committed: Value = serde_json::from_str(&world.ok(&["commit", &transaction])).unwrap();
        assert_eq!(committed["kind"], "Committed", "{backend}: {committed}");
        world
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn file(&self, name: &str, contents: &str) {
        std::fs::write(self.directory.path().join(name), contents).unwrap();
    }

    fn run(&self, verb: &[&str]) -> Output {
        ekr()
            .current_dir(self.directory.path())
            .arg("--host")
            .arg(self.directory.path().join("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(verb)
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    /// A verb that must exit 0: its stdout, as printed.
    fn ok(&self, verb: &[&str]) -> String {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap()
    }

    /// `ekr explain <the first assertion> <options>`, printed and parsed.
    fn explain(&self, options: &[&str]) -> (String, Value) {
        let first = id(ASSERTIONS + 1);
        let mut verb = vec!["explain", first.as_str()];
        verb.extend_from_slice(options);
        let printed = self.ok(&verb);
        let parsed = serde_json::from_str(&printed).unwrap();
        (printed, parsed)
    }
}

/// The ids of the document's Evidence links, in link order.
fn evidence_ids(explained: &Value) -> Vec<String> {
    explained["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|link| link["kind"] == "Evidence")
        .map(|link| link["id"].as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn a_scoped_explanation_answers_only_the_cited_evidence() {
    // The scope names the CLI accepts are the two `ekr.kernel.EvidenceScope` values.
    let scopes: Vec<String> = <ekr::cli::Scope as clap::ValueEnum>::value_variants()
        .iter()
        .filter_map(clap::ValueEnum::to_possible_value)
        .map(|value| value.get_name().to_owned())
        .collect();
    assert_eq!(scopes, ["cited", "all"]);
    for backend in BACKENDS {
        let world = World::committed(backend);
        let every: Vec<String> = (1..=ENTRIES).map(|k| id(EVIDENCE + k)).collect();

        // ekr.explanation/2, printed without a format: every entry of the transaction.
        let (_, plain) = world.explain(&[]);
        assert_eq!(plain["format"], "ekr.explanation/2", "{backend}");
        assert_eq!(evidence_ids(&plain), every, "{backend}: /2 lists the batch");
        assert!(plain.get("omitted_evidence").is_none(), "{backend}");

        // ekr-mcp-fixes-20261015 #7: /3, scope cited by default and when named.
        let (default, cited) = world.explain(&["--format", "ekr.explanation/3"]);
        let (named, _) = world.explain(&["--format", "ekr.explanation/3", "--scope", "cited"]);
        assert_eq!(default, named, "{backend}: cited is /3's default");
        assert_eq!(cited["scope"], "cited", "{backend}");
        assert_eq!(
            evidence_ids(&cited),
            [id(EVIDENCE + 1)],
            "{backend}: the one entry the assertion cites"
        );
        assert_eq!(cited["omitted_evidence"], ENTRIES - 1, "{backend}");
        // The other links are /2's, in /2's order.
        let kinds = |explained: &Value| -> Vec<String> {
            explained["links"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|link| link["kind"] != "Evidence")
                .map(|link| link.to_string())
                .collect()
        };
        assert_eq!(kinds(&cited), kinds(&plain), "{backend}");

        let (_, all) = world.explain(&["--format", "ekr.explanation/3", "--scope", "all"]);
        assert_eq!(all["scope"], "all", "{backend}");
        assert_eq!(evidence_ids(&all), every, "{backend}: scope all");
        assert_eq!(all["omitted_evidence"], 0, "{backend}");
        assert_eq!(
            all["links"], plain["links"],
            "{backend}: scope all is /2's chain"
        );
    }
}

#[test]
fn a_scoped_explanation_with_documents_stays_under_its_cap() {
    for backend in BACKENDS {
        let world = World::committed(backend);

        // The premise: /2 with every record is over the cap.
        let (whole, _) = world.explain(&["--documents"]);
        assert!(whole.len() > CAP, "{backend}: /2 is {} bytes", whole.len());

        // ekr-mcp-fixes-20261015 #7: every entry's text, 56,000 bytes of it, within 48 KiB.
        let options = [
            "--documents",
            "--format",
            "ekr.explanation/3",
            "--scope",
            "all",
        ];
        let (printed, capped) = world.explain(&options);
        assert!(
            printed.len() <= CAP,
            "{backend}: /3 is {} bytes, over {CAP}",
            printed.len()
        );
        assert_eq!(
            world.explain(&options).0,
            printed,
            "{backend}: the same request, the same bytes"
        );
        let links = capped["links"].as_array().unwrap();
        let mut truncated = 0;
        for link in links {
            match link["kind"].as_str().unwrap() {
                "Proposal" => assert!(link.get("record").is_none(), "{backend}: {link}"),
                "Commit" => assert!(
                    link["receipt"].is_object() && link["receipt"].get("proposal").is_none(),
                    "{backend}: a receipt without its proposal record"
                ),
                "Evidence" => {
                    let text = link["text"].as_str().unwrap();
                    assert!(link.get("payload").is_none(), "{backend}: {link}");
                    if link["truncated"] == true {
                        truncated += 1;
                        assert!(text.len() < TEXT_BYTES, "{backend}");
                        assert_eq!(link["offset"], 0, "{backend}: {link}");
                        assert_eq!(link["record_length"], TEXT_BYTES, "{backend}: {link}");
                    } else {
                        assert_eq!(text.len(), TEXT_BYTES, "{backend}");
                    }
                    let k = u64::from_str_radix(&link["id"].as_str().unwrap()[24..], 16).unwrap()
                        - EVIDENCE;
                    assert!(
                        statement(k).starts_with(text),
                        "{backend}: a cut keeps the text's first part"
                    );
                }
                _ => {}
            }
        }
        assert!(truncated > 0, "{backend}: the cap cut a text");
        assert!(
            !capped.to_string().contains("document_bytes"),
            "{backend}: no proposal record anywhere"
        );

        // Scope cited, the default: one text, whole, far under the cap.
        let (printed, cited) = world.explain(&["--documents", "--format", "ekr.explanation/3"]);
        assert!(printed.len() <= CAP, "{backend}");
        let texts: Vec<&Value> = cited["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|link| link["kind"] == "Evidence")
            .collect();
        assert_eq!(texts.len(), 1, "{backend}");
        assert_eq!(texts[0]["text"], statement(1), "{backend}");
        assert!(texts[0].get("truncated").is_none(), "{backend}");
    }
}
