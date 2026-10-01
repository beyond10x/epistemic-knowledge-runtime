//! Adversary pass on `task:one-event-type-rule` (wave extract-06 unit E), through the real
//! `ekr view` binary.
//!
//! The unit made `GET /roles` mark `event` exactly the types `Index::event_types` returns (the
//! overview's valid-time rule) and checks `event` before `observation`. These cases probe what a
//! consumer of `/roles` and `/overview` sees from that, on the `readings` and `sessions` fixture
//! seeds the unit edited, on those seeds as they stood at the unit's base (`40e11f625`), and across
//! revisions.

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

fn seed(store: &str) -> Yaml {
    let text = std::fs::read_to_string(
        manifest_dir().join(format!("tests/fixtures/view/{store}/seed.yaml")),
    )
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

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            host_file().display().to_string(),
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
    /// The body of a `GET path` answered 200.
    fn get(&self, path: &str) -> Vec<u8> {
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
        let head = std::str::from_utf8(&raw[..split]).unwrap();
        let status: u16 = head
            .split("\r\n")
            .next()
            .unwrap()
            .split_whitespace()
            .nth(1)
            .unwrap()
            .parse()
            .unwrap();
        let body = raw[split + 4..].to_vec();
        assert_eq!(
            status,
            200,
            "GET {path}: {}",
            String::from_utf8_lossy(&body)
        );
        body
    }

    /// `/roles…` as `type id (last four digits) → role`.
    fn roles(&self, path: &str) -> BTreeMap<String, String> {
        let body: Value = serde_json::from_slice(&self.get(path)).unwrap();
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

    /// `/overview`'s `roles.observation_type`, last four digits.
    fn observation_type(&self) -> Option<String> {
        let body: Value = serde_json::from_slice(&self.get("/overview")).unwrap();
        body["roles"]["observation_type"]
            .as_str()
            .map(|type_id| type_id.trim_start_matches(P).to_owned())
    }
}

fn expect(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(id, role)| ((*id).to_owned(), (*role).to_owned()))
        .collect()
}

fn assertions(seed: &mut Yaml) -> &mut serde_yaml_ng::Mapping {
    seed["graph"]["graph"]["assertions"]
        .as_mapping_mut()
        .unwrap()
}

fn assertion_mut<'a>(seed: &'a mut Yaml, last: &str) -> &'a mut Yaml {
    assertions(seed)
        .get_mut(Yaml::String(id(last)))
        .unwrap_or_else(|| panic!("assertion {last}"))
}

/// The `readings` seed as it stood at the unit's base: 1504 undated, no 1506.
fn readings_at_base() -> Yaml {
    let mut seed = seed("readings");
    assertion_mut(&mut seed, "1504")["valid_time"]["from"] = Yaml::Null;
    assert!(assertions(&mut seed)
        .remove(Yaml::String(id("1506")))
        .is_some());
    seed
}

/// The `sessions` seed as it stood at the unit's base: 2505 undated, no 2507.
fn sessions_at_base() -> Yaml {
    let mut seed = seed("sessions");
    assertion_mut(&mut seed, "2505")["valid_time"]["from"] = Yaml::Null;
    assert!(assertions(&mut seed)
        .remove(Yaml::String(id("2507")))
        .is_some());
    seed
}

/// Consumer-visible change the fixture edits hide. The base `readings` seed, whose event
/// (`Outage`, 1102) and observation (`Telemetry`, 1101) each carry one dated fact per node, served
/// `{1101 observation, 1102 event, 1103 subject, 1104 subject}` (`readings/roles.json`). Under the
/// one rule no node is judged (a node needs two dated facts or a timestamp-like value), so there is
/// no event type: no `event` and no `observation` at all. By hand: 1102 subject (1101 points at
/// it), 1103 subject, 1104 subject, 1101 and 1105 none.
#[test]
fn the_base_readings_seed_loses_every_event_and_observation() {
    let world = World::seeded(&readings_at_base());
    let server = world.serve();
    assert_eq!(
        server.roles("/roles"),
        expect(&[
            ("1102", "subject"),
            ("1103", "subject"),
            ("1104", "subject")
        ])
    );
    assert_eq!(server.observation_type(), None);
}

/// The same for the base `sessions` seed (`sessions/roles.json`: 2103 and 2104 event, 2105
/// observation). Each Session and Motion node has one dated fact, so no type is an event type. By
/// hand: 2101, 2102, 2103, 2104, 2106 and 2107 (symmetric `CORRESPONDS_WITH`) subject; 2105 none
/// (no sources, no event target); 2108 none.
#[test]
fn the_base_sessions_seed_loses_every_event_and_observation() {
    let world = World::seeded(&sessions_at_base());
    assert_eq!(
        world.serve().roles("/roles"),
        expect(&[
            ("2101", "subject"),
            ("2102", "subject"),
            ("2103", "subject"),
            ("2104", "subject"),
            ("2106", "subject"),
            ("2107", "subject"),
        ])
    );
}

/// `/roles` and `/overview` both name an "observation" for one store and disagree on it.
///
/// `ekr.graph-overview/1`'s `roles.observation_type` is by definition an event type (views.yaml:
/// "among the event types … the one with the most nodes"), and the viewer lays it out as the
/// observation. `/roles` now checks `event` before `observation`, so the overview's observation
/// type is always `event` in `/roles` and can never be its `observation`.
///
/// Store: `readings` with `Telemetry` made an event type (1502, the relation 1401 → 1402, dated
/// ten minutes after 1501) and as well linked as `Outage` (edge type 1206: 1101 → 1104, edge
/// 1703: 1401 → 1404). By hand, overview: event types 1101 and 1102, two neighbour types per node
/// each, one node each, so the lowest id, 1101, is the observation type. At the base, `/roles`
/// placed 1101 `observation` (timed, no sources, its target 1102 timed with targets) and agreed.
/// Now it places 1101 `event`, and no type at all is an `observation`.
#[test]
#[ignore = "adversary x6-e: /roles marks the overview's observation type `event`, never `observation`"]
fn the_overviews_observation_type_is_the_roles_observation() {
    let mut seed = seed("readings");
    assertion_mut(&mut seed, "1502")["valid_time"]["from"] =
        Yaml::Number(1_767_226_200_000_i64.into());
    seed["ontology"]["edge_types"]
        .as_sequence_mut()
        .unwrap()
        .push(yaml(&format!(
            "id: {}\nname: TELEMETRY_SITE\nsource_types:\n- {}\ntarget_types:\n- {}\n\
             cardinality: Many\nproperties: {{}}\ninverse: null\nsymmetric: false\n\
             transitive: false\n",
            id("1206"),
            id("1101"),
            id("1104")
        )));
    seed["graph"]["graph"]["edges"]
        .as_mapping_mut()
        .unwrap()
        .insert(
            Yaml::String(id("1703")),
            yaml(&format!(
                "id: {}\nroot_id: {}\ntype_id: {}\nsource: {}\ntarget: {}\nproperties: {{}}\n",
                id("1703"),
                id("1002"),
                id("1206"),
                id("1401"),
                id("1404")
            )),
        );
    let world = World::seeded(&seed);
    let server = world.serve();
    let observation = server.observation_type();
    assert_eq!(
        observation.as_deref(),
        Some("1101"),
        "the overview's observation type"
    );
    let roles = server.roles("/roles");
    assert_eq!(
        roles.get("1101").map(String::as_str),
        Some("observation"),
        "/roles of the overview's observation type: {roles:?}"
    );
}

/// The same type's role across revisions: the one rule is not monotone in the facts a revision
/// adds. Revision 1 adds a third dated fact about the `Outage` node two days after its first, so
/// that node's dated facts no longer lie within one hour: it is judged and not instant, and 1102
/// stops being an event type. By hand at revision 1: 1102 subject, 1103 subject, 1104 subject,
/// and 1101, pointing at no event, loses `observation`. `docs/cli.md` § Roles names only the
/// other direction ("a revision that adds a dated fact can move a type from `subject` to
/// `event`").
#[test]
fn a_revision_adding_a_late_dated_fact_moves_an_event_type_to_subject() {
    let world = World::seeded(&seed("readings"));
    let transaction = id("1803");
    let path = world.directory.path().join("late.yaml");
    std::fs::write(
        &path,
        format!(
            "format: ekr.transaction-document/1\ntransaction:\n  id: {transaction}\n  \
             proposer: {proposer}\n  operations:\n  - !AddAssertion\n    id: {assertion}\n    \
             root_id: {root}\n    subject: !Node {node}\n    predicate: !Property {property}\n    \
             object: !Value\n      value_kind: String\n      value: full\n    evidence:\n    \
             - {evidence}\n    proposed_by: {proposer}\n    assessment: Proposed\n    \
             lifecycle: Active\n    valid_time:\n      from: 1767398400000\n      to: null\n    \
             transaction_time:\n      recorded_from: 0\n      recorded_to: null\n  evidence:\n  \
             - {evidence}\n",
            proposer = id("0101"),
            assertion = id("1507"),
            root = id("1002"),
            node = id("1402"),
            property = id("1302"),
            evidence = id("1601"),
        ),
    )
    .unwrap();
    let path = path.display().to_string();
    assert_eq!(
        world.ok(&["propose", &path])["transaction_id"],
        transaction.as_str()
    );
    assert_eq!(
        world.ok(&["validate", &transaction, "--against", "0"])["kind"],
        "Validated"
    );
    assert_eq!(world.ok(&["commit", &transaction])["result"]["revision"], 1);
    let server = world.serve();
    assert_eq!(
        server.roles("/roles?revision=0"),
        expect(&[
            ("1101", "observation"),
            ("1102", "event"),
            ("1103", "subject"),
            ("1104", "subject"),
        ]),
        "revision 0"
    );
    assert_eq!(
        server.roles("/roles?revision=1"),
        expect(&[
            ("1102", "subject"),
            ("1103", "subject"),
            ("1104", "subject")
        ]),
        "revision 1"
    );
}

/// `/roles` bytes stay what the base served for the `readings` seed: compact JSON, keys in
/// `format`, `revision`, `node_types` order, entries by type id, `type_id` before `role`.
#[test]
fn the_readings_roles_bytes_are_unchanged() {
    let world = World::seeded(&seed("readings"));
    let body = world.serve().get("/roles");
    let expected = format!(
        "{{\"format\":\"ekr.view-roles/1\",\"revision\":0,\"node_types\":[\
         {{\"type_id\":\"{}\",\"role\":\"observation\"}},\
         {{\"type_id\":\"{}\",\"role\":\"event\"}},\
         {{\"type_id\":\"{}\",\"role\":\"subject\"}},\
         {{\"type_id\":\"{}\",\"role\":\"subject\"}}]}}",
        id("1101"),
        id("1102"),
        id("1103"),
        id("1104")
    );
    assert_eq!(String::from_utf8(body).unwrap(), expected);
}
