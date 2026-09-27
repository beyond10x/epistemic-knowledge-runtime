//! Adversary pass 2 on `story:data-free-graph-viewer`, role half: the correction that widens an
//! edge type's endpoints through `parents` (`docs/cli.md` § "Roles", step 1), driven through the
//! real `ekr view` against stores the unit's own tests do not use.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use serde_json::Value;
use serde_yaml_ng::Value as Yaml;

const P: &str = "00000000-0000-4000-8000-00000000";

fn manifest_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
}

fn readings() -> Yaml {
    let text =
        std::fs::read_to_string(manifest_dir().join("tests/fixtures/view/readings/seed.yaml"))
            .unwrap();
    serde_yaml_ng::from_str(&text).unwrap()
}

fn id(last: &str) -> String {
    format!("{P}{last}")
}

fn yaml(text: &str) -> Yaml {
    serde_yaml_ng::from_str(text).unwrap()
}

struct World {
    directory: tempfile::TempDir,
    host: PathBuf,
}

impl World {
    /// A store seeded from `seed` under the fixture host, or under the same host moved to
    /// validation profile v2 (`docs/cli.md` § "Evolve the schema", step 1) when `v2`.
    fn seeded(seed: &Yaml, v2: bool) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let mut host: Value = serde_json::from_str(
            &std::fs::read_to_string(manifest_dir().join("tests/fixtures/retraction/host.json"))
                .unwrap(),
        )
        .unwrap();
        if v2 {
            let profile = &mut host["authority"]["validation_profile"];
            profile["ruleset"] = Value::from("ekr.p2-deterministic/1");
            profile["application"] = Value::from("ekr.p2-apply/1");
        }
        let host_path = directory.path().join("host.json");
        std::fs::write(&host_path, serde_json::to_vec(&host).unwrap()).unwrap();
        let path = directory.path().join("seed.yaml");
        std::fs::write(&path, serde_yaml_ng::to_string(seed).unwrap()).unwrap();
        let world = Self {
            directory,
            host: host_path,
        };
        let seeded = world.ok(&["seed", &path.display().to_string()]);
        assert_eq!(seeded["result"]["revision"], 0, "{seeded}");
        world
    }

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            self.host.display().to_string(),
            "--store".to_owned(),
            self.directory.path().join("store").display().to_string(),
            "--backend".to_owned(),
            "file".to_owned(),
        ];
        args.extend(verb.iter().map(|arg| (*arg).to_owned()));
        args
    }

    fn run(&self, verb: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(self.args(verb))
            .stdin(Stdio::null())
            .output()
            .unwrap()
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{verb:?}: stdout {} stderr {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// Proposes, validates against `against` and commits the transaction document `text`.
    fn commit(&self, name: &str, text: &str, transaction: &str, against: &str) -> Value {
        let path = self.directory.path().join(name);
        std::fs::write(&path, text).unwrap();
        let path = path.display().to_string();
        assert_eq!(self.ok(&["propose", &path])["transaction_id"], transaction);
        let validated = self.ok(&["validate", transaction, "--against", against]);
        assert_eq!(validated["kind"], "Validated", "{validated}");
        self.ok(&["commit", transaction])
    }

    fn serve(&self) -> Server {
        let mut child = Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(self.args(&["view", "--port", "0"]))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        let printed: Value = serde_json::from_str(&line).unwrap();
        let address = printed["url"]
            .as_str()
            .unwrap()
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_owned();
        Server { child, address }
    }
}

struct Server {
    child: Child,
    address: String,
}

impl Drop for Server {
    fn drop(&mut self) {
        self.child.kill().ok();
        self.child.wait().ok();
    }
}

impl Server {
    /// `GET path` as `type id (last four digits) → role`.
    fn roles(&self, path: &str) -> BTreeMap<String, String> {
        let mut stream = TcpStream::connect(&self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        write!(
            stream,
            "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.address
        )
        .unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = String::from_utf8_lossy(&raw[..split]);
        assert!(head.starts_with("HTTP/1.1 200 "), "GET {path}: {head}");
        let body: Value = serde_json::from_slice(&raw[split + 4..]).unwrap();
        body["node_types"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| {
                (
                    entry["type_id"]
                        .as_str()
                        .unwrap()
                        .trim_start_matches(P)
                        .to_owned(),
                    entry["role"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    }
}

fn expect(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(id, role)| ((*id).to_owned(), (*role).to_owned()))
        .collect()
}

fn readings_roles() -> BTreeMap<String, String> {
    expect(&[
        ("1101", "observation"),
        ("1102", "event"),
        ("1103", "subject"),
        ("1104", "subject"),
    ])
}

fn node_type(last: &str, parents: &[&str], abstract_type: bool, property: Option<&str>) -> Yaml {
    let parents = if parents.is_empty() {
        "[]".to_owned()
    } else {
        parents.iter().map(|p| format!("\n  - {}", id(p))).collect()
    };
    let properties = property.map_or("{}".to_owned(), |p| {
        format!(
            "\n  {pid}:\n    id: {pid}\n    name: adversary_{p}\n    value_type:\n      \
             value_kind: String\n    cardinality: One\n    required: false\n    \
             constraints: []",
            pid = id(p)
        )
    });
    yaml(&format!(
        "id: {}\nname: Adversary{last}\nparents: {parents}\nproperties: {properties}\n\
         abstract_type: {abstract_type}\nlifecycle: null\noperations: {{}}\n",
        id(last)
    ))
}

fn edge_type(last: &str, sources: &[&str], targets: &[&str], symmetric: bool) -> Yaml {
    let list = |ids: &[&str]| {
        ids.iter()
            .map(|t| format!("\n  - {}", id(t)))
            .collect::<String>()
    };
    yaml(&format!(
        "id: {}\nname: ADVERSARY_{last}\nsource_types: {}\ntarget_types: {}\ncardinality: Many\n\
         properties: {{}}\ninverse: null\nsymmetric: {symmetric}\ntransitive: false\n",
        id(last),
        list(sources),
        list(targets)
    ))
}

/// `docs/cli.md` § "Roles" closes: "a type only on self-loops or on no edge type has no role".
/// The `readings` store's 1105 is on one edge type, `REFINES_MARKER` 1105 → 1105, and on nothing
/// else, and has no role at revision 0. Revision 1 is a schema change under profile v2 that
/// declares a new type 1106 with parent 1105 and touches neither 1105 nor any edge type.
///
/// Step 1 as corrected widens `REFINES_MARKER` to {1105, 1106} → {1105, 1106}: arcs 1105 → 1106
/// and 1106 → 1105, so both have a source and both are `subject`. The closing sentence says 1105
/// (only on a self-loop) and 1106 (on no edge type) have no role. The two statements of one doc
/// cannot both hold; this case holds the served roles to the closing sentence for as long as
/// the doc makes it, and goes green when either the sentence or the code moves.
#[test]
fn a_schema_change_under_a_self_loop_type_keeps_the_docs_closing_sentence_true() {
    let world = World::seeded(&readings(), true);
    let transaction = id("1811");
    let evolve = format!(
        "format: ekr.transaction-document/2\ntransaction:\n  id: {transaction}\n  proposer: {}\n  \
         operations:\n  - !DefineNodeType\n    id: {}\n    name: AdversaryRefinement\n    \
         parents:\n    - {}\n    properties: {{}}\n    abstract_type: false\n    lifecycle: null\n    \
         operations: {{}}\n  evidence: []\n  schema_version: {}\n",
        id("0101"),
        id("1106"),
        id("1105"),
        id("1009")
    );
    let committed = world.commit("evolve.yaml", &evolve, &transaction, "0");
    assert_eq!(committed["result"]["revision"], 1, "{committed}");

    let server = world.serve();
    assert_eq!(
        server.roles("/roles?revision=0"),
        readings_roles(),
        "revision 0 reads revision 0's ontology, which has no 1106"
    );
    let at_one = server.roles("/roles?revision=1");

    let doc = std::fs::read_to_string(manifest_dir().join("../../docs/cli.md")).unwrap();
    let doc = doc.split_whitespace().collect::<Vec<_>>().join(" ");
    let sentence = "a type only on self-loops or on no edge type has no role";
    assert!(
        !doc.contains(sentence) || (!at_one.contains_key("1105") && !at_one.contains_key("1106")),
        "docs/cli.md says {sentence:?}, yet at revision 1 the type only on a self-loop (1105) is \
         {:?} and the type on no edge type (1106) is {:?}: {at_one:?}",
        at_one.get("1105"),
        at_one.get("1106")
    );
}

/// Step 1 on a hierarchy the unit's suite has no shape for: a diamond two levels deep, an edge
/// type declared on the root, one declared on a middle type, and a `symmetric` edge type whose
/// declared endpoint is abstract.
///
/// Types: 9101 abstract root; 9102 and 9103 abstract, each parent 9101; 9104 concrete, parents
/// 9102 and 9103 (the diamond), timed; 9105 concrete; 9106 concrete; 9107 abstract; 9108
/// concrete, parent 9107, timed. Edge types: 9201 9101 → 9105; 9202 9106 → 9102; 9203 9105 →
/// 9107, symmetric.
///
/// By hand. Widened: 9201 {9101, 9102, 9103, 9104} → {9105}; 9202 {9106} → {9102, 9104}; 9203
/// {9105} → {9107, 9108} and back. Advancing: 9104 (timed, target 9105) and 9108 (timed, target
/// 9105 through the reverse arc). Roles: 9101 none (no source, target 9105 not advancing), 9102
/// subject, 9103 none, 9104 event, 9105 subject, 9106 observation (no source, target 9104
/// advancing), 9107 subject, 9108 event.
///
/// Kills: widening one level only (9104 loses 9105, so it is a subject and 9106 unplaced); a
/// reverse arc added for the declared lists only (9108 loses its target and is a subject).
#[test]
fn widening_is_transitive_through_a_diamond_and_reverses_a_symmetric_edge_to_descendants() {
    let mut seed = readings();
    seed["ontology"]["node_types"] = Yaml::Sequence(vec![
        node_type("9101", &[], true, None),
        node_type("9102", &["9101"], true, None),
        node_type("9103", &["9101"], true, None),
        node_type("9104", &["9102", "9103"], false, Some("9301")),
        node_type("9105", &[], false, None),
        node_type("9106", &[], false, None),
        node_type("9107", &[], true, None),
        node_type("9108", &["9107"], false, Some("9302")),
    ]);
    seed["ontology"]["edge_types"] = Yaml::Sequence(vec![
        edge_type("9201", &["9101"], &["9105"], false),
        edge_type("9202", &["9106"], &["9102"], false),
        edge_type("9203", &["9105"], &["9107"], true),
    ]);
    let root = id("1002");
    let node = |last: &str, of: &str| {
        (
            Yaml::String(id(last)),
            yaml(&format!(
                "id: {}\nroot_id: {root}\ntype_id: {}\ncanonical_name: Adversary node {last}\n\
                 aliases: []\ntype_state: null\nproperties: {{}}\n",
                id(last),
                id(of)
            )),
        )
    };
    let timed = |last: &str, subject: &str, property: &str| {
        (
            Yaml::String(id(last)),
            yaml(&format!(
                "id: {}\nroot_id: {root}\nsubject: !Node {}\npredicate: !Property {}\n\
                 object: !Value\n  value_kind: String\n  value: open\nevidence:\n- {}\n\
                 proposed_by: {}\nassessment: Proposed\nlifecycle: Active\nvalid_time:\n  \
                 from: 1767225600000\n  to: null\ntransaction_time:\n  recorded_from: 0\n  \
                 recorded_to: null\n",
                id(last),
                id(subject),
                id(property),
                id("1601"),
                id("0101")
            )),
        )
    };
    let graph = &mut seed["graph"]["graph"];
    graph["nodes"] = Yaml::Mapping(
        [node("9404", "9104"), node("9408", "9108")]
            .into_iter()
            .collect(),
    );
    graph["edges"] = Yaml::Mapping(serde_yaml_ng::Mapping::new());
    graph["assertions"] = Yaml::Mapping(
        [timed("9501", "9404", "9301"), timed("9502", "9408", "9302")]
            .into_iter()
            .collect(),
    );
    let world = World::seeded(&seed, false);
    assert_eq!(
        world.serve().roles("/roles"),
        expect(&[
            ("9102", "subject"),
            ("9104", "event"),
            ("9105", "subject"),
            ("9106", "observation"),
            ("9107", "subject"),
            ("9108", "event"),
        ])
    );
}
