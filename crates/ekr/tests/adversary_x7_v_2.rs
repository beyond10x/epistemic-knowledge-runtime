//! Adversarial pass 2 on `story:extraction-verb-shares-the-sdk-path` (wave correct-07, unit V).
//!
//! The fixes of pass 1 decided four things: a fact the store already asserts is `held`; named
//! things sharing an alias are one group, named by the least first alias; a failure after a
//! commit ends the run with `stopped`; and the SDK mirror refuses what the reader refuses while
//! decoding. Each is driven here against what it claims: the mirror against the reader on decode
//! refusals it does not share code for, `stopped` against a failure inside the facts batch, `held`
//! against a store that moved on between two runs, and the verb against documents the reader
//! accepts and the mirror or the ontology planner does not.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::process::Output;

use ekr_sdk::batch::Batcher;
use ekr_sdk::binary::EkrBinary;
use ekr_sdk::document::{AgentId, AssertionId, ExtractionDocument, Operation, Retraction};
use ekr_sdk::extraction::apply;
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use serde_json::Value;

const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";

/// Item 0403's entry and bytes, as the unit's own fixture writes them.
const ITEM_0403: &str = "- evidence:
    id: 00000000-0000-4000-8000-000000000403
    source: !HumanStatement
      identity: Quarterly report
    content_hash: 7afeb9c852d895a2cdf8d7717a49354badd8b8dc79049925137b1f95f4651af5
    extracted_by: 00000000-0000-4000-8000-000000000101
    observed_at: 1773273600000
    confidence: 10000
  payload: [67, 97, 114, 111, 108, 44, 32, 67, 69, 79, 32, 111, 102, 32, 71, 108, 111, 98, 101, 120, 32, 67, 111, 114, 112, 111, 114, 97, 116, 105, 111, 110, 44, 32, 108, 101, 97, 100, 115, 32, 112, 114, 111, 106, 101, 99, 116, 32, 65, 112, 111, 108, 108, 111, 46]
";

/// The unit's own fixture: Carol and Globex are new, both facts cite item 0403.
fn known_types() -> String {
    format!(
        "format: ekr.extraction-document/1
entities:
- node_type: Person
  aliases: [Carol]
facts:
- !Property
  subject: {{node_type: Organization, aliases: [Globex]}}
  property: legal_name
  value: {{value_kind: String, value: Globex Corporation}}
  evidence: [00000000-0000-4000-8000-000000000403]
- !Relation
  subject: {{node_type: Person, aliases: [Carol]}}
  relation: CEO_OF
  object: {{node_type: Organization, aliases: [Globex]}}
  evidence: [00000000-0000-4000-8000-000000000403]
evidence:
{ITEM_0403}"
    )
}

/// A second evidence item, 0404, with its own bytes and their hash.
fn item_0404() -> String {
    let payload = b"Globex Corporation, as the registry lists it.".to_vec();
    let hash = ekr_sdk::document::payload_hash(&payload);
    let bytes: Vec<String> = payload.iter().map(u8::to_string).collect();
    format!(
        "- evidence:
    id: 00000000-0000-4000-8000-000000000404
    source: !HumanStatement
      identity: Registry extract
    content_hash: {hash}
    extracted_by: 00000000-0000-4000-8000-000000000101
    observed_at: 1773360000000
    confidence: 9000
  payload: [{}]
",
        bytes.join(", ")
    )
}

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ekr"))
}

fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(binary());
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

fn text(args: &[&str]) -> String {
    let output = ekr().args(args).output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{args:?}");
    String::from_utf8(output.stdout).unwrap()
}

fn operator() -> AgentId {
    OPERATOR.parse().unwrap()
}

struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    /// The example host under profile v2, so a schema change is admitted, and the example seed.
    fn new(backend: &'static str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        let host = text(&["example", "ekr.cli-host/1"])
            .replace("\"ekr.p1-deterministic/1\"", "\"ekr.p2-deterministic/1\"")
            .replace("\"ekr.p1-apply/1\"", "\"ekr.p2-apply/1\"");
        std::fs::write(world.file("host.json"), host).unwrap();
        std::fs::write(world.file("seed.yaml"), text(&["example", "ekr-seed/2"])).unwrap();
        let seed = world.file("seed.yaml");
        world.json(&["seed", seed.to_str().unwrap()]);
        world
    }

    fn file(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn config(&self) -> StoreConfig {
        StoreConfig {
            host: self.file("host.json"),
            store: self.file("store"),
            backend: match self.backend {
                "file" => Backend::File,
                _ => Backend::Sqlite,
            },
        }
    }

    fn output(&self, args: &[&str]) -> Output {
        ekr()
            .arg("--host")
            .arg(self.file("host.json"))
            .arg("--store")
            .arg(self.file("store"))
            .args(["--backend", self.backend])
            .args(args)
            .output()
            .unwrap()
    }

    fn json(&self, args: &[&str]) -> Value {
        let output = self.output(args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn verb_output(&self, document: &str) -> Output {
        let path = self.file("extraction.yaml");
        std::fs::write(&path, document).unwrap();
        self.output(&["apply-extraction", path.to_str().unwrap()])
    }

    fn verb(&self, document: &str) -> Value {
        let output = self.verb_output(document);
        assert_eq!(
            output.status.code(),
            Some(0),
            "apply-extraction: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn session(&self) -> ProcessSession {
        let binary = EkrBinary::open(binary()).unwrap();
        ProcessSession::start(&binary, self.config(), SessionOptions::default()).unwrap()
    }

    fn revision(&self) -> u64 {
        self.json(&["head"])["revision"].as_u64().unwrap()
    }

    fn graph(&self) -> Value {
        self.json(&["snapshot"])["graph"]["graph"].clone()
    }

    /// The active assertions, each as what it says: its object and evidence, ids of nodes
    /// replaced by their names.
    fn active(&self) -> Vec<String> {
        let graph = self.graph();
        let names: BTreeMap<String, String> = graph["nodes"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(id, node)| {
                (
                    id.clone(),
                    node["canonical_name"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        let mut said: Vec<String> = graph["assertions"]
            .as_object()
            .unwrap()
            .values()
            .filter(|assertion| assertion["lifecycle"] == "Active")
            .map(|assertion| {
                let mut line = format!(
                    "{} {} {} {}",
                    assertion["subject"],
                    assertion["predicate"],
                    assertion["object"],
                    assertion["evidence"]
                );
                for (id, name) in &names {
                    line = line.replace(id, name);
                }
                line
            })
            .collect();
        said.sort();
        said
    }

    /// The id of the one active assertion whose object mentions `needle`.
    fn assertion_saying(&self, needle: &str) -> AssertionId {
        let graph = self.graph();
        let found: Vec<&Value> = graph["assertions"]
            .as_object()
            .unwrap()
            .values()
            .filter(|assertion| {
                assertion["lifecycle"] == "Active"
                    && assertion["object"].to_string().contains(needle)
            })
            .collect();
        assert_eq!(found.len(), 1, "{found:?}");
        found[0]["id"].as_str().unwrap().parse().unwrap()
    }
}

/// The reader refuses `document` while decoding, as `code`; the SDK's mirror must refuse it too.
fn mirror_refuses(document: &str, code: &str) {
    let reader = ekr_integrate::ExtractionDocument::from_yaml(document)
        .expect_err("the reader refuses the document");
    assert_eq!(reader.refusal().code.code(), code, "{reader}");
    let mirror = ExtractionDocument::from_yaml(document);
    assert_eq!(
        mirror
            .as_ref()
            .map(|_| ())
            .map_err(|refusal| refusal.code.as_str()),
        Err(code),
        "the reader refuses as {code} ({reader}); the mirror answers {mirror:?}"
    );
}

// --- 4. The mirror and the reader on every decode refusal ---------------------------------------

/// A confidence past ten thousand basis points: the reader's `Confidence` is `try_from = "u16"`
/// and refuses it while decoding; the SDK's is `transparent` and reads it.
#[test]
fn adv2_mirror_refuses_a_confidence_past_certain() {
    mirror_refuses(
        &known_types().replace("confidence: 10000", "confidence: 20000"),
        "extraction-document-malformed",
    );
}

/// A record value with one field written twice: the reader's `Value::Record` decodes through
/// `ekr_core::decode::unique_map`, the SDK's `Value::Record` through a plain map.
#[test]
fn adv2_mirror_refuses_a_record_value_with_a_field_written_twice() {
    mirror_refuses(
        &known_types().replace(
            "value: {value_kind: String, value: Globex Corporation}",
            "value: {value_kind: Record, value: {a: {value_kind: Integer, value: 1}, a: \
             {value_kind: Integer, value: 2}}}",
        ),
        "extraction-document-malformed",
    );
}

/// A record type with one field written twice in the document's ontology: the reader's
/// `ValueSpec::Record` decodes through `unique_map`, the mirror's wire form through a plain map.
#[test]
fn adv2_mirror_refuses_a_record_type_with_a_field_written_twice() {
    mirror_refuses(
        &known_types().replace(
            "entities:",
            "ontology:\n  node_types:\n  - name: Gadget\n    properties:\n    - name: size\n      \
             value: {value_kind: Record, parameters: {w: {value_kind: Integer}, w: {value_kind: \
             Integer}}}\nentities:",
        ),
        "extraction-document-malformed",
    );
}

/// A fact citing an id the document's evidence does not list is `fact-evidence-unlisted` to the
/// reader, a check of the document alone: the verb writes nothing. Over a child session the SDK
/// routine applies it whenever the store already holds that id, so the two paths write different
/// stores from one document.
#[test]
fn adv2_a_fact_citing_unlisted_evidence_is_refused_on_both_paths() {
    let document = "format: ekr.extraction-document/1
facts:
- !Property
  subject: {node_type: Organization, aliases: [Initech]}
  property: legal_name
  value: {value_kind: String, value: Initech LLC}
  evidence: [00000000-0000-4000-8000-000000000403]
evidence: []
";
    let (by_verb, by_sdk) = (World::new("file"), World::new("file"));
    by_verb.verb(&known_types());
    by_sdk.verb(&known_types());

    let refused = by_verb.verb_output(document);
    assert_eq!(refused.status.code(), Some(2), "the verb refuses it");
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert!(stderr.contains("fact-evidence-unlisted"), "{stderr}");

    let before = (by_sdk.revision(), by_sdk.active());
    let mut session = by_sdk.session();
    let outcome = ExtractionDocument::from_yaml(document)
        .map_err(|refusal| refusal.code)
        .map(|mirrored| apply(&mut session, &mirrored, operator()));
    session.close().unwrap();
    let after = (by_sdk.revision(), by_sdk.active());
    assert_eq!(
        after, before,
        "the SDK over a child session wrote what the verb refused; it answered {outcome:?}"
    );
}

// --- 3. `stopped` ---------------------------------------------------------------------------------

/// A child session that dies after `commits` answered commits: every later request fails as a
/// process that ended before answering.
struct DiesAfter {
    session: ProcessSession,
    commits: usize,
}

impl ekr_sdk::transport::Transport for DiesAfter {
    fn request(
        &mut self,
        request: &ekr_sdk::transport::Request,
    ) -> Result<ekr_sdk::reply::Reply, ekr_sdk::transport::TransportError> {
        if self.commits == 0 {
            return Err(ekr_sdk::transport::TransportError::Died {
                verb: request.verb().to_owned(),
                status: "cut by the test".to_owned(),
                stderr_tail: String::new(),
            });
        }
        let reply = self.session.request(request)?;
        if request.verb() == "commit" && reply.exit == 0 {
            self.commits -= 1;
        }
        Ok(reply)
    }
}

/// The schema a NodeRef fact can be rejected under: `Project.owner`, a reference to a Person.
const PROJECT: &str = "ontology:
  node_types:
  - name: Project
    properties:
    - name: owner
      value: {value_kind: NodeRef, parameters: {allowed_types: [Person]}}
";

/// Three facts the kernel takes in two commits: the batch of three is rejected for facts[1], an
/// owner no graph holds, so it is bisected — facts[0] commits alone, then facts[1] is refused and
/// facts[2] commits.
fn three_facts() -> String {
    format!(
        "format: ekr.extraction-document/1
{PROJECT}facts:
- !Property
  subject: {{node_type: Organization, aliases: [Globex]}}
  property: legal_name
  value: {{value_kind: String, value: Globex One}}
  evidence: [00000000-0000-4000-8000-000000000403]
- !Property
  subject: {{node_type: Project, aliases: [Apollo]}}
  property: owner
  value: {{value_kind: NodeRef, value: 00000000-0000-4000-8000-0000000009ff}}
  evidence: [00000000-0000-4000-8000-000000000403]
- !Property
  subject: {{node_type: Organization, aliases: [Globex]}}
  property: legal_name
  value: {{value_kind: String, value: Globex Two}}
  evidence: [00000000-0000-4000-8000-000000000403]
evidence:
{ITEM_0403}"
    )
}

/// Globex, Apollo and the Project schema, committed by the verb, so a later run of
/// [`three_facts`] has no schema change and no node to create: its first commit is a fact.
fn with_globex_and_apollo() -> World {
    let world = World::new("file");
    world.verb(&format!(
        "format: ekr.extraction-document/1
{PROJECT}entities:
- node_type: Organization
  aliases: [Globex]
- node_type: Project
  aliases: [Apollo]
facts: []
evidence: []
"
    ));
    world
}

/// Acceptance: "a document the engine's reader accepts never ends in a fault after part of it
/// committed: the report names what committed". When the first commit of the run is inside the
/// facts batch and the run then stops, the batch's commits live in `BatchError::report`, which
/// `apply` never absorbs; the run's own report is empty, so `apply` returns the error.
#[test]
fn adv2_a_stop_after_the_runs_first_commit_inside_the_facts_batch_is_a_report() {
    let world = with_globex_and_apollo();
    let before = world.revision();
    let document = ExtractionDocument::from_yaml(&three_facts()).unwrap();
    let mut cut = DiesAfter {
        session: world.session(),
        commits: 1,
    };
    let outcome = apply(&mut cut, &document, operator());
    cut.session.close().unwrap();
    assert_eq!(world.revision(), before + 1, "facts[0] committed");
    let report = match outcome {
        Ok(report) => report,
        Err(error) => {
            panic!("one transaction committed, and apply answered an error, not a report: {error}")
        }
    };
    assert_eq!(report.committed.len(), 1, "{report:?}");
    assert!(report.stopped.is_some(), "{report:?}");
}

/// The same stop after the schema change and the new nodes committed: `stopped` is set, and the
/// report must name every transaction that committed — the commit inside the facts batch too.
#[test]
fn adv2_a_stop_inside_the_facts_batch_reports_every_commit_before_it() {
    let world = World::new("file");
    let before = world.revision();
    let document = ExtractionDocument::from_yaml(&three_facts()).unwrap();
    // The schema change, the new nodes, then facts[0].
    let mut cut = DiesAfter {
        session: world.session(),
        commits: 3,
    };
    let report = apply(&mut cut, &document, operator()).unwrap();
    cut.session.close().unwrap();
    let committed = world.revision() - before;
    assert_eq!(committed, 3, "three transactions committed");
    assert!(report.stopped.is_some(), "{report:?}");
    assert_eq!(
        u64::try_from(report.committed.len()).unwrap(),
        committed,
        "the report names {} of the {committed} transactions that committed: {report:?}",
        report.committed.len()
    );
}

/// After that stop, running the document again commits only what did not: facts[0] is held,
/// facts[1] is rejected again, facts[2] commits.
#[test]
fn adv2_a_rerun_after_a_stop_adds_only_what_did_not_commit() {
    let world = World::new("file");
    let document = ExtractionDocument::from_yaml(&three_facts()).unwrap();
    let mut cut = DiesAfter {
        session: world.session(),
        commits: 3,
    };
    let first = apply(&mut cut, &document, operator()).unwrap();
    cut.session.close().unwrap();
    assert!(first.stopped.is_some(), "{first:?}");
    let before = world.revision();
    let again = world.verb(&three_facts());
    assert_eq!(
        again["held"],
        serde_json::json!([{"item": "facts[0]", "reason": "asserted"}]),
        "{again}"
    );
    assert_eq!(again["committed"].as_array().unwrap().len(), 1, "{again}");
    assert_eq!(again["rejected"][0]["item"], "facts[1]", "{again}");
    assert_eq!(world.revision(), before + 1);
    let third = world.verb(&three_facts());
    assert_eq!(
        third["held"],
        serde_json::json!([
            {"item": "facts[0]", "reason": "asserted"},
            {"item": "facts[2]", "reason": "asserted"}
        ]),
        "{third}"
    );
    assert_eq!(third["committed"], serde_json::json!([]), "{third}");
}

// --- 1. `held` against a store that moved on --------------------------------------------------

/// Acceptance: "applying one document a second time adds no assertion". Between the two runs the
/// operator retracted the legal name the first run asserted. The domain holds only a fact with an
/// *active* identical assertion, so the second run asserts it again, from the same evidence, and
/// the report lists it as committed, not held.
#[test]
fn adv2_a_second_run_does_not_reassert_a_retracted_fact() {
    let world = World::new("file");
    world.verb(&known_types());
    let retracted = world.assertion_saying("Globex Corporation");
    let mut session = world.session();
    let report = Batcher::new(operator())
        .commit(
            &mut session,
            &[vec![Operation::RetractAssertion(Retraction::new(
                retracted,
                "the registry says otherwise",
            ))]],
        )
        .unwrap();
    session.close().unwrap();
    assert_eq!(
        report.committed.len(),
        1,
        "the retraction commits: {report:?}"
    );
    let before = world.active();

    let again = world.verb(&known_types());
    assert_eq!(
        world.active(),
        before,
        "the second run asserted again: {again}"
    );
}

/// As above, after a supersession: the operator replaced the legal name from 2026-04-01 on. The
/// second run asserts the old name again over all valid time, beside its replacement, for a
/// property of cardinality One.
#[test]
fn adv2_a_second_run_does_not_reassert_a_superseded_fact() {
    use ekr_sdk::document::{
        Assertion, NodeId, Object, Predicate, Subject, Supersession, TemporalRange, Timestamp,
        Value as Said,
    };
    let world = World::new("file");
    world.verb(&known_types());
    let old = world.assertion_saying("Globex Corporation");
    let graph = world.graph();
    let globex: NodeId = graph["nodes"]
        .as_object()
        .unwrap()
        .iter()
        .find(|(_, node)| node["canonical_name"] == "Globex")
        .map(|(id, _)| id.parse().unwrap())
        .unwrap();
    let root = graph["root"]["id"].as_str().unwrap().parse().unwrap();
    let from = Timestamp::from_millis(1_775_001_600_000);
    let replacement = Assertion::new(
        root,
        Subject::Node(globex),
        Predicate::Property("00000000-0000-4000-8000-000000000801".parse().unwrap()),
        Object::Value(Said::String("Globex Inc".to_owned())),
        operator(),
    )
    .citing("00000000-0000-4000-8000-000000000403".parse().unwrap())
    .with_valid_time(TemporalRange {
        from: Some(from),
        to: None,
    });
    let by = replacement.id;
    let mut session = world.session();
    let report = Batcher::new(operator())
        .commit(
            &mut session,
            &[vec![
                Operation::AddAssertion(Box::new(replacement)),
                Operation::SupersedeAssertion(Supersession::new(old, by, from)),
            ]],
        )
        .unwrap();
    session.close().unwrap();
    assert_eq!(
        report.committed.len(),
        1,
        "the supersession commits: {report:?}"
    );
    let before = world.active();

    let again = world.verb(&known_types());
    assert_eq!(
        world.active(),
        before,
        "the second run asserted again: {again}"
    );
}

/// The same fact read again from new evidence. Not held — the evidence differs — so asserted;
/// what it must not be is rejected, which would leave a corroborating document unappliable.
#[test]
fn adv2_a_fact_read_again_from_new_evidence_is_not_rejected() {
    let world = World::new("file");
    world.verb(&known_types());
    let document = format!(
        "format: ekr.extraction-document/1
facts:
- !Property
  subject: {{node_type: Organization, aliases: [Globex]}}
  property: legal_name
  value: {{value_kind: String, value: Globex Corporation}}
  evidence: [00000000-0000-4000-8000-000000000404]
evidence:
{}",
        item_0404()
    );
    let report = world.verb(&document);
    assert_eq!(report["rejected"], serde_json::json!([]), "{report}");
    assert_eq!(report["held"], serde_json::json!([]), "{report}");
    assert_eq!(report["committed"].as_array().unwrap().len(), 1, "{report}");
    let again = world.verb(&document);
    assert_eq!(
        again["held"],
        serde_json::json!([{"item": "facts[0]", "reason": "asserted"}]),
        "{again}"
    );
}

// --- 2. Group naming across orders and providers ----------------------------------------------

/// Named things [Zed, Carol] and [carol, Zed] are one Person, named by the least of their first
/// aliases in byte order: `Zed` (0x5A) before `carol` (0x63). Both orders, both providers, one
/// node with the same names.
#[test]
fn adv2_a_group_is_named_alike_in_every_order_on_both_providers() {
    let names = |backend: &'static str, entities: &str| {
        let world = World::new(backend);
        world.verb(&format!(
            "format: ekr.extraction-document/1\nentities:\n{entities}facts: []\nevidence: []\n"
        ));
        let graph = world.graph();
        let made: Vec<(String, BTreeSet<String>)> = graph["nodes"]
            .as_object()
            .unwrap()
            .values()
            .filter(|node| {
                !["Alice", "Bob", "Acme"].contains(&node["canonical_name"].as_str().unwrap())
            })
            .map(|node| {
                (
                    node["canonical_name"].as_str().unwrap().to_owned(),
                    node["aliases"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|alias| alias.as_str().unwrap().to_owned())
                        .collect(),
                )
            })
            .collect();
        made
    };
    let one = "- node_type: Person\n  aliases: [Zed, Carol]\n- node_type: Person\n  aliases: [carol, Zed]\n";
    let other = "- node_type: Person\n  aliases: [carol, Zed]\n- node_type: Person\n  aliases: [Zed, Carol]\n";
    let expected = names("file", one);
    assert_eq!(expected.len(), 1, "{expected:?}");
    assert_eq!(expected[0].0, "Zed", "{expected:?}");
    for (backend, entities) in [("file", other), ("sqlite", one), ("sqlite", other)] {
        assert_eq!(names(backend, entities), expected, "{backend}");
    }
}

// --- Documents the reader accepts and the verb faults on --------------------------------------

/// The reader accepts every `ekr.graph.EvidenceSource`; the SDK mirror's `EvidenceSource` has one
/// variant, so the verb faults (exit 1) on an item from a URL. The domain declares two outcomes,
/// `applied` (a report, the kernel's rejection a row of it) and `refused` (exit 2).
#[test]
fn adv2_the_verb_does_not_fault_on_evidence_from_a_url() {
    let world = World::new("file");
    let document = known_types().replace(
        "source: !HumanStatement\n      identity: Quarterly report",
        "source: !Url https://example.org/report",
    );
    ekr_integrate::ExtractionDocument::from_yaml(&document).expect("the reader decodes it");
    let output = world.verb_output(&document);
    assert!(
        matches!(output.status.code(), Some(0 | 2)),
        "exit {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A new subtype redeclaring a property its parent declares otherwise: the reader takes the
/// subtype's own declaration and accepts the document; `Ontology::ensure` calls it a conflict and
/// the verb faults (exit 1), an outcome the domain does not declare.
#[test]
fn adv2_the_verb_does_not_fault_on_a_subtype_redeclaring_a_property() {
    let world = World::new("file");
    // The reader runs first: had it refused the document, the verb would exit 2.
    let document = "format: ekr.extraction-document/1
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
    let output = world.verb_output(document);
    assert!(
        matches!(output.status.code(), Some(0 | 2)),
        "exit {:?}: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}
