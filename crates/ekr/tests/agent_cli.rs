//! `story:agent-discoverable-cli`: an agent holding only the `ekr` binary can find out what to
//! write, mint the ids it needs, read the revision numbers and ontology ids it needs, and drive
//! the retraction example to the end without reading source or fixtures.
//!
//! Every case drives fresh binary processes. Every printed example is held to the real readers
//! (`TransactionDocument::parse`, `SeedDocument::from_yaml`, `CliHostConfigurationV1::from_json`),
//! so the text an agent reads cannot drift from what the runtime accepts.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::process::Output;

use clap::ValueEnum;
use ekr::host::CliHostConfigurationV1;
use ekr_kernel::{GraphOperation, SeedDocument, TransactionDocument};
use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const FORMATS: [&str; 3] = ["ekr.transaction-document/1", "ekr-seed/2", "ekr.cli-host/1"];
/// 2026-03-12T00:00:00Z.
const MARCH_12: i64 = 1_773_273_600_000;
/// 2020-01-01T00:00:00Z.
const JAN_2020: i64 = 1_577_836_800_000;

/// The kind name of a parsed operation, through the binary's own `OperationKind::of` — the
/// exhaustive `match` over `GraphOperation` with no `_` arm, so a new variant does not compile
/// until it has a kind, and a kind is not listed until it has an example this suite parses.
fn kind_of(operation: &GraphOperation) -> String {
    let kind = ekr::cli::OperationKind::of(operation);
    kind.to_possible_value().unwrap().get_name().to_owned()
}

/// Every name a clap value enum of the binary accepts.
fn names<E: ValueEnum>() -> Vec<String> {
    E::value_variants()
        .iter()
        .map(|v| v.to_possible_value().unwrap().get_name().to_owned())
        .collect()
}

/// `GraphOperation` has twelve variants (the eleven of design § 19 and amendment 87, plus
/// `SupersedeAssertion`).
const KIND_COUNT: usize = 12;

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND"] {
        command.env_remove(var);
    }
    command
}

fn run(args: &[&str]) -> Output {
    ekr().args(args).output().unwrap()
}

/// Exit 0 and stdout as text; stderr empty.
fn text(args: &[&str]) -> String {
    let output = run(args);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{args:?}: stderr {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// Exit 0 and stdout is exactly one JSON document.
fn json(args: &[&str]) -> Value {
    let out = text(args);
    serde_json::from_str(&out).unwrap_or_else(|e| panic!("{args:?}: not JSON ({e}): {out}"))
}

/// One provider in its own directory, configured from files the binary printed.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
    host: PathBuf,
}

impl World {
    fn new(backend: &'static str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let host = directory.path().join("host.json");
        std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
        Self {
            directory,
            backend,
            host,
        }
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            self.host.display().to_string(),
            "--store".to_owned(),
            self.store().display().to_string(),
            "--backend".to_owned(),
            self.backend.to_owned(),
        ];
        args.extend(verb.iter().map(|a| (*a).to_owned()));
        args
    }

    fn run(&self, verb: &[&str]) -> Output {
        ekr().args(self.args(verb)).output().unwrap()
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: stderr {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty(), "{verb:?}: a success wrote stderr");
        serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
            panic!(
                "{verb:?}: not JSON ({e}): {}",
                String::from_utf8_lossy(&output.stdout)
            )
        })
    }

    fn file(&self, name: &str, contents: &str) -> String {
        let path = self.directory.path().join(name);
        std::fs::write(&path, contents).unwrap();
        path.display().to_string()
    }

    fn seed_from_example(&self) -> Value {
        let seed = self.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        self.ok(&["seed", &seed])
    }
}

/// The kinds `ekr operations` lists, one per line, the kind name first.
fn listed_kinds() -> Vec<String> {
    text(&["operations"])
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split_whitespace().next().unwrap().to_owned())
        .collect()
}

/// The example operation in `ekr operations <Kind>`: every line after the `Example` heading.
fn example_operation(kind: &str) -> String {
    let out = text(&["operations", kind]);
    let at = out
        .lines()
        .position(|line| line.starts_with("Example"))
        .unwrap_or_else(|| panic!("{kind}: no Example section in {out}"));
    let mut example: String = out.lines().skip(at + 1).collect::<Vec<_>>().join("\n");
    example.push('\n');
    example
}

/// A complete `ekr.transaction-document/1` around printed operation entries.
fn document(id: &str, proposer: &str, operations: &[String], evidence: &[&str]) -> String {
    let mut text = format!(
        "format: ekr.transaction-document/1\ntransaction:\n  id: {id}\n  proposer: {proposer}\n  operations:\n"
    );
    for operation in operations {
        for line in operation.lines() {
            text.push_str("  ");
            text.push_str(line);
            text.push('\n');
        }
    }
    text.push_str(&format!("  evidence: [{}]\n", evidence.join(", ")));
    text
}

fn parse_one(example: &str) -> GraphOperation {
    let zero = "00000000-0000-4000-8000-000000000000";
    let document = document(zero, zero, &[example.to_owned()], &[]);
    let parsed = TransactionDocument::parse(document.as_bytes())
        .unwrap_or_else(|e| panic!("the real reader refused {document}: {e}"));
    assert_eq!(parsed.transaction().operations.len(), 1, "{document}");
    parsed.transaction().operations[0].clone()
}

// 1 --------------------------------------------------------------------------------------------

#[test]
fn guide_prints_the_workflow_roles_exit_codes_and_where_ids_come_from() {
    let guide = text(&["guide"]);
    for needle in [
        "propose",
        "validate",
        "commit",
        "operator",
        "validator",
        "exit 0",
        "exit 1",
        "exit 2",
        "ekr mint",
        "ekr head",
        "ekr ontology",
        "ekr operations",
        "ekr example",
        "ekr transactions",
        "EKR_HOST",
    ] {
        assert!(guide.contains(needle), "guide lacks {needle:?}:\n{guide}");
    }
    assert!(
        serde_json::from_str::<Value>(&guide).is_err(),
        "guide is text"
    );
}

// 2 --------------------------------------------------------------------------------------------

#[test]
fn operations_lists_every_kind_and_every_example_parses_as_its_kind() {
    let kinds = listed_kinds();
    let distinct: BTreeSet<_> = kinds.iter().cloned().collect();
    assert_eq!(kinds.len(), KIND_COUNT, "{kinds:?}");
    assert_eq!(distinct.len(), KIND_COUNT, "{kinds:?}");
    assert_eq!(kinds, names::<ekr::cli::OperationKind>());
    for kind in &kinds {
        let out = text(&["operations", kind]);
        assert!(out.contains("Fields"), "{kind}: no fields in {out}");
        let parsed = parse_one(&example_operation(kind));
        assert_eq!(
            &kind_of(&parsed),
            kind,
            "{kind}: the example is another kind"
        );
    }
    let unknown = run(&["operations", "NoSuchKind"]);
    assert_eq!(unknown.status.code(), Some(2));
    assert!(unknown.stdout.is_empty());
}

// 3 --------------------------------------------------------------------------------------------

#[test]
fn every_example_document_is_accepted_by_its_real_reader() {
    let transaction = text(&["example", "ekr.transaction-document/1"]);
    let parsed = TransactionDocument::parse(transaction.as_bytes()).unwrap();
    assert!(!parsed.transaction().operations.is_empty());
    SeedDocument::from_yaml(&text(&["example", "ekr-seed/2"])).unwrap();
    assert_eq!(names::<ekr::cli::ExampleFormat>(), FORMATS);
    CliHostConfigurationV1::from_json(text(&["example", "ekr.cli-host/1"]).as_bytes()).unwrap();
    let unknown = run(&["example", "ekr-seed/9"]);
    assert_eq!(unknown.status.code(), Some(2));
}

#[test]
fn the_seed_and_host_examples_seed_a_fresh_store_and_the_transaction_example_commits() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let seeded = world.seed_from_example();
        assert_eq!(seeded["result"]["revision"], 0, "{backend}: {seeded}");
        let proposal = world.file(
            "transaction.yaml",
            &text(&["example", "ekr.transaction-document/1"]),
        );
        let proposed = world.ok(&["propose", &proposal]);
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        assert_eq!(
            world.ok(&["validate", &id])["kind"],
            "Validated",
            "{backend}"
        );
        assert_eq!(world.ok(&["commit", &id])["kind"], "Committed", "{backend}");
    }
}

// 4 --------------------------------------------------------------------------------------------

#[test]
fn mint_prints_a_fresh_id_of_each_named_kind() {
    let kinds = names::<ekr::cli::IdKind>();
    for named in ["node", "edge", "assertion", "transaction", "evidence"] {
        assert!(kinds.iter().any(|k| k == named), "{named} not mintable");
    }
    for kind in &kinds {
        let kind = kind.as_str();
        let first = json(&["mint", kind]);
        let second = json(&["mint", kind]);
        assert_eq!(first["kind"], kind, "{first}");
        let id = first["id"].as_str().unwrap();
        assert_ne!(first["id"], second["id"], "{kind}: two mints agree");
        assert!(id.parse::<ekr_core::NodeId>().is_ok(), "{kind}: {id}");
    }
    assert_eq!(run(&["mint", "unicorn"]).status.code(), Some(2));
}

// 5 --------------------------------------------------------------------------------------------

#[test]
fn head_prints_the_revision_and_root_and_validate_defaults_to_it() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let unseeded = world.run(&["head"]);
        assert_eq!(unseeded.status.code(), Some(1), "{backend}: unseeded head");
        assert!(unseeded.stdout.is_empty());
        world.seed_from_example();
        let head = world.ok(&["head"]);
        assert_eq!(head["revision"], 0, "{head}");
        assert_eq!(head["root"]["revision"], 0, "{head}");
        let proposal = world.file(
            "transaction.yaml",
            &text(&["example", "ekr.transaction-document/1"]),
        );
        let id = world.ok(&["propose", &proposal])["transaction_id"]
            .as_str()
            .unwrap()
            .to_owned();
        let validated = world.ok(&["validate", &id]);
        assert_eq!(validated["kind"], "Validated", "{validated}");
        world.ok(&["commit", &id]);
        assert_eq!(world.ok(&["head"])["revision"], 1, "{backend}");
        // At head 1 the default must be 1: a validation against 0 would commit as Stale.
        let host: Value = serde_json::from_str(&text(&["example", "ekr.cli-host/1"])).unwrap();
        let operator = host["context"]["operator"].as_str().unwrap();
        let retract = world.file(
            "retract.yaml",
            &document(
                json(&["mint", "transaction"])["id"].as_str().unwrap(),
                operator,
                &[example_operation("RetractAssertion")],
                &[],
            ),
        );
        let second = world.ok(&["propose", &retract])["transaction_id"]
            .as_str()
            .unwrap()
            .to_owned();
        assert_eq!(world.ok(&["validate", &second])["kind"], "Validated");
        let committed = world.ok(&["commit", &second]);
        assert_eq!(committed["kind"], "Committed", "{backend}: {committed}");
        assert_eq!(world.ok(&["head"])["revision"], 2, "{backend}");
        let unknown = world.run(&["validate", "00000000-0000-4000-8000-000000000699"]);
        assert_eq!(unknown.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&unknown.stderr).contains("ekr.kernel.TransactionNotFound"));
    }
}

// 6 --------------------------------------------------------------------------------------------

#[test]
fn transactions_lists_id_state_and_proposer_and_filters_by_state() {
    for backend in BACKENDS {
        let world = World::new(backend);
        world.seed_from_example();
        assert_eq!(world.ok(&["transactions"]), serde_json::json!([]));
        let proposal = world.file(
            "transaction.yaml",
            &text(&["example", "ekr.transaction-document/1"]),
        );
        let id = world.ok(&["propose", &proposal])["transaction_id"].clone();
        let host: Value = serde_json::from_str(&text(&["example", "ekr.cli-host/1"])).unwrap();
        let listed = world.ok(&["transactions"]);
        assert_eq!(listed[0]["transaction_id"], id, "{listed}");
        assert_eq!(listed[0]["state"], "Proposed", "{listed}");
        assert_eq!(
            listed[0]["proposer"], host["context"]["operator"],
            "{listed}"
        );
        assert_eq!(
            world.ok(&["transactions", "--state", "Committed"]),
            serde_json::json!([])
        );
        assert_eq!(world.ok(&["transactions", "--state", "Proposed"]), listed);
        assert_eq!(
            world
                .run(&["transactions", "--state", "Bogus"])
                .status
                .code(),
            Some(2)
        );
        for state in names::<ekr::cli::StateFilter>() {
            let selected = world.ok(&["transactions", "--state", &state]);
            let expected = if state == "Proposed" { 1 } else { 0 };
            assert_eq!(selected.as_array().unwrap().len(), expected, "{state}");
        }
    }
}

// 7 --------------------------------------------------------------------------------------------

#[test]
fn ontology_prints_types_and_properties_by_name_and_id() {
    for backend in BACKENDS {
        let world = World::new(backend);
        world.seed_from_example();
        let ontology = world.ok(&["ontology"]);
        assert_eq!(ontology["revision"], 0, "{ontology}");
        let names = |key: &str| -> BTreeSet<String> {
            ontology[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| {
                    assert!(t["id"]
                        .as_str()
                        .unwrap()
                        .parse::<ekr_core::TypeId>()
                        .is_ok());
                    t["name"].as_str().unwrap().to_owned()
                })
                .collect()
        };
        assert!(names("node_types").contains("Person"), "{ontology}");
        assert!(names("node_types").contains("Organization"), "{ontology}");
        assert!(names("edge_types").contains("CEO_OF"), "{ontology}");
        let edge = &ontology["edge_types"][0];
        assert_eq!(edge["source_types"][0]["name"], "Person", "{ontology}");
        assert_eq!(
            edge["target_types"][0]["name"], "Organization",
            "{ontology}"
        );
        for node_type in ontology["node_types"].as_array().unwrap() {
            assert!(node_type["properties"].is_array(), "{ontology}");
        }
    }
}

// 8 --------------------------------------------------------------------------------------------

#[test]
fn host_store_and_backend_are_read_from_the_environment() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let seed = world.file("seed.yaml", &text(&["example", "ekr-seed/2"]));
        let output = ekr()
            .env("EKR_HOST", &world.host)
            .env("EKR_STORE", world.store())
            .env("EKR_BACKEND", backend)
            .args(["seed", &seed])
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(world.ok(&["head"])["revision"], 0);
    }
}

#[test]
fn a_store_verb_without_its_configuration_is_a_usage_error_naming_flag_and_variable() {
    for (flag, var) in [
        ("--host", "EKR_HOST"),
        ("--store", "EKR_STORE"),
        ("--backend", "EKR_BACKEND"),
    ] {
        let mut args = vec![
            "--host",
            "host.json",
            "--store",
            "store",
            "--backend",
            "file",
        ];
        let at = args.iter().position(|a| *a == flag).unwrap();
        args.drain(at..at + 2);
        args.push("head");
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains(flag) && stderr.contains(var), "{stderr}");
        assert!(output.stdout.is_empty());
    }
}

// 9 --------------------------------------------------------------------------------------------

#[test]
fn every_verbs_help_names_its_input_format_and_points_at_the_examples() {
    let help = text(&["--help"]);
    let verbs: Vec<String> = help
        .lines()
        .skip_while(|line| !line.starts_with("Commands:"))
        .skip(1)
        .take_while(|line| line.starts_with("  "))
        .map(|line| line.split_whitespace().next().unwrap().to_owned())
        .filter(|verb| verb != "help")
        .collect();
    for verb in [
        "seed",
        "propose",
        "validate",
        "commit",
        "snapshot",
        "explain",
        "guide",
        "operations",
        "example",
        "mint",
        "head",
        "transactions",
        "ontology",
    ] {
        assert!(
            verbs.iter().any(|v| v == verb),
            "{verb} not in --help: {help}"
        );
    }
    assert!(help.contains("ekr guide"), "{help}");
    for verb in &verbs {
        let out = text(&[verb, "--help"]);
        assert!(
            FORMATS.iter().any(|format| out.contains(format)),
            "{verb} --help names no input format:\n{out}"
        );
        assert!(
            out.contains("ekr example") || out.contains("ekr operations"),
            "{verb} --help points at no example:\n{out}"
        );
    }
    for (verb, format) in [
        ("seed", "ekr-seed/2"),
        ("propose", "ekr.transaction-document/1"),
    ] {
        assert!(text(&[verb, "--help"]).contains(format), "{verb}");
    }
}

// 10 -------------------------------------------------------------------------------------------

/// The node id whose canonical name is `name` in a printed snapshot.
fn node_named(snapshot: &Value, name: &str) -> String {
    snapshot["graph"]["graph"]["nodes"]
        .as_object()
        .unwrap()
        .values()
        .find(|node| node["canonical_name"] == name)
        .unwrap_or_else(|| panic!("no node {name}"))["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn type_named(ontology: &Value, key: &str, name: &str) -> String {
    ontology[key]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["name"] == name)
        .unwrap_or_else(|| panic!("no {key} {name}"))["id"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn minted(kind: &str) -> String {
    json(&["mint", kind])["id"].as_str().unwrap().to_owned()
}

/// Replaces every `(from, to)` in a printed example; each `from` must occur.
fn fill(example: &str, replacements: &[(String, String)]) -> String {
    let mut text = example.to_owned();
    for (from, to) in replacements {
        assert!(text.contains(from.as_str()), "{from} not in {example}");
        text = text.replace(from.as_str(), to);
    }
    text
}

#[test]
fn the_retraction_example_runs_from_printed_strings_only_on_both_providers() {
    // An agent starts from `ekr --help`, which sends it to `ekr guide`; every verb and format
    // this case uses is one the guide names.
    let help = text(&["--help"]);
    assert!(help.contains("ekr guide"), "{help}");
    let guide = text(&["guide"]);
    for used in [
        "ekr example ekr.cli-host/1",
        "ekr example ekr-seed/2",
        "ekr seed",
        "ekr ontology",
        "ekr snapshot",
        "ekr mint",
        "ekr operations",
        "ekr propose",
        "ekr validate",
        "ekr commit",
        "ekr head",
        "ekr transactions",
    ] {
        assert!(
            guide.contains(used),
            "the guide does not name {used:?}:\n{guide}"
        );
    }
    let kinds = listed_kinds();
    for kind in [
        "CreateNode",
        "CreateEdge",
        "AddAssertion",
        "SupersedeAssertion",
        "RetractAssertion",
    ] {
        assert!(kinds.iter().any(|k| k == kind), "{kind} not listed");
    }

    for backend in BACKENDS {
        let world = World::new(backend);
        let host: Value = serde_json::from_str(&text(&["example", "ekr.cli-host/1"])).unwrap();
        let operator = host["context"]["operator"].as_str().unwrap().to_owned();
        world.seed_from_example();

        let ontology = world.ok(&["ontology"]);
        let organization = type_named(&ontology, "node_types", "Organization");
        let ceo_of = type_named(&ontology, "edge_types", "CEO_OF");
        let snapshot = world.ok(&["snapshot"]);
        let root = snapshot["graph"]["graph"]["root"]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        let (alice, bob, acme) = (
            node_named(&snapshot, "Alice"),
            node_named(&snapshot, "Bob"),
            node_named(&snapshot, "Acme"),
        );
        let evidence: Vec<String> = snapshot["graph"]["graph"]["evidence"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        assert!(evidence.len() >= 2, "{snapshot}");

        let create_node = example_operation("CreateNode");
        let GraphOperation::CreateNode(draft) = parse_one(&create_node) else {
            unreachable!()
        };
        let create_edge = example_operation("CreateEdge");
        let GraphOperation::CreateEdge(edge_draft) = parse_one(&create_edge) else {
            unreachable!()
        };
        let add = example_operation("AddAssertion");
        let GraphOperation::AddAssertion(assertion) = parse_one(&add) else {
            unreachable!()
        };
        let supersede = example_operation("SupersedeAssertion");
        let GraphOperation::SupersedeAssertion(supersession) = parse_one(&supersede) else {
            unreachable!()
        };
        let retract = example_operation("RetractAssertion");
        let GraphOperation::RetractAssertion(retraction) = parse_one(&retract) else {
            unreachable!()
        };
        let ekr_graph::Subject::Node(example_subject) = assertion.subject else {
            panic!("the AddAssertion example's subject is not a node")
        };
        let ekr_graph::Predicate::Relation(example_predicate) = assertion.predicate else {
            panic!("the AddAssertion example's predicate is not a relation")
        };
        let ekr_graph::Object::Node(example_object) = assertion.object else {
            panic!("the AddAssertion example's object is not a node")
        };
        let example_evidence = assertion.evidence.iter().next().unwrap().to_string();
        let example_from = assertion.valid_time.from.unwrap().millis().to_string();
        let s = |x: &dyn ToString| x.to_string();

        let assert_that = |id: &str, subject: &str, object: &str, from: i64, evidence: &str| {
            fill(
                &add,
                &[
                    (s(&assertion.id), id.to_owned()),
                    (s(&assertion.root_id), root.clone()),
                    (s(&example_subject), subject.to_owned()),
                    (s(&example_predicate), ceo_of.clone()),
                    (s(&example_object), object.to_owned()),
                    (example_evidence.clone(), evidence.to_owned()),
                    (s(&assertion.proposed_by), operator.clone()),
                    (example_from.clone(), from.to_string()),
                ],
            )
        };
        let drive = |name: &str, document: String| -> Value {
            let path = world.file(name, &document);
            let id = world.ok(&["propose", &path])["transaction_id"]
                .as_str()
                .unwrap()
                .to_owned();
            let head = world.ok(&["head"])["revision"]
                .as_u64()
                .unwrap()
                .to_string();
            let validated = world.ok(&["validate", &id, "--against", &head]);
            assert_eq!(
                validated["kind"], "Validated",
                "{backend} {name}: {validated}"
            );
            let committed = world.ok(&["commit", &id]);
            assert_eq!(
                committed["kind"], "Committed",
                "{backend} {name}: {committed}"
            );
            committed
        };

        // Revision 1: a new organisation, an edge to it, and two assertions.
        let (globex, edge, a_alice, a_bob_globex) = (
            minted("node"),
            minted("edge"),
            minted("assertion"),
            minted("assertion"),
        );
        let node_op = fill(
            &create_node,
            &[
                (s(&draft.id), globex.clone()),
                (s(&draft.root_id), root.clone()),
                (s(&draft.type_id), organization.clone()),
            ],
        );
        let edge_op = fill(
            &create_edge,
            &[
                (s(&edge_draft.id), edge.clone()),
                (s(&edge_draft.root_id), root.clone()),
                (s(&edge_draft.type_id), ceo_of.clone()),
                (s(&edge_draft.source), bob.clone()),
                (s(&edge_draft.target), globex.clone()),
            ],
        );
        let first = drive(
            "t1.yaml",
            document(
                &minted("transaction"),
                &operator,
                &[
                    node_op,
                    edge_op,
                    assert_that(&a_alice, &alice, &acme, JAN_2020, &evidence[0]),
                    assert_that(&a_bob_globex, &bob, &globex, JAN_2020, &evidence[1]),
                ],
                &[&evidence[0], &evidence[1]],
            ),
        );
        assert_eq!(first["result"]["revision"], 1);

        // Revision 2: Bob succeeds Alice at Acme on 2026-03-12.
        let a_bob = minted("assertion");
        let supersede_op = fill(
            &supersede,
            &[
                (s(&supersession.assertion), a_alice.clone()),
                (s(&supersession.by), a_bob.clone()),
                (s(&supersession.effective_from), MARCH_12.to_string()),
            ],
        );
        drive(
            "t2.yaml",
            document(
                &minted("transaction"),
                &operator,
                &[
                    assert_that(&a_bob, &bob, &acme, MARCH_12, &evidence[1]),
                    supersede_op,
                ],
                &[&evidence[1]],
            ),
        );

        // Revision 3: the Globex assertion is withdrawn.
        let retract_op = fill(
            &retract,
            &[(s(&retraction.assertion), a_bob_globex.clone())],
        );
        drive(
            "t3.yaml",
            document(&minted("transaction"), &operator, &[retract_op], &[]),
        );

        // Read back through printed verbs only.
        assert_eq!(world.ok(&["head"])["revision"], 3, "{backend}");
        let committed = world.ok(&["transactions", "--state", "Committed"]);
        assert_eq!(
            committed.as_array().unwrap().len(),
            3,
            "{backend}: {committed}"
        );
        let before = world.ok(&["snapshot", "--valid-at", "2026-03-11"]);
        let after = world.ok(&["snapshot", "--valid-at", "2026-03-12"]);
        let matching = |v: &Value| -> BTreeSet<String> {
            v["matching_assertions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a.as_str().unwrap().to_owned())
                .collect()
        };
        assert_eq!(
            matching(&before),
            BTreeSet::from([a_alice.clone()]),
            "{backend}"
        );
        assert_eq!(
            matching(&after),
            BTreeSet::from([a_bob.clone()]),
            "{backend}"
        );
        let assertions = &after["graph"]["graph"]["assertions"];
        assert!(
            assertions[&a_bob_globex]["lifecycle"]["Retracted"].is_object(),
            "{after}"
        );
        assert!(
            assertions[&a_alice]["lifecycle"]["Superseded"].is_object(),
            "{after}"
        );
        assert!(after["graph"]["graph"]["nodes"][&globex].is_object());
        assert!(after["graph"]["graph"]["edges"][&edge].is_object());
    }
}
