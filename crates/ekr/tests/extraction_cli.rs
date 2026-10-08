//! `story:extraction-verb-shares-the-sdk-path`: `ekr apply-extraction` runs the SDK's apply
//! routine in process, and writes what that routine writes over a child `ekr session`.
//!
//! Every case drives the built binary, and the SDK case a real child session of it. Stores are
//! compared with their ids ignored: a node by its type, name and aliases, an assertion by what it
//! says of whom, an evidence entry whole and by the bytes `ekr explain --documents` returns.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Output;

use ekr_sdk::binary::EkrBinary;
use ekr_sdk::document::{
    AgentId, ExtractedFact, ExtractedReference, ExtractionDocument, ExtractionRefusal,
    PropertyFact, RelationFact, ValueSpec,
};
use ekr_sdk::extraction::{
    apply, AmbiguousExtraction, ApplyError, ApplyOptions, CommittedExtraction, ExtractionIssue,
    ExtractionReport, HeldExtraction, HeldReason, RejectedExtraction,
};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";

/// A document for the example seed's own types: no ontology section, so a store under the example
/// host, whose profile fixes the schema, takes it. Carol is new; Globex is new; both facts cite
/// item 0403.
const KNOWN_TYPES: &str = "format: ekr.extraction-document/1
entities:
- node_type: Person
  aliases: [Carol]
facts:
- !Property
  subject: {node_type: Organization, aliases: [Globex]}
  property: legal_name
  value: {value_kind: String, value: Globex Corporation}
  evidence: [00000000-0000-4000-8000-000000000403]
- !Relation
  subject: {node_type: Person, aliases: [Carol]}
  relation: CEO_OF
  object: {node_type: Organization, aliases: [Globex]}
  evidence: [00000000-0000-4000-8000-000000000403]
evidence:
- evidence:
    id: 00000000-0000-4000-8000-000000000403
    source: !HumanStatement
      identity: Quarterly report
    content_hash: 7afeb9c852d895a2cdf8d7717a49354badd8b8dc79049925137b1f95f4651af5
    extracted_by: 00000000-0000-4000-8000-000000000101
    observed_at: 1773273600000
    confidence: 10000
  payload: [67, 97, 114, 111, 108, 44, 32, 67, 69, 79, 32, 111, 102, 32, 71, 108, 111, 98, 101, 120, 32, 67, 111, 114, 112, 111, 114, 97, 116, 105, 111, 110, 44, 32, 108, 101, 97, 100, 115, 32, 112, 114, 111, 106, 101, 99, 116, 32, 65, 112, 111, 108, 108, 111, 46]
";

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ekr"))
}

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(binary());
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
        "{args:?}: stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// One provider in its own directory: the example host, v2 when asked, and the example seed.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn new(backend: &'static str, schema_evolving: bool) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        let mut host = text(&["example", "ekr.cli-host/1"]);
        if schema_evolving {
            host = host
                .replace("\"ekr.p1-deterministic/1\"", "\"ekr.p2-deterministic/1\"")
                .replace("\"ekr.p1-apply/1\"", "\"ekr.p2-apply/1\"");
        }
        std::fs::write(world.host(), host).unwrap();
        std::fs::write(world.file("seed.yaml"), text(&["example", "ekr-seed/2"])).unwrap();
        let seed = world.file("seed.yaml");
        world.store_json(&["seed", seed.to_str().unwrap()]);
        world
    }

    fn file(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn host(&self) -> PathBuf {
        self.file("host.json")
    }

    fn store(&self) -> PathBuf {
        self.file("store")
    }

    fn config(&self) -> StoreConfig {
        StoreConfig {
            host: self.host(),
            store: self.store(),
            backend: match self.backend {
                "file" => Backend::File,
                _ => Backend::Sqlite,
            },
        }
    }

    fn command(&self, args: &[&str]) -> std::process::Command {
        let mut command = ekr();
        command
            .arg("--host")
            .arg(self.host())
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(args);
        command
    }

    fn store_output(&self, args: &[&str]) -> Output {
        self.command(args).output().unwrap()
    }

    fn store_json(&self, args: &[&str]) -> Value {
        let output = self.store_output(args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?}: stderr {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// `ekr apply-extraction` of `document`, which must exit 0: the report.
    fn apply_verb(&self, document: &str) -> Value {
        let path = self.file("extraction.yaml");
        std::fs::write(&path, document).unwrap();
        self.store_json(&["apply-extraction", path.to_str().unwrap()])
    }

    /// The SDK's routine over a child `ekr session` of the same binary: the report as JSON.
    fn apply_sdk(&self, document: &str) -> Value {
        let binary = EkrBinary::open(binary()).unwrap();
        let mut session =
            ProcessSession::start(&binary, self.config(), SessionOptions::default()).unwrap();
        let document = ExtractionDocument::from_yaml(document).unwrap();
        let report = apply(
            &mut session,
            &document,
            OPERATOR.parse::<AgentId>().unwrap(),
        )
        .unwrap_or_else(|error| panic!("the SDK's apply failed: {error}"));
        session.close().unwrap();
        serde_json::to_value(report).unwrap()
    }

    /// The store with its ids ignored: nodes by type, name and aliases; active assertions by what
    /// they say of whom, each with the bytes of the evidence it cites; evidence entries whole.
    fn content(&self) -> Value {
        let snapshot = self.store_json(&["snapshot"]);
        let graph = &snapshot["graph"]["graph"];
        let mut names: BTreeMap<String, String> = graph["nodes"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(id, node)| {
                (
                    id.clone(),
                    format!("node {}", node["canonical_name"].as_str().unwrap()),
                )
            })
            .collect();
        // Types and properties a schema change added are minted by whoever applied it.
        let ontology = self.store_json(&["ontology"]);
        for list in ["node_types", "edge_types"] {
            for declared in ontology[list].as_array().unwrap() {
                let name = declared["name"].as_str().unwrap();
                names.insert(
                    declared["id"].as_str().unwrap().to_owned(),
                    format!("type {name}"),
                );
                for property in declared["properties"].as_array().unwrap() {
                    names.insert(
                        property["id"].as_str().unwrap().to_owned(),
                        format!("property {name}.{}", property["name"].as_str().unwrap()),
                    );
                }
            }
        }
        let mut nodes: Vec<Value> = graph["nodes"]
            .as_object()
            .unwrap()
            .values()
            .map(|node| {
                let mut node = node.clone();
                node.as_object_mut().unwrap().remove("id");
                renamed(&node, &names)
            })
            .collect();
        nodes.sort_by_key(ToString::to_string);
        let mut assertions: Vec<Value> = graph["assertions"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(id, assertion)| {
                let mut said = assertion.clone();
                let fields = said.as_object_mut().unwrap();
                fields.remove("id");
                fields.remove("transaction_time");
                fields.remove("assessment");
                let explained = self.store_json(&["explain", id, "--documents"]);
                let bytes: Vec<Value> = explained["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|link| link["kind"] == "Evidence")
                    .map(|link| link["payload"].clone())
                    .collect();
                fields.insert("evidence_bytes".to_owned(), Value::Array(bytes));
                renamed(&said, &names)
            })
            .collect();
        assertions.sort_by_key(ToString::to_string);
        serde_json::json!({
            "nodes": nodes,
            "assertions": assertions,
            "evidence": graph["evidence"],
        })
    }
}

/// `value` with every node, type and property id replaced by its name, in values and keys.
fn renamed(value: &Value, names: &BTreeMap<String, String>) -> Value {
    match value {
        Value::String(text) => names
            .get(text)
            .map_or_else(|| value.clone(), |name| Value::String(name.clone())),
        Value::Array(items) => Value::Array(items.iter().map(|v| renamed(v, names)).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .iter()
                .map(|(key, v)| {
                    let key = names.get(key).unwrap_or(key).clone();
                    (key, renamed(v, names))
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

fn example_document() -> String {
    text(&["example", "ekr.extraction-document/1"])
}

/// Acceptance 1: one fixture, through the verb and through the SDK over a child session, gives
/// equal stores — the example document, which grows the schema, on a v2 store of each provider.
#[test]
fn the_verb_and_the_sdk_over_a_child_session_write_equal_stores() {
    let document = example_document();
    for backend in BACKENDS {
        let (verb, sdk) = (World::new(backend, true), World::new(backend, true));
        let by_verb = verb.apply_verb(&document);
        let by_sdk = sdk.apply_sdk(&document);
        for report in [&by_verb, &by_sdk] {
            assert_eq!(
                report["rejected"],
                serde_json::json!([]),
                "{backend}: {report}"
            );
            assert_eq!(
                report["ambiguous"],
                serde_json::json!([]),
                "{backend}: {report}"
            );
            // The schema change, the new nodes, the facts.
            assert_eq!(
                report["committed"].as_array().unwrap().len(),
                3,
                "{backend}: {report}"
            );
        }
        let (left, right) = (verb.content(), sdk.content());
        assert_eq!(left, right, "{backend}: the two stores differ");
        // Four facts, each one active assertion citing 0403's bytes.
        let cited = left["assertions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|assertion| assertion["evidence"].to_string().contains("000000000403"))
            .count();
        assert_eq!(cited, 4, "{backend}: {left}");
        // Carol, Apollo and Globex were created, each named by its alias and carrying it.
        for name in ["Carol", "Apollo", "Globex"] {
            let created = left["nodes"].as_array().unwrap().iter().any(|node| {
                node["canonical_name"] == name && node["aliases"] == serde_json::json!([name])
            });
            assert!(created, "{backend}: {name} in {}", left["nodes"]);
        }
        let ontology = verb.store_json(&["ontology"]).to_string();
        assert!(ontology.contains("\"Project\"") && ontology.contains("\"LEADS\""));
    }
}

/// Acceptance 3: the verb starts no process — with an empty `PATH` it applies a fixture.
#[test]
fn the_verb_applies_a_fixture_with_an_empty_path() {
    for backend in BACKENDS {
        let world = World::new(backend, false);
        let path = world.file("extraction.yaml");
        std::fs::write(&path, KNOWN_TYPES).unwrap();
        let output = world
            .command(&["apply-extraction", path.to_str().unwrap()])
            .env_clear()
            .env("PATH", "")
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["rejected"], serde_json::json!([]), "{report}");
        assert_eq!(report["committed"].as_array().unwrap().len(), 2, "{report}");
        assert_eq!(world.store_json(&["head"])["revision"], 2);
    }
}

/// A document the engine's reader refuses is refused by its code, exit 2, and nothing is written.
#[test]
fn a_document_the_reader_refuses_is_refused_by_name_and_writes_nothing() {
    for backend in BACKENDS {
        let world = World::new(backend, false);
        let path = world.file("extraction.yaml");
        std::fs::write(
            &path,
            KNOWN_TYPES.replace("node_type: Person", "node_type: Starship"),
        )
        .unwrap();
        let output = world.store_output(&["apply-extraction", path.to_str().unwrap()]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{backend}: {stderr}");
        assert!(
            stderr.starts_with("ekr: extraction-type-undeclared: "),
            "{backend}: {stderr}"
        );
        assert!(output.stdout.is_empty());
        assert_eq!(world.store_json(&["head"])["revision"], 0);
        assert_eq!(world.store_json(&["transactions"]), serde_json::json!([]));
    }
}

/// A session refuses the verb: it is a run of the verbs a session already serves.
#[test]
fn a_session_refuses_the_verb() {
    let world = World::new("file", false);
    let mut command = world.command(&["session"]);
    command
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());
    let mut child = command.spawn().unwrap();
    {
        use std::io::Write as _;
        let mut stdin = child.stdin.take().unwrap();
        writeln!(
            stdin,
            "{{\"argv\": [\"apply-extraction\", \"-\"], \"stdin\": \"\"}}"
        )
        .unwrap();
    }
    let output = child.wait_with_output().unwrap();
    let answer: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(answer["exit"], 2, "{answer}");
    assert!(
        answer["stderr"]
            .as_str()
            .unwrap()
            .starts_with("ekr: session-verb-refused: "),
        "{answer}"
    );
}

/// The SDK's mirror reads the document `ekr example` prints in its value-type form, and what it
/// writes back the engine's reader reads as the same document.
#[test]
fn the_sdk_mirror_reads_the_example_and_the_engine_reads_what_it_writes() {
    let example = example_document();
    let mirrored = ExtractionDocument::from_yaml(&example).unwrap();
    let project = mirrored
        .ontology
        .node_types
        .iter()
        .find(|node| node.name == "Project")
        .expect("the example declares Project");
    assert_eq!(
        project.properties[0].value,
        ValueSpec::Enum(vec!["active".to_owned(), "closed".to_owned()])
    );
    let written = mirrored.to_yaml().unwrap();
    assert_eq!(ExtractionDocument::from_yaml(&written).unwrap(), mirrored);
    assert_eq!(
        ekr_integrate::ExtractionDocument::from_yaml(&written).unwrap(),
        ekr_integrate::ExtractionDocument::from_yaml(&example).unwrap(),
        "{written}"
    );
}

/// A `NodeRef` names its node types in `parameters: {allowed_types: [...]}`, as the engine reads
/// it; the tuple form the SDK's own type has is not the document's.
#[test]
fn the_sdk_mirror_reads_a_node_ref_as_the_document_writes_it() {
    let document = |parameters: &str| {
        format!(
            "format: ekr.extraction-document/1\nontology:\n  node_types:\n  - name: Ticket\n    \
             properties:\n    - name: owner\n      value: {{value_kind: NodeRef, parameters: \
             {parameters}}}\nfacts: []\nevidence: []\n"
        )
    };
    let read = ExtractionDocument::from_yaml(&document("{allowed_types: [Person]}")).unwrap();
    assert_eq!(
        read.ontology.node_types[0].properties[0].value,
        ValueSpec::NodeRef(vec!["Person".to_owned()])
    );
    let written = read.to_yaml().unwrap();
    assert!(written.contains("allowed_types:"), "{written}");
    assert!(ekr_integrate::ExtractionDocument::from_yaml(&written).is_ok());
    assert!(ExtractionDocument::from_yaml(&document("[Person]")).is_err());
}

/// The fields `systems/ekr/domains/integrate.yaml` declares for `name`.
fn declared_fields(name: &str) -> std::collections::BTreeSet<String> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let path = PathBuf::from(manifest).join("../../systems/ekr/domains/integrate.yaml");
    let domain: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    let declared = domain["types"]
        .as_sequence()
        .unwrap()
        .iter()
        .find(|declared| declared["name"].as_str() == Some(name))
        .unwrap_or_else(|| panic!("integrate.yaml declares no {name}"));
    declared["fields"]
        .as_sequence()
        .unwrap()
        .iter()
        .map(|field| field["name"].as_str().unwrap().to_owned())
        .collect()
}

#[track_caller]
fn keys(value: &Value) -> std::collections::BTreeSet<String> {
    let Some(fields) = value.as_object() else {
        panic!("not an object: {value}")
    };
    fields.keys().cloned().collect()
}

/// The report carries exactly the fields the domain declares for `ekr.integrate.ExtractionReport`
/// and each of its rows: a committed transaction, an ambiguous named thing — whose fact is not
/// applied — and a part validation rejected, with its transaction and issues.
#[test]
fn the_report_carries_the_fields_the_domain_declares() {
    let world = World::new("file", false);
    // Dana and Dee are two Persons, so a reference known by both names is ambiguous.
    let two = KNOWN_TYPES.replace(
        "entities:\n- node_type: Person\n  aliases: [Carol]\n",
        "entities:\n- node_type: Person\n  aliases: [Carol]\n- node_type: Person\n  aliases: \
         [Dana]\n- node_type: Person\n  aliases: [Dee]\n",
    );
    let first = world.apply_verb(&two);
    assert_eq!(first["rejected"], serde_json::json!([]), "{first}");
    let report = world.apply_verb(&KNOWN_TYPES.replace("aliases: [Carol]", "aliases: [Dana, Dee]"));
    assert_eq!(
        keys(&report),
        declared_fields("ekr.integrate.ExtractionReport")
    );
    assert_eq!(
        keys(&first["committed"][0]),
        declared_fields("ekr.integrate.CommittedExtraction"),
        "{first}"
    );
    let row = &report["ambiguous"][0];
    assert_eq!(
        keys(row),
        declared_fields("ekr.integrate.AmbiguousExtraction"),
        "{report}"
    );
    assert_eq!(row["candidates"].as_array().unwrap().len(), 2, "{report}");
    assert_eq!(
        row["reference"]["aliases"],
        serde_json::json!(["Dana", "Dee"])
    );
    assert_eq!(report["ambiguous"].as_array().unwrap().len(), 1, "{report}");
    // The relation about Dana or Dee was not applied; the property about Globex is held: the first
    // document asserted it with the same evidence, so nothing is committed.
    assert_eq!(report["rejected"], serde_json::json!([]), "{report}");
    assert_eq!(report["committed"], serde_json::json!([]), "{report}");
    assert_eq!(
        report["held"],
        serde_json::json!([{"item": "facts[0]", "reason": "asserted"}]),
        "{report}"
    );
    assert_eq!(
        keys(&report["held"][0]),
        declared_fields("ekr.integrate.HeldExtraction"),
        "{report}"
    );
    assert_eq!(report["stopped"], Value::Null, "{report}");

    // The example grows the schema, which the example host's profile fixes: the schema change is
    // rejected by validation, and nothing after it is tried.
    let fixed = World::new("sqlite", false);
    let report = fixed.apply_verb(&example_document());
    assert_eq!(report["committed"], serde_json::json!([]), "{report}");
    let rejected = &report["rejected"][0];
    assert_eq!(
        keys(rejected),
        declared_fields("ekr.integrate.RejectedExtraction"),
        "{report}"
    );
    assert_eq!(rejected["item"], "ontology", "{report}");
    assert!(rejected["transaction_id"].is_string(), "{report}");
    assert_eq!(
        keys(&rejected["issues"][0]),
        declared_fields("ekr.integrate.ExtractionIssue"),
        "{report}"
    );
    assert_eq!(fixed.store_json(&["head"])["revision"], 0);
}

/// The SDK's routine over a child session, read through its own types: the mirror's format and
/// facts, a committed transaction, an ambiguous named thing, a rejected schema change with its
/// issues, a document the mirror does not read, and a name no declaration gives, which stops the
/// routine as `ApplyError::Undeclared` before it writes anything.
#[test]
fn the_sdk_routine_reports_through_its_own_types() {
    assert_eq!(
        ekr_sdk::document::EXTRACTION_FORMAT,
        ekr_integrate::EXTRACTION_FORMAT
    );
    let unread: ExtractionRefusal = ExtractionDocument::from_yaml(
        "format: ekr.extraction-document/0\nfacts: []\nevidence: []\n",
    )
    .unwrap_err();
    assert_eq!(unread.code, "extraction-document-malformed", "{unread}");

    let world = World::new("file", false);
    let binary = EkrBinary::open(binary()).unwrap();
    let mut session =
        ProcessSession::start(&binary, world.config(), SessionOptions::default()).unwrap();
    let operator = OPERATOR.parse::<AgentId>().unwrap();

    // Dana and Dee are two Persons, so a reference known by both names is ambiguous.
    let two = KNOWN_TYPES.replace(
        "entities:\n- node_type: Person\n  aliases: [Carol]\n",
        "entities:\n- node_type: Person\n  aliases: [Carol]\n- node_type: Person\n  aliases: \
         [Dana]\n- node_type: Person\n  aliases: [Dee]\n",
    );
    let first: ExtractionReport = apply(
        &mut session,
        &ExtractionDocument::from_yaml(&two).unwrap(),
        operator,
    )
    .unwrap();
    assert!(first.rejected.is_empty(), "{first:?}");

    let document = ExtractionDocument::from_yaml(
        &KNOWN_TYPES.replace("aliases: [Carol]", "aliases: [Dana, Dee]"),
    )
    .unwrap();
    let property: PropertyFact = match &document.facts[0] {
        ExtractedFact::Property(fact) => fact.clone(),
        other => panic!("facts[0] is a property: {other:?}"),
    };
    assert_eq!(
        property.subject,
        ExtractedReference::new("Organization", ["Globex"])
    );
    let relation: RelationFact = match &document.facts[1] {
        ExtractedFact::Relation(fact) => fact.clone(),
        other => panic!("facts[1] is a relation: {other:?}"),
    };
    assert_eq!(
        relation.subject,
        ExtractedReference::new("Person", ["Dana", "Dee"])
    );

    let committed: &CommittedExtraction = &first.committed[0];
    assert!(committed.revision > 0, "{first:?}");
    let report: ExtractionReport = apply(&mut session, &document, operator).unwrap();
    assert!(report.rejected.is_empty(), "{report:?}");
    // The property about Globex is the one the first document asserted: held, not asserted again.
    assert!(report.committed.is_empty(), "{report:?}");
    let held: &HeldExtraction = &report.held[0];
    assert_eq!(
        (held.item.as_str(), held.reason),
        ("facts[0]", HeldReason::Asserted),
        "{report:?}"
    );
    assert_eq!(report.held.len(), 1, "{report:?}");
    assert_eq!(report.stopped, None);
    assert_eq!(report.ambiguous.len(), 1, "{report:?}");
    let ambiguous: &AmbiguousExtraction = &report.ambiguous[0];
    assert_eq!(ambiguous.reference, relation.subject);
    assert_eq!(ambiguous.candidates.len(), 2, "{report:?}");

    // No ontology section, and a type the store does not declare: nothing is written.
    session.close().unwrap();
    let head = world.store_json(&["head"]);
    let mut session =
        ProcessSession::start(&binary, world.config(), SessionOptions::default()).unwrap();
    let robot = ExtractionDocument::from_yaml(&KNOWN_TYPES.replace(
        "- node_type: Person\n  aliases: [Carol]\n",
        "- node_type: Robot\n  aliases: [Carol]\n",
    ))
    .unwrap();
    match apply(&mut session, &robot, operator) {
        Err(ApplyError::Undeclared { kind, name }) => {
            assert_eq!((kind, name.as_str()), ("node type", "Robot"));
        }
        other => panic!("an undeclared type stops the routine: {other:?}"),
    }
    session.close().unwrap();
    assert_eq!(world.store_json(&["head"]), head);

    // The example grows the schema, which the example host's profile fixes.
    let fixed = World::new("file", false);
    let mut session =
        ProcessSession::start(&binary, fixed.config(), SessionOptions::default()).unwrap();
    let example = ExtractionDocument::from_yaml(&example_document()).unwrap();
    let report: ExtractionReport = apply(&mut session, &example, operator).unwrap();
    session.close().unwrap();
    assert!(report.committed.is_empty(), "{report:?}");
    let rejected: &RejectedExtraction = &report.rejected[0];
    assert_eq!(rejected.item, "ontology");
    assert!(rejected.transaction_id.is_some(), "{report:?}");
    assert_eq!(rejected.refusal, None);
    let issue: &ExtractionIssue = &rejected.issues[0];
    assert!(
        !issue.code.is_empty() && !issue.validator.is_empty(),
        "{report:?}"
    );
}

/// The SDK's mirror refuses what the engine's reader refuses while decoding, and two evidence
/// items under one id or a payload that does not hash to its entry, each by the reader's code;
/// its limits are the reader's.
#[test]
fn the_mirror_refuses_by_the_readers_codes() {
    assert_eq!(
        ekr_sdk::document::EXTRACTION_INPUT_BYTES,
        ekr_integrate::EXTRACTION_INPUT_BYTES
    );
    assert_eq!(
        ekr_sdk::document::EXTRACTION_DEPTH,
        ekr_integrate::EXTRACTION_DEPTH
    );
    let mut deep = "{value_kind: String, value: x}".to_owned();
    for _ in 0..20 {
        deep = format!("{{value_kind: List, value: [{deep}]}}");
    }
    let second = KNOWN_TYPES
        .split_once("evidence:\n- evidence:")
        .map(|(_, item)| format!("- evidence:{item}"))
        .unwrap()
        .replace("identity: Quarterly report", "identity: Another report");
    let cases = [
        (
            "format: ekr.extraction-document/1\nentities:\n- node_type: Person\n  aliases: [&n \
             Carol]\n- node_type: Person\n  aliases: [*n]\nfacts: []\nevidence: []\n"
                .to_owned(),
            "extraction-yaml-alias",
        ),
        (
            "format: ekr.extraction-document/1\nentities:\n- node_type: Person\n  aliases: \
             [2024]\nfacts: []\nevidence: []\n"
                .to_owned(),
            "extraction-document-malformed",
        ),
        (
            format!(
                "format: ekr.extraction-document/1\nentities:\n- node_type: Person\n  aliases: \
                 [{}]\nfacts: []\nevidence: []\n",
                "C".repeat(ekr_integrate::EXTRACTION_INPUT_BYTES)
            ),
            "extraction-document-too-large",
        ),
        (
            format!(
                "format: ekr.extraction-document/1\nfacts:\n- !Property\n  subject: {{node_type: \
                 Person, aliases: [Carol]}}\n  property: tags\n  value: {deep}\n  evidence: \
                 [00000000-0000-4000-8000-000000000403]\nevidence: []\n"
            ),
            "extraction-document-too-deep",
        ),
        (format!("{KNOWN_TYPES}{second}"), "duplicate-identity"),
        (
            KNOWN_TYPES.replace("confidence: 10000", "confidence: 20000"),
            "extraction-document-malformed",
        ),
        (
            KNOWN_TYPES.replace(
                "value: {value_kind: String, value: Globex Corporation}",
                "value: {value_kind: Record, value: {a: {value_kind: Integer, value: 1}, a: \
                 {value_kind: Integer, value: 2}}}",
            ),
            "extraction-document-malformed",
        ),
        (
            KNOWN_TYPES.replace(
                "entities:",
                "ontology:\n  node_types:\n  - name: Gadget\n    properties:\n    - name: size\n      \
                 value: {value_kind: Record, parameters: {w: {value_kind: Integer}, w: \
                 {value_kind: Integer}}}\nentities:",
            ),
            "extraction-document-malformed",
        ),
        (
            KNOWN_TYPES.replacen(
                "evidence: [00000000-0000-4000-8000-000000000403]",
                "evidence: [00000000-0000-4000-8000-000000000404]",
                1,
            ),
            "fact-evidence-unlisted",
        ),
        (
            KNOWN_TYPES.replace(
                "source: !HumanStatement\n      identity: Quarterly report",
                "source: !Url https://example.org/report",
            ),
            "extraction-evidence-kind-unsupported",
        ),
        (
            KNOWN_TYPES.replace("payload: [67,", "payload: [68,"),
            "evidence-payload-mismatch",
        ),
    ];
    let world = World::new("file", false);
    for (document, code) in cases {
        let reader = match ekr_integrate::ExtractionDocument::from_yaml(&document) {
            Err(error) => error.refusal().code.code().to_owned(),
            // A check of the document alone, which the reader makes after the store's: read off
            // the strict verb's refusal over a store the document's types are declared in.
            Ok(_) => {
                let path = world.file("refused.yaml");
                std::fs::write(&path, &document).unwrap();
                let output =
                    world.store_output(&["apply-extraction", "--strict", path.to_str().unwrap()]);
                assert_eq!(output.status.code(), Some(2), "{code}");
                let stderr = String::from_utf8(output.stderr).unwrap();
                stderr
                    .strip_prefix("ekr: ")
                    .and_then(|rest| rest.split_once(':'))
                    .map(|(name, _)| name.to_owned())
                    .unwrap_or(stderr)
            }
        };
        assert_eq!(reader, code, "the reader");
        let mirror = ExtractionDocument::from_yaml(&document).map(|_| ());
        assert_eq!(
            mirror.map_err(|refusal| refusal.code),
            Err(code.to_owned()),
            "the mirror"
        );
    }
}

/// A child session that dies after its first commit: every later request is refused as a
/// process that ended before answering.
struct DiesAfterFirstCommit {
    session: ProcessSession,
    committed: bool,
}

impl ekr_sdk::transport::Transport for DiesAfterFirstCommit {
    fn request(
        &mut self,
        request: &ekr_sdk::transport::Request,
    ) -> Result<ekr_sdk::reply::Reply, ekr_sdk::transport::TransportError> {
        if self.committed {
            return Err(ekr_sdk::transport::TransportError::Died {
                verb: request.verb().to_owned(),
                status: "cut by the test".to_owned(),
                stderr_tail: String::new(),
            });
        }
        let reply = self.session.request(request)?;
        self.committed = request.verb() == "commit" && reply.exit == 0;
        Ok(reply)
    }
}

/// Once something has committed, the routine does not end in an error: a request that then gets
/// no answer stops it, and the report lists what committed and says why it stopped. Before
/// anything committed, the same failure is the error it is, with nothing written.
#[test]
fn a_failure_after_a_commit_is_reported_with_what_committed() {
    let world = World::new("file", true);
    let binary = EkrBinary::open(binary()).unwrap();
    let document = ExtractionDocument::from_yaml(&example_document()).unwrap();
    let operator = OPERATOR.parse::<AgentId>().unwrap();
    let mut cut = DiesAfterFirstCommit {
        session: ProcessSession::start(&binary, world.config(), SessionOptions::default()).unwrap(),
        committed: false,
    };
    let report = apply(&mut cut, &document, operator).unwrap();
    // The schema change committed; resolving the first named thing found the session gone.
    assert_eq!(report.committed.len(), 1, "{report:?}");
    let stopped = report.stopped.as_deref().unwrap_or_default();
    assert!(stopped.contains("cut by the test"), "{report:?}");
    cut.session.close().unwrap();
    assert_eq!(world.store_json(&["head"])["revision"], 1);

    let fresh = World::new("file", true);
    let mut dead = DiesAfterFirstCommit {
        session: ProcessSession::start(&binary, fresh.config(), SessionOptions::default()).unwrap(),
        committed: true,
    };
    assert!(matches!(
        apply(&mut dead, &document, operator),
        Err(ApplyError::Read(_))
    ));
    dead.session.close().unwrap();
    assert_eq!(fresh.store_json(&["head"])["revision"], 0);
}

/// A new subtype redeclaring a property its parent declares otherwise: the property is the
/// parent's and no schema operation lets the subtype change it. Decided (wave correct-07): the
/// reader refuses the document as `extraction-property-conflict`, exit 2, before anything is
/// written; over a child session the SDK routine refuses it before any write too. The same
/// declaration made alike, and the subtype's own property declared on the parent, both apply.
#[test]
fn a_subtype_redeclaring_an_inherited_property_is_refused_before_any_write() {
    let conflicting = "format: ekr.extraction-document/1
ontology:
  node_types:
  - name: Company
    parents: [Organization]
    properties:
    - name: legal_name
      value: {value_kind: Integer}
entities:
- node_type: Company
  aliases: [Initech]
facts: []
evidence: []
";
    let world = World::new("file", true);
    let path = world.file("conflict.yaml");
    std::fs::write(&path, conflicting).unwrap();
    let output = world.store_output(&["apply-extraction", path.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.starts_with("ekr: extraction-property-conflict: ")
            && stderr.contains("Company.legal_name"),
        "{stderr}"
    );
    assert_eq!(world.store_json(&["head"])["revision"], 0);

    let binary = EkrBinary::open(binary()).unwrap();
    let mut session =
        ProcessSession::start(&binary, world.config(), SessionOptions::default()).unwrap();
    let mirrored = ExtractionDocument::from_yaml(conflicting).unwrap();
    let refused = apply(
        &mut session,
        &mirrored,
        OPERATOR.parse::<AgentId>().unwrap(),
    );
    session.close().unwrap();
    assert!(
        matches!(refused, Err(ApplyError::Ontology(_))),
        "{refused:?}"
    );
    assert_eq!(world.store_json(&["head"])["revision"], 0);

    // Declared alike, the subtype inherits the parent's property and the document applies.
    let alike = world.apply_verb(&conflicting.replace(
        "value: {value_kind: Integer}",
        "value: {value_kind: String}",
    ));
    assert_eq!(alike["rejected"], serde_json::json!([]), "{alike}");
    assert_eq!(alike["stopped"], Value::Null, "{alike}");
}

/// The id of the one active assertion of `world` whose object mentions `needle`.
fn active_saying(world: &World, needle: &str) -> ekr_sdk::document::AssertionId {
    let snapshot = world.store_json(&["snapshot"]);
    let found: Vec<&Value> = snapshot["graph"]["graph"]["assertions"]
        .as_object()
        .unwrap()
        .values()
        .filter(|assertion| {
            assertion["lifecycle"] == "Active" && assertion["object"].to_string().contains(needle)
        })
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
    found[0]["id"].as_str().unwrap().parse().unwrap()
}

/// `operations` committed as one transaction over a child session, as the host operator.
fn commit_by_hand(world: &World, operations: Vec<ekr_sdk::document::Operation>) {
    let binary = EkrBinary::open(binary()).unwrap();
    let mut session =
        ProcessSession::start(&binary, world.config(), SessionOptions::default()).unwrap();
    let report = ekr_sdk::batch::Batcher::new(OPERATOR.parse().unwrap())
        .commit(&mut session, &[operations])
        .unwrap();
    session.close().unwrap();
    assert_eq!(report.committed.len(), 1, "{report:?}");
}

/// A claim an operator retracted or superseded is not asserted again from the evidence it cited:
/// the report holds the fact with that reason. From evidence the claim never cited, it is
/// asserted.
#[test]
fn a_retracted_or_superseded_claim_is_held_with_its_reason() {
    use ekr_sdk::document::{
        Assertion, Object, Operation, Predicate, Retraction, Subject, Supersession, TemporalRange,
        Timestamp, Value as Said,
    };

    let retracted = World::new("file", true);
    retracted.apply_verb(KNOWN_TYPES);
    let id = active_saying(&retracted, "Globex Corporation");
    commit_by_hand(
        &retracted,
        vec![Operation::RetractAssertion(Retraction::new(id, "wrong"))],
    );
    let report = retracted.apply_verb(KNOWN_TYPES);
    assert_eq!(report["committed"], serde_json::json!([]), "{report}");
    assert_eq!(
        report["held"],
        serde_json::json!([
            {"item": "facts[0]", "reason": "retracted"},
            {"item": "facts[1]", "reason": "asserted"}
        ]),
        "{report}"
    );

    // The same claim from item 0404, which the retracted assertion never cited: asserted.
    let payload = b"Globex Corporation, as the registry lists it.".to_vec();
    let bytes: Vec<String> = payload.iter().map(u8::to_string).collect();
    let fresh = format!(
        "format: ekr.extraction-document/1
facts:
- !Property
  subject: {{node_type: Organization, aliases: [Globex]}}
  property: legal_name
  value: {{value_kind: String, value: Globex Corporation}}
  evidence: [00000000-0000-4000-8000-000000000404]
evidence:
- evidence:
    id: 00000000-0000-4000-8000-000000000404
    source: !HumanStatement
      identity: Registry extract
    content_hash: {}
    extracted_by: 00000000-0000-4000-8000-000000000101
    observed_at: 1773360000000
    confidence: 9000
  payload: [{}]
",
        ekr_sdk::document::payload_hash(&payload),
        bytes.join(", ")
    );
    let report = retracted.apply_verb(&fresh);
    assert_eq!(report["held"], serde_json::json!([]), "{report}");
    assert_eq!(report["committed"].as_array().unwrap().len(), 1, "{report}");

    let superseded = World::new("file", true);
    superseded.apply_verb(KNOWN_TYPES);
    let old = active_saying(&superseded, "Globex Corporation");
    let snapshot = superseded.store_json(&["snapshot"]);
    let graph = &snapshot["graph"]["graph"];
    let globex = graph["nodes"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, node)| node["canonical_name"] == "Globex")
        .map(|(id, _)| id.parse().unwrap())
        .unwrap();
    let from = Timestamp::from_millis(1_775_001_600_000);
    let replacement = Assertion::new(
        graph["root"]["id"].as_str().unwrap().parse().unwrap(),
        Subject::Node(globex),
        Predicate::Property("00000000-0000-4000-8000-000000000801".parse().unwrap()),
        Object::Value(Said::String("Globex Inc".to_owned())),
        OPERATOR.parse().unwrap(),
    )
    .citing("00000000-0000-4000-8000-000000000403".parse().unwrap())
    .with_valid_time(TemporalRange::since(from));
    let by = replacement.id;
    commit_by_hand(
        &superseded,
        vec![
            Operation::AddAssertion(Box::new(replacement)),
            Operation::SupersedeAssertion(Supersession::new(old, by, from)),
        ],
    );
    let report = superseded.apply_verb(KNOWN_TYPES);
    assert_eq!(report["committed"], serde_json::json!([]), "{report}");
    assert_eq!(
        report["held"][0],
        serde_json::json!({"item": "facts[0]", "reason": "superseded"}),
        "{report}"
    );
}

/// The id `00000000-0000-4000-8000-000000000<suffix>`.
fn evidence_id(suffix: &str) -> String {
    format!("00000000-0000-4000-8000-000000000{suffix}")
}

/// One evidence item of a document, under [`evidence_id`] of `suffix`, observed at
/// `observed_at`, whose bytes are `payload`.
fn evidence_item(suffix: &str, payload: &[u8], observed_at: u64) -> String {
    let bytes: Vec<String> = payload.iter().map(u8::to_string).collect();
    format!(
        "- evidence:
    id: {}
    source: !HumanStatement
      identity: Source {suffix}
    content_hash: {}
    extracted_by: {OPERATOR}
    observed_at: {observed_at}
    confidence: 9000
  payload: [{}]
",
        evidence_id(suffix),
        ekr_sdk::document::payload_hash(payload),
        bytes.join(", ")
    )
}

/// A `!Property` fact: the Organization known as `name` has the legal name `value`, citing the
/// items of `evidence`, by suffix.
fn legal_name(name: &str, value: &str, evidence: &[&str]) -> String {
    let cited: Vec<String> = evidence.iter().map(|suffix| evidence_id(suffix)).collect();
    format!(
        "- !Property
  subject: {{node_type: Organization, aliases: [{name}]}}
  property: legal_name
  value: {{value_kind: String, value: {value}}}
  evidence: [{}]
",
        cited.join(", ")
    )
}

/// A document of `facts` and `evidence`, each already written as list items.
fn extraction(facts: &[String], evidence: &[String]) -> String {
    format!(
        "format: ekr.extraction-document/1\nfacts:\n{}evidence:\n{}",
        facts.concat(),
        evidence.concat()
    )
}

/// The SDK's routine over a child session on a document the mirror decodes without its own
/// checks: the report as JSON.
fn apply_decoded(world: &World, text: &str, options: &ApplyOptions) -> Result<Value, ApplyError> {
    let binary = EkrBinary::open(binary()).unwrap();
    let mut session =
        ProcessSession::start(&binary, world.config(), SessionOptions::default()).unwrap();
    let document = ExtractionDocument::decode(text).unwrap();
    let report = ekr_sdk::extraction::apply_with(
        &mut session,
        &document,
        OPERATOR.parse::<AgentId>().unwrap(),
        options,
    );
    session.close().unwrap();
    report.map(|report| serde_json::to_value(report).unwrap())
}

/// The canonical names of the nodes of `content` that start with `prefix`, sorted.
fn named(content: &Value, prefix: &str) -> Vec<String> {
    let mut names: Vec<String> = content["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|node| node["canonical_name"].as_str())
        .filter(|name| name.starts_with(prefix))
        .map(str::to_owned)
        .collect();
    names.sort();
    names
}

/// `story:extraction-partial-apply`, acceptance: ten facts, one of which (`facts[4]`) cites
/// evidence the document does not carry. Through the verb and through the SDK over a child
/// session the other nine apply, and the report lists `facts[4]` with its index and the refusal
/// code `fact-evidence-unlisted`. Nothing only it names — the organisation it is about, the
/// evidence item only it cites — is created or added. Asked for strictly, the verb (`--strict`)
/// and the SDK ([`ApplyOptions::strict`]) refuse the whole document and write nothing.
#[test]
fn one_bad_fact_of_ten_is_skipped_and_the_other_nine_apply() {
    let facts: Vec<String> = (0..10)
        .map(|at| {
            if at == 4 {
                legal_name("Org4", "Org Four", &["410", "499"])
            } else {
                legal_name(&format!("Org{at}"), &format!("Org {at} Ltd"), &["400"])
            }
        })
        .collect();
    let text = extraction(
        &facts,
        &[
            evidence_item(
                "400",
                b"Ten organisations and their legal names.",
                1_773_273_600_000,
            ),
            evidence_item(
                "410",
                b"Org Four, as one register lists it.",
                1_773_360_000_000,
            ),
        ],
    );
    let skipped = serde_json::json!([{
        "item": "facts[4]",
        "transaction_id": null,
        "issues": [],
        "refusal": format!("fact-evidence-unlisted: facts[4]: {}", evidence_id("499")),
    }]);
    for backend in BACKENDS {
        let (verb, sdk) = (World::new(backend, false), World::new(backend, false));
        let by_verb = verb.apply_verb(&text);
        let by_sdk = apply_decoded(&sdk, &text, &ApplyOptions::default())
            .unwrap_or_else(|error| panic!("{backend}: the SDK's apply failed: {error}"));
        for report in [&by_verb, &by_sdk] {
            assert_eq!(report["rejected"], skipped, "{backend}: {report}");
            assert_eq!(report["held"], serde_json::json!([]), "{backend}: {report}");
            assert_eq!(report["stopped"], Value::Null, "{backend}: {report}");
            // The new nodes, then the facts.
            assert_eq!(
                report["committed"].as_array().unwrap().len(),
                2,
                "{backend}: {report}"
            );
        }
        let (left, right) = (verb.content(), sdk.content());
        assert_eq!(left, right, "{backend}: the two stores differ");
        let expected: Vec<String> = (0..10)
            .filter(|at| *at != 4)
            .map(|at| format!("Org{at}"))
            .collect();
        assert_eq!(named(&left, "Org"), expected, "{backend}");
        let asserted = left["assertions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|assertion| {
                assertion["evidence"]
                    .to_string()
                    .contains(&evidence_id("400"))
            })
            .count();
        assert_eq!(asserted, 9, "{backend}: {left}");
        assert!(
            !left["evidence"].to_string().contains(&evidence_id("410")),
            "{backend}: the item only the skipped fact cites was added: {}",
            left["evidence"]
        );
        assert!(!left.to_string().contains("Org Four"), "{backend}: {left}");
    }

    let world = World::new("file", false);
    let path = world.file("extraction.yaml");
    std::fs::write(&path, &text).unwrap();
    let output = world.store_output(&["apply-extraction", "--strict", path.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(
        stderr.starts_with("ekr: fact-evidence-unlisted: "),
        "{stderr}"
    );
    assert!(output.stdout.is_empty());
    match apply_decoded(&world, &text, &ApplyOptions::default().strict()) {
        Err(ApplyError::Refused(refusal)) => {
            assert_eq!(refusal.code, "fact-evidence-unlisted", "{refusal}");
        }
        other => panic!("strict refuses the whole document: {other:?}"),
    }
    assert_eq!(world.store_json(&["head"])["revision"], 0);

    // A fact the caller's own reader refused, as the verb hands the engine reader's refusals on,
    // is skipped with that refusal beside the routine's own, and nothing only it names is made.
    let caller = ExtractionRefusal {
        code: "extraction-value-mismatch".to_owned(),
        name: "facts[7]: Organization.legal_name".to_owned(),
    };
    let report = apply_decoded(
        &world,
        &text,
        &ApplyOptions::default().refusing(BTreeMap::from([(7, caller.clone())])),
    )
    .unwrap();
    let rows: Vec<(Value, Value)> = report["rejected"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| (row["item"].clone(), row["refusal"].clone()))
        .collect();
    assert_eq!(
        rows,
        [
            (serde_json::json!("facts[4]"), skipped[0]["refusal"].clone()),
            (
                serde_json::json!("facts[7]"),
                serde_json::json!(caller.to_string())
            ),
        ],
        "{report}"
    );
    assert_eq!(named(&world.content(), "Org").len(), 8);
}

/// `story:extraction-partial-apply`, decision 3: each fact the engine's reader refuses is skipped
/// by the verb with the reader's code — a property its subject's type does not declare, a value
/// its property does not hold, a relation between types its edge type does not connect, a subject
/// of a type nothing declares — and the rest applies. The SDK routine over a child session skips
/// the two of them it names itself by the same code and reason. A defect of the document as a
/// whole, two evidence items under one id, still refuses it, exit 2, with nothing written.
#[test]
fn every_fact_the_reader_refuses_is_skipped_by_its_code_and_the_rest_applies() {
    let cite = evidence_id("400");
    let facts = vec![
        legal_name("Zenith0", "Zenith Zero", &["400"]),
        format!(
            "- !Property\n  subject: {{node_type: Organization, aliases: [Zenith1]}}\n  property: \
             ticker\n  value: {{value_kind: String, value: ACM}}\n  evidence: [{cite}]\n"
        ),
        format!(
            "- !Property\n  subject: {{node_type: Organization, aliases: [Zenith2]}}\n  property: \
             legal_name\n  value: {{value_kind: Integer, value: 2}}\n  evidence: [{cite}]\n"
        ),
        format!(
            "- !Relation\n  subject: {{node_type: Organization, aliases: [Zenith3]}}\n  relation: \
             CEO_OF\n  object: {{node_type: Person, aliases: [Dora]}}\n  evidence: [{cite}]\n"
        ),
        format!(
            "- !Property\n  subject: {{node_type: Starship, aliases: [Enterprise]}}\n  property: \
             legal_name\n  value: {{value_kind: String, value: NCC-1701}}\n  evidence: [{cite}]\n"
        ),
        legal_name("Zenith5", "Zenith Five", &["400"]),
    ];
    let item = evidence_item(
        "400",
        b"Zenith and its sister companies, by legal name.",
        1_773_273_600_000,
    );
    let text = extraction(&facts, std::slice::from_ref(&item));
    let row = |at: usize, refusal: &str| {
        serde_json::json!({
            "item": format!("facts[{at}]"),
            "transaction_id": null,
            "issues": [],
            "refusal": refusal,
        })
    };
    let undeclared_property = row(1, "extraction-property-undeclared: Organization.ticker");
    let undeclared_type = row(4, "extraction-type-undeclared: Starship");
    for backend in BACKENDS {
        let world = World::new(backend, false);
        let report = world.apply_verb(&text);
        assert_eq!(
            report["rejected"],
            serde_json::json!([
                undeclared_property,
                row(
                    2,
                    "extraction-value-mismatch: facts[2]: Organization.legal_name"
                ),
                row(
                    3,
                    "extraction-relation-ends: facts[3]: Organization CEO_OF Person"
                ),
                undeclared_type,
            ]),
            "{backend}: {report}"
        );
        let content = world.content();
        assert_eq!(
            named(&content, "Zenith"),
            ["Zenith0", "Zenith5"],
            "{backend}"
        );
        for absent in ["Dora", "Enterprise"] {
            assert!(named(&content, absent).is_empty(), "{backend}: {absent}");
        }

        let sdk = World::new(backend, false);
        let report = apply_decoded(&sdk, &text, &ApplyOptions::default()).unwrap();
        let rows = report["rejected"].as_array().unwrap();
        for wanted in [&undeclared_property, &undeclared_type] {
            assert!(rows.contains(wanted), "{backend}: {wanted} in {report}");
        }
        assert!(named(&sdk.content(), "Enterprise").is_empty(), "{backend}");
    }

    let twice = extraction(&facts, &[item.clone(), item]);
    let world = World::new("file", false);
    let path = world.file("extraction.yaml");
    std::fs::write(&path, twice).unwrap();
    let output = world.store_output(&["apply-extraction", path.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(stderr.starts_with("ekr: duplicate-identity: "), "{stderr}");
    assert_eq!(world.store_json(&["head"])["revision"], 0);
}

/// Each active assertion of `world` about the node named `name`, as its predicate's kind and its
/// valid time's `from`, sorted.
fn valid_from(world: &World, name: &str) -> Vec<(String, Value)> {
    let snapshot = world.store_json(&["snapshot"]);
    let graph = &snapshot["graph"]["graph"];
    let node = graph["nodes"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, node)| node["canonical_name"] == name)
        .map(|(id, _)| id.clone())
        .unwrap_or_else(|| panic!("no node {name}"));
    let mut found: Vec<(String, Value)> = graph["assertions"]
        .as_object()
        .unwrap()
        .values()
        .filter(|assertion| {
            assertion["lifecycle"] == "Active" && assertion["subject"]["Node"] == node.as_str()
        })
        .map(|assertion| {
            let kind = assertion["predicate"]
                .as_object()
                .and_then(|predicate| predicate.keys().next().cloned())
                .unwrap_or_default();
            assert_eq!(assertion["valid_time"]["to"], Value::Null, "{assertion}");
            (kind, assertion["valid_time"]["from"].clone())
        })
        .collect();
    found.sort_by_key(|(kind, from)| format!("{kind} {from}"));
    found
}

/// `story:extraction-valid-time`, acceptance: through the verb and through the SDK over a child
/// session, a fact is valid from the observed time of the evidence it cites — the earliest, when it
/// cites several — with no end, a relation as a property is.
#[test]
fn an_extracted_fact_is_valid_from_its_evidence_observed_time() {
    let (early, middle, late) = (
        1_773_100_000_000_u64,
        1_773_273_600_000_u64,
        1_773_360_000_000_u64,
    );
    let facts = vec![
        legal_name("Vega", "Vega Holdings", &["400"]),
        legal_name("Lyra", "Lyra Group", &["401", "402"]),
        format!(
            "- !Relation\n  subject: {{node_type: Person, aliases: [Ann]}}\n  relation: CEO_OF\n  \
             object: {{node_type: Organization, aliases: [Lyra]}}\n  evidence: [{}]\n",
            evidence_id("401")
        ),
    ];
    let text = extraction(
        &facts,
        &[
            evidence_item("400", b"Vega Holdings, from its filing.", middle),
            evidence_item("401", b"Lyra Group and its chief executive, Ann.", late),
            evidence_item("402", b"Lyra Group, from an older register.", early),
        ],
    );
    for backend in BACKENDS {
        let (verb, sdk) = (World::new(backend, false), World::new(backend, false));
        let by_verb = verb.apply_verb(&text);
        let by_sdk = apply_decoded(&sdk, &text, &ApplyOptions::default()).unwrap();
        for (world, report) in [(&verb, &by_verb), (&sdk, &by_sdk)] {
            assert_eq!(
                report["rejected"],
                serde_json::json!([]),
                "{backend}: {report}"
            );
            assert_eq!(
                valid_from(world, "Vega"),
                [("Property".to_owned(), serde_json::json!(middle))],
                "{backend}"
            );
            assert_eq!(
                valid_from(world, "Lyra"),
                [("Property".to_owned(), serde_json::json!(early))],
                "{backend}: the earliest of the two items it cites"
            );
            assert_eq!(
                valid_from(world, "Ann"),
                [("Relation".to_owned(), serde_json::json!(late))],
                "{backend}"
            );
        }
        assert_eq!(
            verb.content(),
            sdk.content(),
            "{backend}: the two stores differ"
        );
    }
}

/// `story:extraction-valid-time`, decision 2: a store holding a claim as extraction wrote it before
/// valid time — valid from an unbounded past, citing the fact's evidence — holds the fact on a
/// later run, as `asserted`, rather than adding a second active assertion valid from the evidence's
/// observed time.
#[test]
fn a_claim_applied_before_valid_time_is_held_not_asserted_again() {
    use ekr_sdk::document::{Assertion, Object, Operation, Predicate, Subject, Value as Said};

    let property = "- !Property\n  subject: {node_type: Organization, aliases: [Globex]}\n  \
                    property: legal_name\n  value: {value_kind: String, value: Globex \
                    Corporation}\n  evidence: [00000000-0000-4000-8000-000000000403]\n";
    assert!(KNOWN_TYPES.contains(property));
    for backend in BACKENDS {
        let world = World::new(backend, true);
        world.apply_verb(&KNOWN_TYPES.replace(property, ""));
        let snapshot = world.store_json(&["snapshot"]);
        let graph = &snapshot["graph"]["graph"];
        let globex = graph["nodes"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(_, node)| node["canonical_name"] == "Globex")
            .map(|(id, _)| id.parse().unwrap())
            .unwrap();
        let unbounded = Assertion::new(
            graph["root"]["id"].as_str().unwrap().parse().unwrap(),
            Subject::Node(globex),
            Predicate::Property("00000000-0000-4000-8000-000000000801".parse().unwrap()),
            Object::Value(Said::String("Globex Corporation".to_owned())),
            OPERATOR.parse().unwrap(),
        )
        .citing("00000000-0000-4000-8000-000000000403".parse().unwrap());
        commit_by_hand(&world, vec![Operation::AddAssertion(Box::new(unbounded))]);

        let report = world.apply_verb(KNOWN_TYPES);
        assert_eq!(
            report["committed"],
            serde_json::json!([]),
            "{backend}: {report}"
        );
        assert_eq!(
            report["held"],
            serde_json::json!([
                {"item": "facts[0]", "reason": "asserted"},
                {"item": "facts[1]", "reason": "asserted"}
            ]),
            "{backend}: {report}"
        );
        assert_eq!(
            valid_from(&world, "Globex"),
            [("Property".to_owned(), Value::Null)],
            "{backend}"
        );
    }
}

/// `ekr mcp` over `world`, sent `calls` as `tools/call` requests after the handshake: each call's
/// structured result, in order.
fn mcp(world: &World, calls: &[(&str, Value)]) -> Vec<Value> {
    let mut lines = vec![
        serde_json::json!({"jsonrpc": "2.0", "id": 0, "method": "initialize", "params": {
            "protocolVersion": "2025-11-25",
            "capabilities": {},
            "clientInfo": {"name": "extraction-test", "version": "0"}
        }}),
        serde_json::json!({"jsonrpc": "2.0", "method": "notifications/initialized"}),
    ];
    for (at, (tool, arguments)) in calls.iter().enumerate() {
        lines.push(serde_json::json!({
            "jsonrpc": "2.0",
            "id": at + 1,
            "method": "tools/call",
            "params": {"name": tool, "arguments": arguments},
        }));
    }
    let mut input = String::new();
    for line in &lines {
        input.push_str(&line.to_string());
        input.push('\n');
    }
    let path = world.file("mcp-input.jsonl");
    std::fs::write(&path, input).unwrap();
    let output = world
        .command(&["mcp"])
        .stdin(std::fs::File::open(&path).unwrap())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let answers: BTreeMap<u64, Value> = String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str::<Value>(line).unwrap())
        .filter_map(|answer| Some((answer["id"].as_u64()?, answer)))
        .collect();
    (1..=calls.len() as u64)
        .map(|id| {
            let result = &answers[&id]["result"];
            assert_eq!(result["isError"], false, "{}", answers[&id]);
            result["structuredContent"].clone()
        })
        .collect()
}

/// Every object anywhere in `value` whose `name` is `name`.
fn objects_named<'a>(value: &'a Value, name: &str, into: &mut Vec<&'a Value>) {
    match value {
        Value::Object(fields) => {
            if fields.get("name").and_then(Value::as_str) == Some(name) {
                into.push(value);
            }
            fields
                .values()
                .for_each(|field| objects_named(field, name, into));
        }
        Value::Array(items) => items
            .iter()
            .for_each(|item| objects_named(item, name, into)),
        _ => {}
    }
}

/// The first number anywhere in `value` under the key `key`.
fn number_at(value: &Value, key: &str) -> Option<u64> {
    match value {
        Value::Object(fields) => fields
            .get(key)
            .and_then(Value::as_u64)
            .or_else(|| fields.values().find_map(|field| number_at(field, key))),
        Value::Array(items) => items.iter().find_map(|item| number_at(item, key)),
        _ => None,
    }
}

/// `story:extracted-relations-visible-to-graph-reads`, acceptance: a document with one `!Relation`
/// fact applies through the verb, and afterwards MCP `search` reports `degree` 1 for both ends and
/// `expand` from either end returns the other node and one edge. Over a child session the SDK
/// routine writes the same store.
#[test]
fn a_relation_applied_through_extraction_is_walked_by_graph_reads() {
    let facts = vec![format!(
        "- !Relation\n  subject: {{node_type: Person, aliases: [Rigel]}}\n  relation: CEO_OF\n  \
         object: {{node_type: Organization, aliases: [Betelgeuse]}}\n  evidence: [{}]\n",
        evidence_id("400")
    )];
    let text = extraction(
        &facts,
        &[evidence_item(
            "400",
            b"Rigel runs Betelgeuse.",
            1_773_273_600_000,
        )],
    );
    for backend in BACKENDS {
        let (verb, sdk) = (World::new(backend, false), World::new(backend, false));
        let report = verb.apply_verb(&text);
        assert_eq!(
            report["rejected"],
            serde_json::json!([]),
            "{backend}: {report}"
        );
        apply_decoded(&sdk, &text, &ApplyOptions::default()).unwrap();
        assert_eq!(
            verb.content(),
            sdk.content(),
            "{backend}: the two stores differ"
        );

        let searched = mcp(
            &verb,
            &[
                ("search", serde_json::json!({"text": "Rigel"})),
                ("search", serde_json::json!({"text": "Betelgeuse"})),
            ],
        );
        let mut ends = Vec::new();
        for (name, answer) in ["Rigel", "Betelgeuse"].into_iter().zip(&searched) {
            let mut found = Vec::new();
            objects_named(answer, name, &mut found);
            assert_eq!(found.len(), 1, "{backend}: {name} in {answer}");
            assert_eq!(found[0]["degree"], 1, "{backend}: {name} in {answer}");
            ends.push(found[0]["id"].as_str().unwrap().to_owned());
        }
        for (seed, other) in [(&ends[0], "Betelgeuse"), (&ends[1], "Rigel")] {
            let expanded = mcp(
                &verb,
                &[(
                    "expand",
                    serde_json::json!({"seeds": [seed], "depth": 1, "limit": 50}),
                )],
            );
            let answer = &expanded[0];
            assert_eq!(
                number_at(answer, "node_total"),
                Some(2),
                "{backend}: {answer}"
            );
            assert_eq!(
                number_at(answer, "edge_total"),
                Some(1),
                "{backend}: {answer}"
            );
            let mut found = Vec::new();
            objects_named(answer, other, &mut found);
            assert!(!found.is_empty(), "{backend}: {other} in {answer}");
        }
    }
}

/// `story:extracted-relations-visible-to-graph-reads`, coordinator decision: the edge goes in its
/// assertion's group, so an edge the kernel refuses — here a second `CEO_OF`, whose cardinality is
/// `One`, out of one person — refuses that fact, assertion and all, with the validator's issue. The
/// same relation read again from new evidence is asserted and adds no second edge, so it is not
/// refused by that cardinality.
#[test]
fn an_edge_the_kernel_refuses_refuses_its_fact_and_a_corroborated_relation_adds_no_edge() {
    let relation = |object: &str, cited: &str| {
        extraction(
            &[format!(
                "- !Relation\n  subject: {{node_type: Person, aliases: [Rigel]}}\n  relation: \
                 CEO_OF\n  object: {{node_type: Organization, aliases: [{object}]}}\n  evidence: \
                 [{}]\n",
                evidence_id(cited)
            )],
            &[evidence_item(
                cited,
                format!("Rigel runs {object}, source {cited}.").as_bytes(),
                1_773_273_600_000,
            )],
        )
    };
    let edges = |world: &World| {
        world.store_json(&["snapshot"])["graph"]["graph"]["edges"]
            .as_object()
            .unwrap()
            .len()
    };
    for backend in BACKENDS {
        let world = World::new(backend, false);
        let seeded = edges(&world);
        let first = world.apply_verb(&relation("Betelgeuse", "400"));
        assert_eq!(
            first["rejected"],
            serde_json::json!([]),
            "{backend}: {first}"
        );
        assert_eq!(edges(&world), seeded + 1, "{backend}");

        let again = world.apply_verb(&relation("Betelgeuse", "401"));
        assert_eq!(
            again["rejected"],
            serde_json::json!([]),
            "{backend}: {again}"
        );
        assert_eq!(again["held"], serde_json::json!([]), "{backend}: {again}");
        assert_eq!(edges(&world), seeded + 1, "{backend}: no second edge");
        assert_eq!(
            valid_from(&world, "Rigel").len(),
            2,
            "{backend}: two assertions of the claim, one per source"
        );

        let second = world.apply_verb(&relation("Mintaka", "402"));
        let rejected = &second["rejected"];
        assert_eq!(rejected.as_array().unwrap().len(), 1, "{backend}: {second}");
        assert_eq!(rejected[0]["item"], "facts[0]", "{backend}: {second}");
        assert!(
            rejected[0]["issues"]
                .as_array()
                .unwrap()
                .iter()
                .any(|issue| issue["code"] == "edge-cardinality"),
            "{backend}: {second}"
        );
        assert_eq!(edges(&world), seeded + 1, "{backend}");
        assert_eq!(
            valid_from(&world, "Rigel").len(),
            2,
            "{backend}: the assertion fell too"
        );
    }
}

/// Every assertion of `world` about the node named `name` with the `!Property` `property`, as
/// `(lifecycle, object, evidence, valid_time, id)`, sorted by object.
fn about(world: &World, name: &str, property: &str) -> Vec<(Value, Value, Value, Value, String)> {
    let snapshot = world.store_json(&["snapshot"]);
    let graph = &snapshot["graph"]["graph"];
    let node = graph["nodes"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, node)| node["canonical_name"] == name)
        .map(|(id, _)| id.clone())
        .unwrap_or_else(|| panic!("no node {name}"));
    let mut found: Vec<(Value, Value, Value, Value, String)> = graph["assertions"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(_, assertion)| {
            assertion["subject"]["Node"] == node.as_str()
                && assertion["predicate"]["Property"] == property
        })
        .map(|(id, assertion)| {
            (
                assertion["lifecycle"].clone(),
                assertion["object"].clone(),
                assertion["evidence"].clone(),
                assertion["valid_time"].clone(),
                id.clone(),
            )
        })
        .collect();
    found.sort_by_key(|row| row.1.to_string());
    found
}

/// `story:extraction-supersession`, acceptance: a `!Property` fact marked `replaces: true`
/// replaces the active value of the same subject and property. Through the verb and through the
/// SDK over a child session it applies one `SupersedeAssertion`: afterwards exactly one assertion
/// of that subject and property is active, carrying the new value, valid from the new evidence's
/// observed time, and the old one is superseded by it from then. A replacement naming no active
/// assertion is refused for that fact alone, with a reason, and the rest of the document applies.
#[test]
fn a_replacing_fact_supersedes_the_active_value() {
    const LEGAL_NAME: &str = "00000000-0000-4000-8000-000000000801";
    assert_eq!(
        ekr_sdk::extraction::REPLACES_NOTHING,
        "replacement-without-active-assertion",
        "the code is part of the report's contract"
    );
    let from = 1_775_001_600_000_u64;
    let replacing = extraction(
        &[
            legal_name("Globex", "Globex Inc", &["410"]).replace(
                "  property: legal_name\n",
                "  property: legal_name\n  replaces: true\n",
            ),
            legal_name("Nova", "Nova Ltd", &["410"]).replace(
                "  property: legal_name\n",
                "  property: legal_name\n  replaces: true\n",
            ),
            legal_name("Vela", "Vela Partners", &["410"]),
        ],
        &[evidence_item(
            "410",
            b"Globex Corporation is now Globex Inc; Vela Partners registered.",
            from,
        )],
    );
    for backend in BACKENDS {
        let (verb, sdk) = (World::new(backend, false), World::new(backend, false));
        verb.apply_verb(KNOWN_TYPES);
        sdk.apply_verb(KNOWN_TYPES);
        let old: Vec<String> = [&verb, &sdk]
            .iter()
            .map(|world| {
                let rows = about(world, "Globex", LEGAL_NAME);
                assert_eq!(rows.len(), 1, "{backend}: {rows:?}");
                rows[0].4.clone()
            })
            .collect();

        let by_verb = verb.apply_verb(&replacing);
        let by_sdk = apply_decoded(&sdk, &replacing, &ApplyOptions::default()).unwrap();
        for ((world, report), old) in [(&verb, &by_verb), (&sdk, &by_sdk)].into_iter().zip(&old) {
            let rejected = report["rejected"].as_array().unwrap();
            assert_eq!(rejected.len(), 1, "{backend}: {report}");
            assert_eq!(rejected[0]["item"], "facts[1]", "{backend}: {report}");
            assert!(
                rejected[0]["refusal"].as_str().is_some_and(|refusal| {
                    refusal.starts_with(&format!(
                        "{}: facts[1]",
                        ekr_sdk::extraction::REPLACES_NOTHING
                    ))
                }),
                "{backend}: {report}"
            );
            assert_eq!(report["held"], serde_json::json!([]), "{backend}: {report}");

            let rows = about(world, "Globex", LEGAL_NAME);
            let active: Vec<_> = rows.iter().filter(|row| row.0 == "Active").collect();
            assert_eq!(active.len(), 1, "{backend}: {rows:?}");
            let (_, object, evidence, valid_time, new) = active[0];
            assert_eq!(
                object["Value"]["value"], "Globex Inc",
                "{backend}: {rows:?}"
            );
            assert_eq!(
                evidence,
                &serde_json::json!([evidence_id("410")]),
                "{backend}"
            );
            assert_eq!(valid_time["from"], from, "{backend}: {rows:?}");
            let superseded: Vec<_> = rows.iter().filter(|row| row.0 != "Active").collect();
            assert_eq!(superseded.len(), 1, "{backend}: {rows:?}");
            assert_eq!(&superseded[0].4, old, "{backend}: the old one");
            assert_eq!(
                superseded[0].0["Superseded"]["by"],
                new.as_str(),
                "{backend}: {rows:?}"
            );
            assert_eq!(
                superseded[0].0["Superseded"]["effective_from"], from,
                "{backend}: {rows:?}"
            );
            assert_eq!(superseded[0].3["to"], from, "{backend}: {rows:?}");
            assert_eq!(
                about(world, "Vela", LEGAL_NAME).len(),
                1,
                "{backend}: the rest applies"
            );
        }
        // The stores are not compared whole: a superseded assertion names its replacement by id,
        // which differs between them. Each was checked above.

        // Applied again, the replacement is the active claim: held, nothing superseded.
        let again = verb.apply_verb(&replacing);
        assert_eq!(
            again["held"],
            serde_json::json!([
                {"item": "facts[0]", "reason": "asserted"},
                {"item": "facts[2]", "reason": "asserted"}
            ]),
            "{backend}: {again}"
        );
        assert_eq!(about(&verb, "Globex", LEGAL_NAME).len(), 2, "{backend}");
    }
}

/// `story:extraction-supersession`: the marker is part of the document. The engine's reader and
/// the SDK's mirror both read `replaces: true`, the mirror writes it back, and a fact without it
/// is written as before.
#[test]
fn both_readers_read_the_replacement_marker() {
    let text = extraction(
        &[legal_name("Globex", "Globex Inc", &["410"]).replace(
            "  property: legal_name\n",
            "  property: legal_name\n  replaces: true\n",
        )],
        &[evidence_item("410", b"Globex Inc.", 1_775_001_600_000)],
    );
    let mirrored = ExtractionDocument::from_yaml(&text).unwrap();
    match &mirrored.facts[0] {
        ExtractedFact::Property(fact) => assert!(fact.replaces),
        other => panic!("a property fact: {other:?}"),
    }
    let written = mirrored.to_yaml().unwrap();
    assert!(written.contains("replaces: true"), "{written}");
    let read = ekr_integrate::ExtractionDocument::from_yaml(&written).unwrap();
    match &read.facts[0] {
        ekr_integrate::ExtractedFact::Property(fact) => assert!(fact.replaces),
        other => panic!("a property fact: {other:?}"),
    }
    let plain = ExtractionDocument::from_yaml(KNOWN_TYPES).unwrap();
    assert!(!plain.to_yaml().unwrap().contains("replaces"));
}

/// The id of the node of `world` whose canonical name is `name`, if there is one.
fn node(world: &World, name: &str) -> Option<String> {
    graph(world)["nodes"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, node)| node["canonical_name"] == name)
        .map(|(id, _)| id.clone())
}

/// The canonical graph of `world`.
fn graph(world: &World) -> Value {
    world.store_json(&["snapshot"])["graph"]["graph"].clone()
}

/// The values of the active `legal_name` assertions of the node of `world` named `name`, sorted.
fn active_legal_names(world: &World, name: &str) -> Vec<String> {
    const LEGAL_NAME: &str = "00000000-0000-4000-8000-000000000801";
    let node = node(world, name).unwrap_or_else(|| panic!("no node {name}"));
    let mut found: Vec<String> = graph(world)["assertions"]
        .as_object()
        .unwrap()
        .values()
        .filter(|assertion| {
            assertion["lifecycle"] == "Active"
                && assertion["subject"]["Node"] == node.as_str()
                && assertion["predicate"]["Property"] == LEGAL_NAME
        })
        .map(|assertion| assertion["object"]["Value"]["value"].to_string())
        .collect();
    found.sort();
    found
}

/// The SDK's routine, default options, over a child session, on a document the mirror decodes
/// without its own checks: the report.
fn apply_sdk_decoded(world: &World, document: &str) -> Value {
    apply_decoded(world, document, &ApplyOptions::default())
        .unwrap_or_else(|error| panic!("the SDK's apply failed: {error}"))
}

/// The same fact, marked `replaces: true`.
fn replacing(fact: &str) -> String {
    fact.replace(
        "  property: legal_name\n",
        "  property: legal_name\n  replaces: true\n",
    )
}

/// A `!Relation` fact: Person `subject` is `CEO_OF` Organization `object`, citing `cited`.
fn ceo_of(subject: &str, object: &str, cited: &str) -> String {
    format!(
        "- !Relation\n  subject: {{node_type: Person, aliases: [{subject}]}}\n  relation: CEO_OF\n  \
         object: {{node_type: Organization, aliases: [{object}]}}\n  evidence: [{}]\n",
        evidence_id(cited)
    )
}

/// Each report row's `item`, in order.
fn items(rows: &Value) -> Vec<String> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|row| row["item"].as_str().unwrap().to_owned())
        .collect()
}

/// Partial apply, decision 4 ("a skipped fact creates nothing that only it introduced"), through
/// the mechanism supersession decision 2 says a refused replacement uses: a `replaces` fact about
/// an organisation the store does not hold is refused as `replacement-without-active-assertion`,
/// and no node is created for the organisation only it names. Verb and SDK alike.
#[test]
fn a_replacement_refused_for_nothing_to_replace_creates_no_node() {
    let text = extraction(
        &[
            replacing(&legal_name("Nova", "Nova Ltd", &["410"])),
            legal_name("Vela", "Vela Partners", &["410"]),
        ],
        &[evidence_item(
            "410",
            b"Nova Ltd and Vela Partners, newly registered.",
            1_775_001_600_000,
        )],
    );
    for backend in BACKENDS {
        for path in ["verb", "sdk"] {
            let world = World::new(backend, false);
            let report = if path == "verb" {
                world.apply_verb(&text)
            } else {
                apply_sdk_decoded(&world, &text)
            };
            assert_eq!(
                items(&report["rejected"]),
                ["facts[0]"],
                "{backend}/{path}: {report}"
            );
            assert!(
                report["rejected"][0]["refusal"]
                    .as_str()
                    .is_some_and(|refusal| refusal
                        .starts_with("replacement-without-active-assertion: facts[0]")),
                "{backend}/{path}: {report}"
            );
            assert!(node(&world, "Vela").is_some(), "{backend}/{path}");
            assert_eq!(
                node(&world, "Nova"),
                None,
                "{backend}/{path}: the node only the refused replacement names was created"
            );
        }
    }
}

/// Supersession: a replacing fact the kernel refuses (its evidence predates the active value, so
/// `invalid-supersession`) must not take a later, valid replacement of the same subject and
/// property down with it. The later fact alone supersedes the store's value; applied together,
/// it is refused because it is made to supersede the first fact's assertion, which never
/// committed. A fact valid alone fails for one rejected beside it.
#[test]
fn a_valid_replacement_is_not_refused_for_a_refused_one_before_it() {
    let text = extraction(
        &[
            replacing(&legal_name("Globex", "Globex Old", &["411"])),
            replacing(&legal_name("Globex", "Globex Inc", &["412"])),
        ],
        &[
            evidence_item(
                "411",
                b"Globex, as a 2023 register lists it.",
                1_700_000_000_000,
            ),
            evidence_item(
                "412",
                b"Globex Corporation is now Globex Inc.",
                1_775_001_600_000,
            ),
        ],
    );
    for backend in BACKENDS {
        let world = World::new(backend, false);
        world.apply_verb(KNOWN_TYPES);
        assert_eq!(
            active_legal_names(&world, "Globex"),
            ["\"Globex Corporation\""],
            "{backend}"
        );
        let report = world.apply_verb(&text);
        assert_eq!(
            items(&report["rejected"]),
            ["facts[0]"],
            "{backend}: only the replacement older than the active value is refused: {report}"
        );
        assert_eq!(
            active_legal_names(&world, "Globex"),
            ["\"Globex Inc\""],
            "{backend}: {report}"
        );
    }
}

/// Edges: the coordinator decision is that an edge the kernel refuses refuses its fact,
/// "assertion and all", and `integrate.yaml` says a relation is written as a `CreateEdge` "unless
/// an edge of its type already joins the two nodes". Here the first `CEO_OF` out of Rigel holds
/// (cardinality `One`), and a document reads Rigel `CEO_OF` Mintaka twice, from two sources. Both
/// facts must be refused, as either is alone; instead the second is asserted with no edge, since
/// the first fact's refused edge counts as joining the nodes — a relation graph reads cannot see.
#[test]
fn a_relation_read_twice_after_its_edge_was_refused_is_not_asserted_without_an_edge() {
    let first = extraction(
        &[ceo_of("Rigel", "Betelgeuse", "400")],
        &[evidence_item(
            "400",
            b"Rigel runs Betelgeuse.",
            1_773_273_600_000,
        )],
    );
    let twice = extraction(
        &[
            ceo_of("Rigel", "Mintaka", "402"),
            ceo_of("Rigel", "Mintaka", "403"),
        ],
        &[
            evidence_item(
                "402",
                b"Rigel runs Mintaka, says one source.",
                1_773_273_600_000,
            ),
            evidence_item(
                "403",
                b"Rigel runs Mintaka, says another.",
                1_773_360_000_000,
            ),
        ],
    );
    for backend in BACKENDS {
        let world = World::new(backend, false);
        let report = world.apply_verb(&first);
        assert_eq!(
            report["rejected"],
            serde_json::json!([]),
            "{backend}: {report}"
        );
        let report = world.apply_verb(&twice);

        let graph = graph(&world);
        let rigel = node(&world, "Rigel").unwrap();
        let edges: Vec<(String, String, String)> = graph["edges"]
            .as_object()
            .unwrap()
            .values()
            .map(|edge| {
                (
                    edge["source"].as_str().unwrap().to_owned(),
                    edge["type_id"].as_str().unwrap().to_owned(),
                    edge["target"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        for assertion in graph["assertions"].as_object().unwrap().values() {
            if assertion["lifecycle"] != "Active" || assertion["subject"]["Node"] != rigel.as_str()
            {
                continue;
            }
            let Some(relation) = assertion["predicate"]["Relation"].as_str() else {
                continue;
            };
            let target = assertion["object"]["Node"].as_str().unwrap();
            assert!(
                edges.contains(&(rigel.clone(), relation.to_owned(), target.to_owned())),
                "{backend}: an active relation assertion of Rigel with no edge for graph reads: \
                 {assertion}\nreport: {report}"
            );
        }
        assert_eq!(
            items(&report["rejected"]),
            ["facts[0]", "facts[1]"],
            "{backend}: {report}"
        );
    }
}

/// Valid time: before this unit a fact citing part of the evidence an earlier fact of the same
/// claim cites was held (`repeated` in one document, `asserted` across documents). Now each is
/// keyed by its own earliest `observed_at`, so a fact citing the later item alone is asserted a
/// second time: two active assertions of one value, from one source, on every re-read of it.
#[test]
fn a_fact_citing_part_of_an_earlier_claims_evidence_is_still_held() {
    let evidence = [
        evidence_item(
            "401",
            b"Lyra Group, from an older register.",
            1_773_100_000_000,
        ),
        evidence_item("402", b"Lyra Group, from its filing.", 1_773_360_000_000),
    ];
    let both = legal_name("Lyra", "Lyra Group", &["401", "402"]);
    let later = legal_name("Lyra", "Lyra Group", &["402"]);
    for backend in BACKENDS {
        let world = World::new(backend, false);
        let report = world.apply_verb(&extraction(&[both.clone(), later.clone()], &evidence));
        assert_eq!(
            report["held"],
            serde_json::json!([{"item": "facts[1]", "reason": "repeated"}]),
            "{backend}: {report}"
        );
        assert_eq!(
            active_legal_names(&world, "Lyra"),
            ["\"Lyra Group\""],
            "{backend}: {report}"
        );

        // Across documents, as a consumer re-reading an overlapping batch does.
        let again = World::new(backend, false);
        again.apply_verb(&extraction(std::slice::from_ref(&both), &evidence));
        let report = again.apply_verb(&extraction(std::slice::from_ref(&later), &evidence));
        assert_eq!(
            report["held"],
            serde_json::json!([{"item": "facts[0]", "reason": "asserted"}]),
            "{backend}: {report}"
        );
    }
}

/// Partial apply, decision 3 ("on the CLI and the SDK path alike") and decision 4: a relation
/// between types its edge type does not connect is skipped by the verb as
/// `extraction-relation-ends` and creates nothing. Over a child session the SDK routine does not
/// check the ends before it writes: it resolves and creates both named things, then the kernel's
/// type validator rejects the assertion and its edge (`edge-endpoint-type`) — so the fact is
/// rejected there too, but the person only that fact names has already been created.
#[test]
fn on_the_sdk_path_a_relation_refused_for_its_ends_creates_no_node() {
    let text = extraction(
        &[
            format!(
                "- !Relation\n  subject: {{node_type: Organization, aliases: [Zenith3]}}\n  \
                 relation: CEO_OF\n  object: {{node_type: Person, aliases: [Dora]}}\n  \
                 evidence: [{}]\n",
                evidence_id("400")
            ),
            legal_name("Zenith5", "Zenith Five", &["400"]),
        ],
        &[evidence_item(
            "400",
            b"Zenith and its sister companies.",
            1_773_273_600_000,
        )],
    );
    for backend in BACKENDS {
        let verb = World::new(backend, false);
        let by_verb = verb.apply_verb(&text);
        assert_eq!(
            items(&by_verb["rejected"]),
            ["facts[0]"],
            "{backend}: {by_verb}"
        );
        assert_eq!(node(&verb, "Dora"), None, "{backend}: verb");

        let sdk = World::new(backend, false);
        let by_sdk = apply_sdk_decoded(&sdk, &text);
        assert_eq!(
            items(&by_sdk["rejected"]),
            ["facts[0]"],
            "{backend}: {by_sdk}"
        );
        assert_eq!(
            node(&sdk, "Dora"),
            None,
            "{backend}: the SDK path created the person only the refused relation names: {by_sdk}"
        );
    }
}

/// Supersession plus a held claim: the store holds two active legal names for Globex (the second
/// read without `replaces`). A document marks "Globex Corporation" as replacing the active value,
/// from the evidence that already asserted it. The fact is held (`asserted`) before replacement is
/// considered, so nothing is superseded and two values stay active, though the document says the
/// one value replaces the others.
#[test]
fn a_held_replacement_still_leaves_one_active_value() {
    let second = extraction(
        &[legal_name("Globex", "Globex Inc", &["410"])],
        // Observed before the store's value, so superseding it from 1773273600000 is valid.
        &[evidence_item(
            "410",
            b"Globex Inc, says an older source.",
            1_773_100_000_000,
        )],
    );
    let replacing_known = KNOWN_TYPES.replace(
        "  property: legal_name\n",
        "  property: legal_name\n  replaces: true\n",
    );
    assert_ne!(replacing_known, KNOWN_TYPES);
    for backend in BACKENDS {
        let world = World::new(backend, false);
        world.apply_verb(KNOWN_TYPES);
        world.apply_verb(&second);
        assert_eq!(
            active_legal_names(&world, "Globex"),
            ["\"Globex Corporation\"", "\"Globex Inc\""],
            "{backend}"
        );
        let report = world.apply_verb(&replacing_known);
        assert_eq!(
            active_legal_names(&world, "Globex").len(),
            1,
            "{backend}: a replacing fact left {:?} active: {report}",
            active_legal_names(&world, "Globex")
        );
    }
}

/// What a run records as done changes only when a fact's group commits: a fact citing the same
/// evidence as an earlier fact the kernel rejected is tried on its own and reported on its own
/// result, never held as `repeated`, since nothing was asserted. Here both read Rigel `CEO_OF`
/// Mintaka from one item while Rigel already runs Betelgeuse (cardinality `One`).
#[test]
fn a_fact_repeating_a_rejected_facts_evidence_is_tried_not_held() {
    let first = extraction(
        &[ceo_of("Rigel", "Betelgeuse", "400")],
        &[evidence_item(
            "400",
            b"Rigel runs Betelgeuse.",
            1_773_273_600_000,
        )],
    );
    let twice = extraction(
        &[
            ceo_of("Rigel", "Mintaka", "402"),
            ceo_of("Rigel", "Mintaka", "402"),
        ],
        &[evidence_item(
            "402",
            b"Rigel runs Mintaka, says one source.",
            1_773_273_600_000,
        )],
    );
    for backend in BACKENDS {
        let world = World::new(backend, false);
        world.apply_verb(&first);
        let report = world.apply_verb(&twice);
        assert_eq!(report["held"], serde_json::json!([]), "{backend}: {report}");
        assert_eq!(
            items(&report["rejected"]),
            ["facts[0]", "facts[1]"],
            "{backend}: {report}"
        );
        for row in report["rejected"].as_array().unwrap() {
            assert!(
                row["issues"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|issue| issue["code"] == "edge-cardinality"),
                "{backend}: {report}"
            );
        }
        // Only the node Mintaka committed; Rigel still has one relation, to Betelgeuse.
        let rigel = node(&world, "Rigel").unwrap();
        let relations = graph(&world)["assertions"]
            .as_object()
            .unwrap()
            .values()
            .filter(|assertion| {
                assertion["lifecycle"] == "Active"
                    && assertion["subject"]["Node"] == rigel.as_str()
                    && assertion["predicate"]["Relation"].is_string()
            })
            .count();
        assert_eq!(relations, 1, "{backend}: {report}");
    }
}

/// Report counts: a held replacement still supersedes the other active values, in a group of its
/// own. When validation rejects that group — here the other value starts after the held one, so
/// superseding it from the held one's start is `invalid-supersession` — the fact is listed only
/// under `rejected`: a fact has one outcome in the report.
#[test]
fn a_held_replacement_whose_supersession_is_rejected_is_reported_once() {
    let newer = extraction(
        &[legal_name("Globex", "Globex Inc", &["410"])],
        &[evidence_item(
            "410",
            b"Globex Inc, says a newer source.",
            1_775_001_600_000,
        )],
    );
    let replacing_known = KNOWN_TYPES.replace(
        "  property: legal_name\n",
        "  property: legal_name\n  replaces: true\n",
    );
    assert_ne!(replacing_known, KNOWN_TYPES);
    for backend in BACKENDS {
        let world = World::new(backend, false);
        world.apply_verb(KNOWN_TYPES);
        world.apply_verb(&newer);
        assert_eq!(
            active_legal_names(&world, "Globex"),
            ["\"Globex Corporation\"", "\"Globex Inc\""],
            "{backend}"
        );
        let report = world.apply_verb(&replacing_known);
        let outcomes: Vec<String> = items(&report["held"])
            .into_iter()
            .chain(items(&report["rejected"]))
            .filter(|item| item == "facts[0]")
            .collect();
        assert_eq!(
            outcomes.len(),
            1,
            "{backend}: facts[0] has {} outcomes in the report: {report}",
            outcomes.len()
        );
    }
}

/// Rounds: facts making one claim from different evidence do not depend on each other — none can
/// be held by another, since none cites all of another's evidence — so they commit in the same
/// round. Folded from a 200-fact case (201 transactions before the fix, one per round), cut to 20
/// facts for time: the new node, then the facts, in at most 3 transactions. Counted, not timed.
#[test]
fn twenty_corroborating_facts_of_one_claim_commit_in_at_most_three_transactions() {
    let count = 20_usize;
    let suffix = |n: usize| format!("{}", 500 + n);
    let facts: Vec<String> = (0..count)
        .map(|n| legal_name("Polaris", "Polaris Ltd", &[suffix(n).as_str()]))
        .collect();
    let evidence: Vec<String> = (0..count)
        .map(|n| {
            evidence_item(
                &suffix(n),
                format!("Polaris Ltd, as source {n} names it.").as_bytes(),
                1_773_273_600_000 + n as u64,
            )
        })
        .collect();
    let world = World::new("file", false);
    let report = world.apply_verb(&extraction(&facts, &evidence));
    assert_eq!(report["rejected"], serde_json::json!([]), "{report}");
    assert_eq!(report["held"], serde_json::json!([]), "{report}");
    assert_eq!(
        active_legal_names(&world, "Polaris").len(),
        count,
        "every fact asserted"
    );
    let transactions = report["committed"].as_array().unwrap().len();
    assert!(
        transactions <= 3,
        "{transactions} transactions committed for {count} corroborating facts: {report}"
    );
}

/// `--strict` and its documentation: `docs/cli.md` says a replacement with nothing to replace is
/// a row of the report "with or without `--strict`", and the verb behaves so — it exits 0 and
/// writes the rest. `ekr apply-extraction --help` must not promise nothing is written when
/// something is.
#[test]
fn strict_help_and_strict_behaviour_agree_on_a_replacement_with_nothing_to_replace() {
    let text_of = extraction(
        &[
            replacing(&legal_name("Nova", "Nova Ltd", &["410"])),
            legal_name("Vela", "Vela Partners", &["410"]),
        ],
        &[evidence_item(
            "410",
            b"Nova Ltd and Vela Partners, newly registered.",
            1_775_001_600_000,
        )],
    );
    let help = text(&["apply-extraction", "--help"]);
    let world = World::new("file", false);
    let path = world.file("extraction.yaml");
    std::fs::write(&path, &text_of).unwrap();
    let output = world.store_output(&["apply-extraction", "--strict", path.to_str().unwrap()]);
    let written = world.store_json(&["head"])["revision"] != 0;
    if output.status.code() == Some(0) && written {
        let strict = help
            .split("--strict")
            .nth(1)
            .unwrap_or_default()
            .to_lowercase();
        assert!(
            !strict.contains("nothing written") || strict.contains("replac"),
            "--strict exited 0 and wrote (Vela: {:?}), but --help says:\n{help}",
            node(&world, "Vela")
        );
    } else {
        assert_eq!(output.status.code(), Some(2));
        assert!(!written);
    }
}
