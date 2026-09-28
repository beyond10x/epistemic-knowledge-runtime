//! Adversary pass 1 on `story:data-free-graph-viewer`, role half: `docs/cli.md` § "Roles" applied
//! by hand to stores the unit's own tests do not use, each served by the real `ekr view` binary.
//!
//! Every store here is the `readings` fixture seed with one structural change, or a store with a
//! type hierarchy, written to a temporary directory. Expected roles are the doc's rule applied by
//! hand, stated beside each case.

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

fn host_file() -> PathBuf {
    manifest_dir().join("tests/fixtures/retraction/host.json")
}

fn readings_text() -> String {
    std::fs::read_to_string(manifest_dir().join("tests/fixtures/view/readings/seed.yaml")).unwrap()
}

fn readings() -> Yaml {
    serde_yaml_ng::from_str(&readings_text()).unwrap()
}

fn id(last: &str) -> String {
    format!("{P}{last}")
}

fn yaml(text: &str) -> Yaml {
    serde_yaml_ng::from_str(text).unwrap()
}

struct World {
    directory: tempfile::TempDir,
}

impl World {
    fn seeded(seed: &Yaml) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("seed.yaml");
        std::fs::write(&path, serde_yaml_ng::to_string(seed).unwrap()).unwrap();
        let world = Self { directory };
        let seeded = world.ok(&["seed", &path.display().to_string()]);
        assert_eq!(seeded["result"]["revision"], 0, "{seeded}");
        world
    }

    fn store(&self) -> PathBuf {
        self.directory.path().join("store")
    }

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            host_file().display().to_string(),
            "--store".to_owned(),
            self.store().display().to_string(),
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

struct Response {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Response {
    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}

impl Server {
    fn request(&self, method: &str, path: &str) -> Response {
        let mut stream = TcpStream::connect(&self.address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(60)))
            .unwrap();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            self.address
        )
        .unwrap();
        let mut raw = Vec::new();
        stream.read_to_end(&mut raw).unwrap();
        let split = raw.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = std::str::from_utf8(&raw[..split]).unwrap();
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        let headers = lines
            .map(|line| {
                let (name, value) = line.split_once(':').unwrap();
                (name.trim().to_ascii_lowercase(), value.trim().to_owned())
            })
            .collect();
        Response {
            status,
            headers,
            body: raw[split + 4..].to_vec(),
        }
    }

    /// `/roles…` as `type id (last four digits) → role`.
    fn roles(&self, path: &str) -> BTreeMap<String, String> {
        let response = self.request("GET", path);
        assert_eq!(
            response.status,
            200,
            "GET {path}: {}",
            String::from_utf8_lossy(&response.body)
        );
        let body: Value = serde_json::from_slice(&response.body).unwrap();
        body["node_types"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| {
                let type_id = entry["type_id"].as_str().unwrap();
                (
                    type_id.trim_start_matches(P).to_owned(),
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

/// The roles `readings/roles.json` documents.
fn readings_roles() -> BTreeMap<String, String> {
    expect(&[
        ("1101", "observation"),
        ("1102", "event"),
        ("1103", "subject"),
        ("1104", "subject"),
    ])
}

fn edge_types(seed: &mut Yaml) -> &mut Vec<Yaml> {
    seed["ontology"]["edge_types"]
        .as_sequence_mut()
        .expect("edge_types is a list")
}

fn edge_type(last: &str, sources: &[&str], targets: &[&str]) -> Yaml {
    let list = |ids: &[&str]| {
        if ids.is_empty() {
            "[]".to_owned()
        } else {
            ids.iter()
                .map(|t| format!("\n  - {}", id(t)))
                .collect::<String>()
        }
    };
    yaml(&format!(
        "id: {}\nname: ADVERSARY_{last}\nsource_types: {}\ntarget_types: {}\ncardinality: Many\n\
         properties: {{}}\ninverse: null\nsymmetric: false\ntransitive: false\n",
        id(last),
        list(sources),
        list(targets)
    ))
}

fn assertion_mut<'a>(seed: &'a mut Yaml, last: &str) -> &'a mut Yaml {
    let key = Yaml::String(id(last));
    seed["graph"]["graph"]["assertions"]
        .as_mapping_mut()
        .unwrap()
        .get_mut(&key)
        .unwrap()
}

/// Rule step 4 and the observation row: "at least one target is advancing". Type 1101 gets a
/// second target, 1104, which is not advancing (untimed). By hand: 1101 has no sources and one
/// advancing target (1102), so it stays `observation`; nothing else moves. Kills the mutant that
/// reads the row as "every target is advancing" — no unit fixture has an observation candidate
/// with a non-advancing target (`readings` 1101 has one target, `sessions` 2105 two advancing).
#[test]
fn an_observation_needs_one_advancing_target_not_every_one() {
    let mut seed = readings();
    edge_types(&mut seed).push(edge_type("1206", &["1101"], &["1104"]));
    let world = World::seeded(&seed);
    assert_eq!(world.serve().roles("/roles"), readings_roles());
}

/// Rule step 3: timed means "a valid time with `from` or `to` set". The only timed assertion
/// about a 1102 node gets `from: null` and keeps its `to`. By hand nothing moves. Kills the
/// mutant that reads only `from` — no unit fixture carries a valid time bounded by `to` alone.
#[test]
fn a_valid_time_bounded_by_to_alone_makes_its_type_timed() {
    let mut seed = readings();
    assertion_mut(&mut seed, "1503")["valid_time"]["from"] = Yaml::Null;
    let world = World::seeded(&seed);
    assert_eq!(world.serve().roles("/roles"), readings_roles());
}

/// Rule step 3: "whatever its assessment or lifecycle". The unit's own revision-1 transaction adds
/// the only timed assertion about a 1103 node, which commit accepts; revision 2 retracts it. By
/// hand 1103 is still timed at revision 2, so it stays `event` as `roles-revision-1.json` has it.
/// Every unit fixture assertion is active, so a mutant that skips retracted assertions is not
/// caught by the unit's suite.
#[test]
fn a_retracted_timed_assertion_still_makes_its_type_timed() {
    let world = World::seeded(&readings());
    let serial = manifest_dir()
        .join("tests/fixtures/view/readings/propose-timed-serial.yaml")
        .display()
        .to_string();
    let t1 = id("1801");
    assert_eq!(
        world.ok(&["propose", &serial])["transaction_id"],
        t1.as_str()
    );
    assert_eq!(
        world.ok(&["validate", &t1, "--against", "0"])["kind"],
        "Validated"
    );
    assert_eq!(world.ok(&["commit", &t1])["result"]["revision"], 1);
    let t2 = id("1802");
    let retract = world.directory.path().join("retract.yaml");
    std::fs::write(
        &retract,
        format!(
            "format: ekr.transaction-document/1\ntransaction:\n  id: {t2}\n  proposer: {}\n  \
             operations:\n  - !RetractAssertion\n    assertion: {}\n    reason: adversary \
             withdrawal\n  evidence: []\n",
            id("0101"),
            id("1505")
        ),
    )
    .unwrap();
    let retract = retract.display().to_string();
    assert_eq!(
        world.ok(&["propose", &retract])["transaction_id"],
        t2.as_str()
    );
    assert_eq!(
        world.ok(&["validate", &t2, "--against", "1"])["kind"],
        "Validated"
    );
    assert_eq!(world.ok(&["commit", &t2])["result"]["revision"], 2);
    let mut at_one = readings_roles();
    at_one.insert("1103".to_owned(), "event".to_owned());
    let server = world.serve();
    assert_eq!(server.roles("/roles?revision=1"), at_one, "revision 1");
    assert_eq!(
        server.roles("/roles?revision=2"),
        at_one,
        "revision 2, after the retraction"
    );
}

/// A cycle: an edge type 1102 → 1101 closes one. By hand: 1101 now has a source, so it is not an
/// observation; it is timed and has a target, so `event`. 1102 `event`, 1103 and 1104
/// `subject`, 1105 still none (self-loop only). (An edge type with an empty `source_types` is
/// refused at seed, `ekr.kernel.InvalidSeed`, so that edge input is not reachable by seeding.)
#[test]
fn a_cycle_follows_the_rule_by_hand() {
    let mut seed = readings();
    edge_types(&mut seed).push(edge_type("1207", &["1102"], &["1101"]));
    let world = World::seeded(&seed);
    assert_eq!(
        world.serve().roles("/roles"),
        expect(&[
            ("1101", "event"),
            ("1102", "event"),
            ("1103", "subject"),
            ("1104", "subject"),
        ])
    );
}

/// Test 3 renames names and keeps every id, so it cannot see a dependence on the ids' order.
/// Here the five type ids are relabelled so that they sort in the opposite order (1101 ↔ 1105,
/// 1102 ↔ 1104). By hand the roles follow the relabelling: 1105 observation, 1104 event, 1103
/// and 1102 subject, 1101 none.
#[test]
fn relabelling_the_type_ids_moves_the_roles_with_them() {
    let mut text = readings_text();
    for (from, to) in [
        ("1101", "x1"),
        ("1102", "x2"),
        ("1104", "1102"),
        ("1105", "1101"),
        ("x1", "1105"),
        ("x2", "1104"),
    ] {
        text = text.replace(&id(from), &id(to));
    }
    let seed: Yaml = serde_yaml_ng::from_str(&text).unwrap();
    let world = World::seeded(&seed);
    let served = world.serve();
    assert_eq!(
        served.roles("/roles"),
        expect(&[
            ("1102", "subject"),
            ("1103", "subject"),
            ("1104", "event"),
            ("1105", "observation"),
        ])
    );
}

/// The story's context: "the same viewer reads a store with any ontology". An ontology that
/// declares its edge types on abstract parents — which the kernel accepts, since an endpoint is
/// checked with `conforms_to` (`crates/ekr-kernel/src/validate/types.rs:508`) — carries every
/// node, edge and timed assertion on concrete children. By the doc's rule ("`parents` are not
/// consulted") the concrete types get no arc and no role, and only the abstract parents, which
/// hold no node, are placed. So a reading → incident → asset store of this shape shows no event.
///
/// Types: 9101 abstract reading, 9102 concrete reading (parent 9101), 9103 abstract incident,
/// 9104 concrete incident (parent 9103, timed), 9105 asset. Edge types 9201: 9101 → 9103 and
/// 9202: 9103 → 9105. Asserted here: the concrete incident, timed and pointing on through its
/// parent's edge type, is an `event`, and the concrete reading pointing at it an `observation`.
#[test]
fn a_store_whose_edge_types_name_abstract_parents_places_its_concrete_timed_types() {
    let mut seed = readings();
    let node_type = |last: &str, parents: &[&str], abstract_type: bool, property: Option<&str>| {
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
    };
    seed["ontology"]["node_types"] = Yaml::Sequence(vec![
        node_type("9101", &[], true, None),
        node_type("9102", &["9101"], false, None),
        node_type("9103", &[], true, None),
        node_type("9104", &["9103"], false, Some("9301")),
        node_type("9105", &[], false, None),
    ]);
    seed["ontology"]["edge_types"] = Yaml::Sequence(vec![
        edge_type("9201", &["9101"], &["9103"]),
        edge_type("9202", &["9103"], &["9105"]),
    ]);
    let root = id("1002");
    let node = |last: &str, of: &str| {
        yaml(&format!(
            "id: {}\nroot_id: {root}\ntype_id: {}\ncanonical_name: Adversary node {last}\n\
             aliases: []\ntype_state: null\nproperties: {{}}\n",
            id(last),
            id(of)
        ))
    };
    let edge = |last: &str, of: &str, source: &str, target: &str| {
        yaml(&format!(
            "id: {}\nroot_id: {root}\ntype_id: {}\nsource: {}\ntarget: {}\nproperties: {{}}\n",
            id(last),
            id(of),
            id(source),
            id(target)
        ))
    };
    let mapping = |entries: Vec<(&str, Yaml)>| {
        Yaml::Mapping(
            entries
                .into_iter()
                .map(|(last, value)| (Yaml::String(id(last)), value))
                .collect(),
        )
    };
    let graph = &mut seed["graph"]["graph"];
    graph["nodes"] = mapping(vec![
        ("9402", node("9402", "9102")),
        ("9404", node("9404", "9104")),
        ("9405", node("9405", "9105")),
    ]);
    graph["edges"] = mapping(vec![
        ("9701", edge("9701", "9201", "9402", "9404")),
        ("9702", edge("9702", "9202", "9404", "9405")),
    ]);
    graph["assertions"] = mapping(vec![(
        "9501",
        yaml(&format!(
            "id: {}\nroot_id: {root}\nsubject: !Node {}\npredicate: !Property {}\nobject: !Value\n  \
             value_kind: String\n  value: open\nevidence:\n- {}\nproposed_by: {}\n\
             assessment: Proposed\nlifecycle: Active\nvalid_time:\n  from: 1767225600000\n  \
             to: null\ntransaction_time:\n  recorded_from: 0\n  recorded_to: null\n",
            id("9501"),
            id("9404"),
            id("9301"),
            id("1601"),
            id("0101")
        )),
    )]);
    let world = World::seeded(&seed);
    let roles = world.serve().roles("/roles");
    assert_eq!(
        roles.get("9104").map(String::as_str),
        Some("event"),
        "the concrete, timed incident type that points on: {roles:?}"
    );
    assert_eq!(
        roles.get("9102").map(String::as_str),
        Some("observation"),
        "the concrete reading type that points at it: {roles:?}"
    );
}

/// The endpoint's refusals, beyond the three the unit tests: for each query, `/roles` answers
/// with the status, refusal code and headers `/projection` answers with; a HEAD is 405 with
/// `Allow: GET` like any other method.
#[test]
fn every_query_is_answered_by_roles_as_by_projection() {
    let world = World::seeded(&readings());
    let server = world.serve();
    for query in [
        "",
        "?",
        "?&",
        "?revision=",
        "?revision=-1",
        "?revision=+0",
        "?revision=00",
        "?revision=0&revision=0",
        "?revision=0&x=1",
        "?REVISION=0",
        "?revision=18446744073709551615",
        "?revision=18446744073709551616",
    ] {
        let roles = server.request("GET", &format!("/roles{query}"));
        let projection = server.request("GET", &format!("/projection{query}"));
        assert_eq!(roles.status, projection.status, "{query}");
        for header in ["content-type", "x-content-type-options", "cache-control"] {
            assert_eq!(
                roles.header(header),
                projection.header(header),
                "{query}: {header}"
            );
        }
        if roles.status != 200 {
            assert_eq!(
                roles.body, projection.body,
                "{query}: the same refusal body"
            );
        }
    }
    let head = server.request("HEAD", "/roles");
    assert_eq!(head.status, 405);
    assert_eq!(head.header("allow"), Some("GET"));
}

/// `docs/cli.md` § "Roles", added by this unit: "400 `invalid-query` for any query but
/// `revision=N`". A query of one `&` names no revision, and `revision=+0` is not `revision=N`
/// with `N` a revision number as the doc writes it; both are answered 200 (the shared `revision`
/// parser skips empty pairs and takes `u64::from_str`, which accepts a leading `+`).
#[test]
fn the_documented_invalid_query_refusal_covers_every_query_but_revision_n() {
    let world = World::seeded(&readings());
    let server = world.serve();
    for query in ["?&", "?revision=+0"] {
        let served = server.request("GET", &format!("/roles{query}"));
        assert_eq!(
            served.status,
            400,
            "GET /roles{query}: {}",
            String::from_utf8_lossy(&served.body)
        );
    }
}
