//! `story:data-free-graph-viewer`: `ekr view` serves `GET /roles[?revision=N]`, the node-type
//! roles derived from a store's event types, observation type and edge types alone, driven end to
//! end through the real binary.
//!
//! Two fixture stores under `tests/fixtures/view/` carry different ontologies — different type,
//! edge-type, property and entity names and a different shape — and each documents the answer the
//! rule gives it in `roles.json`, by type id. How each type gets its role, by the rule in
//! `docs/cli.md` § "Roles". "Event type" is the one rule's (`task:one-event-type-rule`): a type
//! whose node has two dated facts within an hour of each other is one; a node with one dated fact
//! is not judged. The observation type is the overview's: among the event types whose neighbour
//! types per node reach the mean, the one with the most nodes, ties to the lowest id.
//!
//! `readings`, five types:
//!
//! | type id (last digits) | event type | neighbour types per node | pointed at by | role |
//! |---|---|---|---|---|
//! | 1101 | yes: two dated facts ten minutes apart | 2 (1102, 1104) | — | observation: ties 1102 and has the lower id |
//! | 1102 | yes: three dated facts within ten minutes | 2 (1101, 1103) | 1101 | event |
//! | 1103 | no: one dated fact | — | 1102 | subject |
//! | 1104 | no | — | 1101, 1102, 1103 | subject |
//! | 1105 | no | — | itself only | none: a self-loop is no arc |
//!
//! `sessions`, eight types:
//!
//! | type id (last digits) | event type | neighbour types per node | pointed at by | role |
//! |---|---|---|---|---|
//! | 2101 | no | — | 2103, 2105 | subject |
//! | 2102 | no | — | 2101, 2103, 2104, 2107 | subject |
//! | 2103 | yes: two dated facts ten minutes apart | 2 (2104, 2106) | 2104, 2105 | event |
//! | 2104 | yes: two dated facts ten minutes apart | 2 (2103, 2105) | 2105 | event |
//! | 2105 | yes: two dated facts five minutes apart | 3 (2101, 2104, 2106) | — | observation: alone above the mean of 7/3 |
//! | 2106 | no: one dated fact | — | 2103, 2105 | subject |
//! | 2107 | no | — | 2102 (symmetric) | subject: only because a symmetric edge type is read both ways |
//! | 2108 | no | — | — | none: no arc at all |
//!
//! `readings/propose-timed-serial.yaml` adds a second dated fact about the 1103 node, half an hour
//! after its first, so at revision 1 that type is an event type and an event; 1101 still ties
//! for the observation type with the lowest id (`roles-revision-1.json`).

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use ekr::host::CliHostConfigurationV1;
use ekr_kernel::Runtime;
use serde_json::Value;
use serde_yaml_ng::Value as Yaml;

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const STORES: [&str; 2] = ["readings", "sessions"];
const T_SERIAL: &str = "00000000-0000-4000-8000-000000001801";

fn manifest_dir() -> PathBuf {
    PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR")
            .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time"),
    )
}

fn fixture(store: &str, name: &str) -> PathBuf {
    manifest_dir()
        .join("tests/fixtures/view")
        .join(store)
        .join(name)
}

fn host_file() -> PathBuf {
    manifest_dir().join("tests/fixtures/retraction/host.json")
}

fn documented(store: &str, name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(fixture(store, name)).unwrap()).unwrap()
}

/// One store on one provider, in its own directory.
struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn new(backend: &'static str) -> Self {
        Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        }
    }

    /// A store seeded from `seed`, at revision 0.
    fn seeded(backend: &'static str, seed: &std::path::Path) -> Self {
        let world = Self::new(backend);
        let seed = seed.display().to_string();
        assert_eq!(world.ok(&["seed", &seed])["result"]["revision"], 0);
        world
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
            host_file().display().to_string(),
            "--store".to_owned(),
            self.store().display().to_string(),
            "--backend".to_owned(),
            self.backend.to_owned(),
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
            "{} {verb:?}: stderr {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn runtime(&self) -> Runtime {
        let host = CliHostConfigurationV1::from_json(&std::fs::read(host_file()).unwrap()).unwrap();
        match self.backend {
            "file" => {
                Runtime::file_existing(&self.store(), &host.tenant, host.context, host.authority)
            }
            _ => {
                Runtime::sqlite_existing(&self.store(), &host.tenant, host.context, host.authority)
            }
        }
        .unwrap()
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
        let mut server = Server {
            child,
            address: String::new(),
        };
        let printed: Value = serde_json::from_str(&line)
            .unwrap_or_else(|error| panic!("`ekr view` printed no JSON line ({error}): {line:?}"));
        server.address = printed["url"]
            .as_str()
            .unwrap()
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_owned();
        server
    }
}

/// A running `ekr view`, killed when dropped.
struct Server {
    child: Child,
    address: String,
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
        Response::parse(&raw)
    }

    fn get(&self, path: &str) -> Response {
        self.request("GET", path)
    }

    /// `GET /roles…`, answered 200 as JSON, parsed.
    fn roles(&self, path: &str) -> Value {
        let response = self.get(path);
        assert_eq!(
            response.status,
            200,
            "GET {path}: {}",
            String::from_utf8_lossy(&response.body)
        );
        assert_eq!(
            response.header("content-type"),
            Some("application/json"),
            "{path}"
        );
        response.assert_plain(path);
        serde_json::from_slice(&response.body).unwrap()
    }
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
    fn parse(raw: &[u8]) -> Self {
        let split = raw
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap_or_else(|| panic!("no header end in {:?}", String::from_utf8_lossy(raw)));
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
        Self {
            status,
            headers,
            body: raw[split + 4..].to_vec(),
        }
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// The headers every `ekr view` response carries, and none it never does.
    fn assert_plain(&self, what: &str) {
        assert_eq!(
            self.header("x-content-type-options"),
            Some("nosniff"),
            "{what}"
        );
        assert_eq!(self.header("cache-control"), Some("no-store"), "{what}");
        assert_eq!(self.header("connection"), Some("close"), "{what}");
        for (name, value) in &self.headers {
            assert!(
                name != "set-cookie" && !name.starts_with("access-control-"),
                "{what}: {name}: {value}"
            );
        }
    }
}

/// The `/roles` body's own shape: the format literal, the revision, entries ordered by `type_id`
/// with one of the three roles each.
fn assert_shape(roles: &Value, revision: u64, what: &str) {
    assert_eq!(roles["format"], "ekr.view-roles/1", "{what}");
    assert_eq!(roles["revision"], revision, "{what}");
    let ids: Vec<&str> = roles["node_types"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            assert!(
                matches!(
                    entry["role"].as_str(),
                    Some("event" | "subject" | "observation")
                ),
                "{what}: {entry}"
            );
            entry["type_id"].as_str().unwrap()
        })
        .collect();
    let mut sorted = ids.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(ids, sorted, "{what}: ordered by type_id, each once");
}

/// Tests 2 and 4: each fixture store, served beside the other by its own `ekr view`, answers
/// `/roles` with the assignment its fixture documents — at the head and at revision 0 — and
/// `/projection` with its own projection, on both providers.
#[test]
fn each_fixture_store_is_served_its_documented_roles_beside_the_other_on_both_providers() {
    for backend in BACKENDS {
        let worlds: Vec<(&str, World)> = STORES
            .iter()
            .map(|&store| (store, World::seeded(backend, &fixture(store, "seed.yaml"))))
            .collect();
        let projections: Vec<Vec<u8>> = worlds
            .iter()
            .map(|(_, world)| ekr_views::project(&world.runtime(), None).unwrap().bytes)
            .collect();
        assert_ne!(
            projections[0], projections[1],
            "{backend}: two different stores"
        );
        let servers: Vec<Server> = worlds.iter().map(|(_, world)| world.serve()).collect();
        for (((store, _), server), projection) in worlds.iter().zip(&servers).zip(&projections) {
            let expected = documented(store, "roles.json");
            assert_shape(&expected, 0, &format!("{store}/roles.json"));
            for path in ["/roles", "/roles?revision=0"] {
                let served = server.roles(path);
                assert_shape(&served, 0, &format!("{backend} {store} {path}"));
                assert_eq!(served, expected, "{backend} {store} GET {path}");
            }
            let served = server.get("/projection");
            assert_eq!(served.status, 200, "{backend} {store}");
            assert_eq!(
                &served.body, projection,
                "{backend} {store}: its projection"
            );
        }
    }
}

/// The body follows the revision asked for, and refuses what `/projection` refuses, the same way.
#[test]
fn roles_follow_the_revision_asked_for_and_refuse_as_the_projection_does() {
    let world = World::seeded("file", &fixture("readings", "seed.yaml"));
    let propose = fixture("readings", "propose-timed-serial.yaml")
        .display()
        .to_string();
    assert_eq!(world.ok(&["propose", &propose])["transaction_id"], T_SERIAL);
    assert_eq!(
        world.ok(&["validate", T_SERIAL, "--against", "0"])["kind"],
        "Validated"
    );
    assert_eq!(world.ok(&["commit", T_SERIAL])["result"]["revision"], 1);
    let server = world.serve();

    let at_one = documented("readings", "roles-revision-1.json");
    assert_shape(&at_one, 1, "roles-revision-1.json");
    assert_ne!(at_one, documented("readings", "roles.json"));
    assert_eq!(server.roles("/roles"), at_one, "the head");
    assert_eq!(server.roles("/roles?revision=1"), at_one, "revision 1");
    assert_eq!(
        server.roles("/roles?revision=0"),
        documented("readings", "roles.json"),
        "revision 0"
    );

    for (path, status, refusal) in [
        ("/roles?revision=9", 404, "ekr.views.RevisionNotFound"),
        ("/roles?revision=x", 400, "invalid-query"),
        ("/roles?at=0", 400, "invalid-query"),
    ] {
        let refused = server.get(path);
        assert_eq!(refused.status, status, "GET {path}");
        refused.assert_plain(path);
        let body: Value = serde_json::from_slice(&refused.body).unwrap();
        assert_eq!(body["refusal"], refusal, "GET {path}: {body}");
        let projection = server.get(&path.replace("/roles", "/projection"));
        assert_eq!(
            projection.status, status,
            "the projection refuses {path} alike"
        );
    }
    for path in ["/roles/", "/roles/0"] {
        assert_eq!(server.get(path).status, 404, "GET {path}");
    }
    let refused = server.request("POST", "/roles");
    assert_eq!(refused.status, 405, "POST /roles");
    assert_eq!(refused.header("allow"), Some("GET"));
}

/// Every string a seed document names a thing with: node-type, edge-type and property names
/// (`name`), entity names (`canonical_name`) and aliases, wherever they sit.
fn names(value: &Yaml, into: &mut Vec<String>) {
    match value {
        Yaml::Mapping(mapping) => {
            for (key, value) in mapping {
                match (key.as_str(), value) {
                    (Some("name" | "canonical_name"), Yaml::String(name)) => {
                        into.push(name.clone());
                    }
                    (Some("aliases"), Yaml::Sequence(aliases)) => {
                        into.extend(aliases.iter().filter_map(Yaml::as_str).map(str::to_owned));
                    }
                    _ => names(value, into),
                }
            }
        }
        Yaml::Sequence(items) => items.iter().for_each(|item| names(item, into)),
        Yaml::Tagged(tagged) => names(&tagged.value, into),
        _ => {}
    }
}

/// `value` with every name [`names`] finds replaced through `renamed`.
fn rename(value: &mut Yaml, renamed: &BTreeMap<String, String>) {
    match value {
        Yaml::Mapping(mapping) => {
            for (key, value) in mapping.iter_mut() {
                match (key.as_str(), value) {
                    (Some("name" | "canonical_name"), Yaml::String(name)) => {
                        *name = renamed[name.as_str()].clone();
                    }
                    (Some("aliases"), Yaml::Sequence(aliases)) => {
                        for alias in aliases {
                            if let Yaml::String(alias) = alias {
                                *alias = renamed[alias.as_str()].clone();
                            }
                        }
                    }
                    (_, value) => rename(value, renamed),
                }
            }
        }
        Yaml::Sequence(items) => items.iter_mut().for_each(|item| rename(item, renamed)),
        Yaml::Tagged(tagged) => rename(&mut tagged.value, renamed),
        _ => {}
    }
}

/// Test 3: a copy of each store with every type, edge-type, property and entity name replaced —
/// by names that sort in the opposite order — is served the same roles, id for id.
#[test]
fn renaming_every_name_in_a_store_leaves_its_roles_unchanged_id_for_id() {
    for store in STORES {
        let text = std::fs::read_to_string(fixture(store, "seed.yaml")).unwrap();
        let mut document: Yaml = serde_yaml_ng::from_str(&text).unwrap();
        let mut found = Vec::new();
        names(&document, &mut found);
        found.sort();
        found.dedup();
        let seed: Value = serde_json::to_value(&document).unwrap();
        let ontology = &seed["ontology"];
        let declared = ontology["node_types"].as_array().unwrap().len()
            + ontology["edge_types"].as_array().unwrap().len()
            + seed["graph"]["graph"]["nodes"].as_object().unwrap().len();
        assert!(
            found.len() > declared,
            "{store}: {} names found for {declared} types, edge types and nodes, and properties \
             besides: {found:?}",
            found.len()
        );
        let count = found.len();
        let renamed: BTreeMap<String, String> = found
            .iter()
            .enumerate()
            .map(|(at, name)| (name.clone(), format!("q{:03}", count - at)))
            .collect();
        rename(&mut document, &renamed);
        let copy = serde_yaml_ng::to_string(&document).unwrap();
        for name in &found {
            assert!(
                !copy.contains(name.as_str()),
                "{store}: {name:?} survived the rename"
            );
        }
        let directory = tempfile::tempdir().unwrap();
        let copy_path = directory.path().join("renamed-seed.yaml");
        std::fs::write(&copy_path, &copy).unwrap();

        let original = World::seeded("file", &fixture(store, "seed.yaml"));
        let renamed_world = World::seeded("file", &copy_path);
        let projection = ekr_views::project(&renamed_world.runtime(), None)
            .unwrap()
            .bytes;
        let projection = String::from_utf8(projection).unwrap();
        for name in &found {
            assert!(
                !projection.contains(&format!("\"{name}\"")),
                "{store}: the renamed store's projection still names {name:?}"
            );
        }
        let (served_original, served_copy) = {
            let first = original.serve();
            let second = renamed_world.serve();
            (first.roles("/roles"), second.roles("/roles"))
        };
        assert_eq!(served_original, documented(store, "roles.json"), "{store}");
        assert_eq!(
            served_copy, served_original,
            "{store}: the renamed copy's roles, id for id"
        );
    }
}

/// `task:one-event-type-rule`, through the binary: on each fixture store, and on `readings` at
/// revision 1, the `event` and `observation` entries of `/roles` together are exactly the
/// overview's event types (`roles.types[].event`, which the timeline reads) and exactly the event
/// types `ekr ocel` writes when no type is named, there is at least one, and the `observation`
/// entry is the overview's `roles.observation_type`.
#[test]
fn roles_the_overview_and_ocel_name_the_same_event_types() {
    use std::collections::BTreeSet;

    /// `/roles`' event and observation entries together, and its observation entries; the
    /// overview's event types, and its observation type.
    struct Served {
        roles: BTreeSet<String>,
        observed: BTreeSet<String>,
        overview: BTreeSet<String>,
        observation: BTreeSet<String>,
    }

    fn served_events(server: &Server, revision: u64) -> Served {
        let roles = server.roles(&format!("/roles?revision={revision}"));
        let with = |role: &str| -> BTreeSet<String> {
            roles["node_types"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|entry| entry["role"] == role)
                .map(|entry| entry["type_id"].as_str().unwrap().to_owned())
                .collect()
        };
        let observed = with("observation");
        let roles = with("event").union(&observed).cloned().collect();
        let overview = server.get(&format!("/overview?revision={revision}"));
        assert_eq!(overview.status, 200, "GET /overview?revision={revision}");
        let overview: Value = serde_json::from_slice(&overview.body).unwrap();
        let observation = overview["roles"]["observation_type"]
            .as_str()
            .map(str::to_owned)
            .into_iter()
            .collect();
        let overview = overview["roles"]["types"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|timing| timing["event"] == true)
            .map(|timing| timing["type"].as_str().unwrap().to_owned())
            .collect();
        Served {
            roles,
            observed,
            overview,
            observation,
        }
    }

    fn ocel_events(world: &World, revision: u64) -> BTreeSet<String> {
        let exported = world.ok(&["ocel", "--revision", &revision.to_string()]);
        exported["ocel"]["eventTypes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|event_type| event_type["name"].as_str().unwrap().to_owned())
            .collect()
    }

    let readings = World::seeded("file", &fixture("readings", "seed.yaml"));
    let propose = fixture("readings", "propose-timed-serial.yaml")
        .display()
        .to_string();
    assert_eq!(
        readings.ok(&["propose", &propose])["transaction_id"],
        T_SERIAL
    );
    assert_eq!(
        readings.ok(&["validate", T_SERIAL, "--against", "0"])["kind"],
        "Validated"
    );
    assert_eq!(readings.ok(&["commit", T_SERIAL])["result"]["revision"], 1);
    let sessions = World::seeded("file", &fixture("sessions", "seed.yaml"));
    for (store, world, revisions) in [
        ("readings", &readings, &[0_u64, 1][..]),
        ("sessions", &sessions, &[0][..]),
    ] {
        let server = world.serve();
        for &revision in revisions {
            let Served {
                roles,
                observed,
                overview,
                observation,
            } = served_events(&server, revision);
            assert_eq!(
                observed, observation,
                "{store} at {revision}: /roles' observation and the overview's"
            );
            let ocel = ocel_events(world, revision);
            assert!(!roles.is_empty(), "{store} at {revision}: some event type");
            assert_eq!(
                roles, overview,
                "{store} at {revision}: /roles and the overview"
            );
            assert_eq!(roles, ocel, "{store} at {revision}: /roles and ekr ocel");
        }
    }
}
