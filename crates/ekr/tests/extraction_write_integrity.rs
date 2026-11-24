//! `story:extraction-write-integrity`: an extraction writes the same state whether its facts
//! arrive in one batch or one by one, in one process or two, and every committed fact cites the
//! evidence it was extracted from (review of 0.0.42, findings 1-4, and single-valued adds).
//!
//! Every case drives the built binary: the verb, and the SDK's routine over a child `ekr session`
//! of it, on both providers. The two-writer case interleaves two child sessions on one store at a
//! fixed point of the first writer's requests, so its outcome does not depend on timing.

use std::path::PathBuf;
use std::process::Output;

use ekr_sdk::binary::EkrBinary;
use ekr_sdk::document::{AgentId, ExtractionDocument};
use ekr_sdk::extraction::{ApplyError, ApplyOptions};
use ekr_sdk::reply::Reply;
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{Request, Transport, TransportError};
use serde_json::Value;

const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
/// The example seed's `Organization.legal_name`, declared `cardinality: One`.
const LEGAL_NAME: &str = "00000000-0000-4000-8000-000000000801";
/// A start of valid time, in milliseconds; each case counts from it.
const T0: u64 = 1_775_000_000_000;

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

/// One provider in its own directory: the example host and the example seed.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn new(backend: &'static str) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        std::fs::write(
            world.file("host.json"),
            text(&["example", "ekr.cli-host/1"]),
        )
        .unwrap();
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
            "{args:?}: stderr {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// `ekr apply-extraction` of `document`: its output, whatever its exit.
    fn apply_verb_output(&self, document: &str) -> Output {
        let path = self.file("extraction.yaml");
        std::fs::write(&path, document).unwrap();
        self.output(&["apply-extraction", path.to_str().unwrap()])
    }

    /// `ekr apply-extraction` of `document`, which must exit 0: the report.
    fn apply_verb(&self, document: &str) -> Value {
        let output = self.apply_verb_output(document);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}: stderr {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// A child `ekr session` of this world's store.
    fn session(&self) -> ProcessSession {
        let binary = EkrBinary::open(binary()).unwrap();
        ProcessSession::start(&binary, self.config(), SessionOptions::default()).unwrap()
    }

    fn head(&self) -> u64 {
        self.json(&["head"])["revision"].as_u64().unwrap()
    }

    fn graph(&self) -> Value {
        self.json(&["snapshot"])["graph"]["graph"].clone()
    }

    /// The id of the node named `name`, or holding it as an alias.
    fn node(&self, name: &str) -> String {
        self.graph()["nodes"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(_, node)| {
                node["canonical_name"] == name
                    || node["aliases"]
                        .as_array()
                        .is_some_and(|aliases| aliases.iter().any(|alias| alias == name))
            })
            .map(|(id, _)| id.clone())
            .unwrap_or_else(|| panic!("{}: no node {name}", self.backend))
    }

    /// The values of the active `legal_name` assertions of the node named `name`, sorted.
    fn active_legal_names(&self, name: &str) -> Vec<String> {
        let node = self.node(name);
        let mut found: Vec<String> = self.graph()["assertions"]
            .as_object()
            .unwrap()
            .values()
            .filter(|assertion| {
                assertion["lifecycle"] == "Active"
                    && assertion["subject"]["Node"] == node.as_str()
                    && assertion["predicate"]["Property"] == LEGAL_NAME
            })
            .map(|assertion| {
                assertion["object"]["Value"]["value"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        found.sort();
        found
    }

    /// Every active `legal_name` assertion of the store: its node and value.
    fn every_active_legal_name(&self) -> Vec<(String, String)> {
        let mut found: Vec<(String, String)> = self.graph()["assertions"]
            .as_object()
            .unwrap()
            .values()
            .filter(|assertion| {
                assertion["lifecycle"] == "Active"
                    && assertion["predicate"]["Property"] == LEGAL_NAME
            })
            .map(|assertion| {
                (
                    assertion["subject"]["Node"].as_str().unwrap().to_owned(),
                    assertion["object"]["Value"]["value"]
                        .as_str()
                        .unwrap()
                        .to_owned(),
                )
            })
            .collect();
        found.sort();
        found
    }
}

/// The id `00000000-0000-4000-8000-000000000<suffix>`.
fn evidence_id(suffix: &str) -> String {
    format!("00000000-0000-4000-8000-000000000{suffix}")
}

/// One evidence item under [`evidence_id`] of `suffix`, observed at `observed_at`, of `payload`.
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

/// A `!Property` fact: the Organization known by `aliases` has the legal name `value`, citing
/// [`evidence_id`] of `cited`, marked `replaces` when `replaces`.
fn legal_name(aliases: &[&str], value: &str, cited: &str, replaces: bool) -> String {
    format!(
        "- !Property
  subject: {{node_type: Organization, aliases: [{}]}}
  property: legal_name
{}  value: {{value_kind: String, value: {value}}}
  evidence: [{}]
",
        aliases.join(", "),
        if replaces { "  replaces: true\n" } else { "" },
        evidence_id(cited)
    )
}

/// A document of `entities`, `facts` and `evidence`, each already written as list items.
fn extraction(entities: &[&str], facts: &[String], evidence: &[String]) -> String {
    let entities: String = entities
        .iter()
        .map(|aliases| format!("- node_type: Organization\n  aliases: [{aliases}]\n"))
        .collect();
    let entities = if entities.is_empty() {
        String::new()
    } else {
        format!("entities:\n{entities}")
    };
    format!(
        "format: ekr.extraction-document/1\n{entities}facts:\n{}evidence:\n{}",
        facts.concat(),
        evidence.concat()
    )
}

/// The SDK's routine over `transport` on `document`: the report as JSON.
fn apply_over(
    transport: &mut dyn Transport,
    document: &str,
    options: &ApplyOptions,
) -> Result<Value, ApplyError> {
    let document = ExtractionDocument::decode(document).unwrap();
    ekr_sdk::extraction::apply_with(
        transport,
        &document,
        OPERATOR.parse::<AgentId>().unwrap(),
        options,
    )
    .map(|report| serde_json::to_value(report).unwrap())
}

/// The SDK's routine over a child session of `world`.
fn apply_sdk(world: &World, document: &str, options: &ApplyOptions) -> Result<Value, ApplyError> {
    let mut session = world.session();
    let report = apply_over(&mut session, document, options);
    session.close().unwrap();
    report
}

/// Each report row's `item`, in order.
fn items(rows: &Value) -> Vec<String> {
    rows.as_array()
        .unwrap()
        .iter()
        .map(|row| row["item"].as_str().unwrap().to_owned())
        .collect()
}

/// Finding 1: a document reusing a retained evidence id for another item is refused by name,
/// `duplicate-identity`, through the verb and through the SDK, strict or not, and writes nothing;
/// the same document applied again is held and writes nothing either.
fn a_reused_evidence_id_naming_another_item_is_refused_and_a_verbatim_retry_is_held(
    backend: &'static str,
) {
    let first = extraction(
        &[],
        &[legal_name(&["Vela"], "Vela Green", "501", false)],
        &[evidence_item("501", b"Vela is green.", T0 + 2_000)],
    );
    // The same id, another statement observed earlier.
    let reused = extraction(
        &[],
        &[legal_name(&["Vela"], "Vela Blue", "501", false)],
        &[evidence_item("501", b"Vela is blue.", T0 + 1_000)],
    );
    let world = World::new(backend);
    let report = world.apply_verb(&first);
    assert_eq!(
        report["rejected"],
        serde_json::json!([]),
        "{backend}: {report}"
    );
    let head = world.head();

    let again = world.apply_verb(&first);
    assert_eq!(
        again["committed"],
        serde_json::json!([]),
        "{backend}: {again}"
    );
    assert_eq!(items(&again["held"]), ["facts[0]"], "{backend}: {again}");
    let again = apply_sdk(&world, &first, &ApplyOptions::default()).unwrap();
    assert_eq!(items(&again["held"]), ["facts[0]"], "{backend}: {again}");
    assert_eq!(world.head(), head, "{backend}: a retry writes nothing");

    for strict in [false, true] {
        let mut args = vec!["apply-extraction"];
        if strict {
            args.push("--strict");
        }
        let path = world.file("reused.yaml");
        std::fs::write(&path, &reused).unwrap();
        args.push(path.to_str().unwrap());
        let output = world.output(&args);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{backend} {strict}: {stderr}"
        );
        assert!(
            stderr.starts_with("ekr: duplicate-identity: ") && stderr.contains(&evidence_id("501")),
            "{backend} {strict}: {stderr}"
        );

        let options = if strict {
            ApplyOptions::default().strict()
        } else {
            ApplyOptions::default()
        };
        match apply_sdk(&world, &reused, &options) {
            Err(ApplyError::Refused(refusal)) => {
                assert_eq!(refusal.code, "duplicate-identity", "{backend} {strict}");
                assert_eq!(refusal.name, evidence_id("501"), "{backend} {strict}");
            }
            other => panic!("{backend} {strict}: refused, not {other:?}"),
        }
    }
    assert_eq!(world.head(), head, "{backend}: a refusal writes nothing");
    assert_eq!(
        world.active_legal_names("Vela"),
        ["Vela Green"],
        "{backend}"
    );
}

/// A transport over a child session that, before its first `propose`, runs `other` to its end:
/// another writer commits between this one's read of the store and its first write.
struct Interleaved<'w> {
    session: ProcessSession,
    other: Option<Box<dyn FnOnce() + 'w>>,
}

impl Transport for Interleaved<'_> {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        if request.verb() == "propose" {
            if let Some(other) = self.other.take() {
                other();
            }
        }
        self.session.request(request)
    }
}

/// Finding 2: two writers import the same 12 facts with the same evidence into one store, the
/// second committing all of them between the first's read of the store and its first write. The
/// store ends with one active assertion per claim, and the first writer holds every fact.
fn two_writers_of_the_same_facts_leave_one_active_assertion_per_claim(backend: &'static str) {
    let names: Vec<String> = (0..12).map(|n| format!("Org{n:02}")).collect();
    let entities = extraction(
        &names.iter().map(String::as_str).collect::<Vec<_>>(),
        &[],
        &[],
    )
    .replace("facts:\nevidence:\n", "facts: []\nevidence: []\n");
    let facts: Vec<String> = names
        .iter()
        .map(|name| legal_name(&[name.as_str()], &format!("{name} Ltd"), "601", false))
        .collect();
    let document = extraction(
        &[],
        &facts,
        &[evidence_item(
            "601",
            b"The register lists twelve companies.",
            T0,
        )],
    );
    let world = World::new(backend);
    world.apply_verb(&entities);
    let before = world.head();

    let mut first = Interleaved {
        session: world.session(),
        other: Some(Box::new(|| {
            let report = apply_sdk(&world, &document, &ApplyOptions::default()).unwrap();
            assert_eq!(
                report["rejected"],
                serde_json::json!([]),
                "{backend}: {report}"
            );
            assert_eq!(report["held"], serde_json::json!([]), "{backend}: {report}");
        })),
    };
    let report = apply_over(&mut first, &document, &ApplyOptions::default());
    assert!(first.other.is_none(), "{backend}: the second writer ran");
    first.session.close().unwrap();
    let report = report.unwrap_or_else(|error| panic!("{backend}: {error}"));

    assert_eq!(
        report["rejected"],
        serde_json::json!([]),
        "{backend}: {report}"
    );
    assert_eq!(report["stopped"], Value::Null, "{backend}: {report}");
    assert_eq!(
        report["committed"],
        serde_json::json!([]),
        "{backend}: {report}"
    );
    assert_eq!(items(&report["held"]).len(), 12, "{backend}: {report}");
    let active = world.every_active_legal_name();
    assert_eq!(active.len(), 12, "{backend}: {active:?}");
    assert_eq!(
        world.head(),
        before + 1,
        "{backend}: one commit, the second writer's"
    );
}

/// Finding 3: one document adds green at 1000, replaces it with blue at 2000, then adds yellow at
/// 3000, all of one single-valued property of one node. In one batch it leaves what the three
/// facts applied one by one leave: yellow active, blue superseded by it, green by blue.
fn a_batch_of_ordered_facts_leaves_what_the_facts_one_by_one_leave(backend: &'static str) {
    let facts = [
        legal_name(&["Vela"], "Green", "701", false),
        legal_name(&["Vela"], "Blue", "702", true),
        legal_name(&["Vela"], "Yellow", "703", false),
    ];
    let evidence = [
        evidence_item("701", b"Vela is green.", T0 + 1_000),
        evidence_item("702", b"Vela is blue now.", T0 + 2_000),
        evidence_item("703", b"Vela is yellow.", T0 + 3_000),
    ];
    let batch = World::new(backend);
    let report = batch.apply_verb(&extraction(&[], &facts, &evidence));
    assert_eq!(
        report["rejected"],
        serde_json::json!([]),
        "{backend}: {report}"
    );

    let sdk = World::new(backend);
    let report = apply_sdk(
        &sdk,
        &extraction(&[], &facts, &evidence),
        &ApplyOptions::default(),
    )
    .unwrap();
    assert_eq!(
        report["rejected"],
        serde_json::json!([]),
        "{backend}: {report}"
    );

    let one_by_one = World::new(backend);
    for (fact, item) in facts.iter().zip(&evidence) {
        let report = one_by_one.apply_verb(&extraction(
            &[],
            std::slice::from_ref(fact),
            std::slice::from_ref(item),
        ));
        assert_eq!(
            report["rejected"],
            serde_json::json!([]),
            "{backend}: {report}"
        );
    }

    let expected = one_by_one.active_legal_names("Vela");
    assert_eq!(expected, ["Yellow"], "{backend}");
    assert_eq!(batch.active_legal_names("Vela"), expected, "{backend}");
    assert_eq!(sdk.active_legal_names("Vela"), expected, "{backend}");
}

/// Finding 4: the store's node holds two aliases, `red-spot` and `r-1`. One document sets green
/// under the first and replaces it with blue under the second: the replacement applies, as it does
/// when both facts use the first alias.
fn a_replacement_under_a_second_alias_of_the_node_replaces_the_value_set_under_the_first(
    backend: &'static str,
) {
    let entity = extraction(&["red-spot, r-1"], &[], &[])
        .replace("facts:\nevidence:\n", "facts: []\nevidence: []\n");
    let evidence = [
        evidence_item("801", b"red-spot is green.", T0 + 1_000),
        evidence_item("802", b"r-1 is blue now.", T0 + 2_000),
    ];
    for (second, how) in [("r-1", "the second alias"), ("red-spot", "the first alias")] {
        let document = extraction(
            &[],
            &[
                legal_name(&["red-spot"], "Green", "801", false),
                legal_name(&[second], "Blue", "802", true),
            ],
            &evidence,
        );
        let world = World::new(backend);
        world.apply_verb(&entity);
        let node = world.node("r-1");
        assert_eq!(
            world.node("red-spot"),
            node,
            "{backend}: one node of two aliases"
        );

        let report = world.apply_verb(&document);
        assert_eq!(
            report["rejected"],
            serde_json::json!([]),
            "{backend}, {how}: {report}"
        );
        assert_eq!(
            world.active_legal_names("r-1"),
            ["Blue"],
            "{backend}, {how}"
        );
    }
}

/// Cardinality One: a value of a single-valued property, not marked `replaces`, supersedes the
/// active value of another object from its own valid time, through the verb and the SDK. An equal
/// value from new evidence stays active beside the one it corroborates, and an older value is
/// rejected (`invalid-supersession`), leaving the newer one active.
fn a_second_value_of_a_single_valued_property_supersedes_the_first(backend: &'static str) {
    let first = extraction(
        &[],
        &[legal_name(&["Vela"], "Vela One", "901", false)],
        &[evidence_item("901", b"Vela One registered.", T0 + 1_000)],
    );
    let second = extraction(
        &[],
        &[legal_name(&["Vela"], "Vela Two", "902", false)],
        &[evidence_item("902", b"Vela is now Vela Two.", T0 + 2_000)],
    );
    let corroborated = extraction(
        &[],
        &[legal_name(&["Vela"], "Vela Two", "903", false)],
        &[evidence_item(
            "903",
            b"The register says Vela Two.",
            T0 + 3_000,
        )],
    );
    let older = extraction(
        &[],
        &[legal_name(&["Vela"], "Vela Zero", "904", false)],
        &[evidence_item(
            "904",
            b"An old letter says Vela Zero.",
            T0 + 500,
        )],
    );
    for by_sdk in [false, true] {
        let world = World::new(backend);
        let apply = |document: &str| {
            if by_sdk {
                apply_sdk(&world, document, &ApplyOptions::default()).unwrap()
            } else {
                world.apply_verb(document)
            }
        };
        apply(&first);
        let report = apply(&second);
        assert_eq!(
            report["rejected"],
            serde_json::json!([]),
            "{backend}: {report}"
        );
        assert_eq!(
            world.active_legal_names("Vela"),
            ["Vela Two"],
            "{backend} {by_sdk}"
        );

        let node = world.node("Vela");
        let graph = world.graph();
        let superseded: Vec<&Value> = graph["assertions"]
            .as_object()
            .unwrap()
            .values()
            .filter(|assertion| {
                assertion["subject"]["Node"] == node.as_str()
                    && assertion["object"]["Value"]["value"] == "Vela One"
            })
            .collect();
        assert_eq!(superseded.len(), 1, "{backend} {by_sdk}");
        assert_eq!(
            superseded[0]["lifecycle"]["Superseded"]["effective_from"],
            T0 + 2_000,
            "{backend} {by_sdk}: {}",
            superseded[0]
        );

        let report = apply(&corroborated);
        assert_eq!(
            report["rejected"],
            serde_json::json!([]),
            "{backend}: {report}"
        );
        assert_eq!(
            world.active_legal_names("Vela"),
            ["Vela Two", "Vela Two"],
            "{backend} {by_sdk}: a corroboration is not superseded"
        );

        let report = apply(&older);
        assert_eq!(
            items(&report["rejected"]),
            ["facts[0]"],
            "{backend}: {report}"
        );
        assert_eq!(
            report["rejected"][0]["issues"][0]["code"], "invalid-supersession",
            "{backend}: {report}"
        );
        assert_eq!(
            world.active_legal_names("Vela"),
            ["Vela Two", "Vela Two"],
            "{backend} {by_sdk}"
        );
    }
}

/// Each case once per provider, a test each, so no test holds both (the 60 s bound).
macro_rules! per_provider {
    ($provider:ident) => {
        mod $provider {
            #[test]
            fn a_reused_evidence_id_naming_another_item_is_refused_and_a_verbatim_retry_is_held() {
                super::a_reused_evidence_id_naming_another_item_is_refused_and_a_verbatim_retry_is_held(
                    stringify!($provider),
                );
            }

            #[test]
            fn two_writers_of_the_same_facts_leave_one_active_assertion_per_claim() {
                super::two_writers_of_the_same_facts_leave_one_active_assertion_per_claim(
                    stringify!($provider),
                );
            }

            #[test]
            fn a_batch_of_ordered_facts_leaves_what_the_facts_one_by_one_leave() {
                super::a_batch_of_ordered_facts_leaves_what_the_facts_one_by_one_leave(
                    stringify!($provider),
                );
            }

            #[test]
            fn a_replacement_under_a_second_alias_of_the_node_replaces_the_value_set_under_the_first(
            ) {
                super::a_replacement_under_a_second_alias_of_the_node_replaces_the_value_set_under_the_first(
                    stringify!($provider),
                );
            }

            #[test]
            fn a_second_value_of_a_single_valued_property_supersedes_the_first() {
                super::a_second_value_of_a_single_valued_property_supersedes_the_first(
                    stringify!($provider),
                );
            }
        }
    };
}

per_provider!(file);
per_provider!(sqlite);
