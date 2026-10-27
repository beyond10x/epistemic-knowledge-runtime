//! Adversary pass 1, wave 2026-10-19a, `story:supersede-node`: the consumer case through
//! extraction. A duplicate Organization, known as `globex-corp`, is superseded by the
//! Organization the store held first, `Globex`. `ekr resolve` of the duplicate's key answers the
//! kept node (ADR 0017 decision 5, `docs/cli.md` "A node superseded by another": "so the next run
//! that resolves the old key lands on the node kept"). The next extraction run that names the old
//! key and replaces its legal name therefore replaces the kept node's active legal name; the SDK's
//! planner says of its `holds` check that it answers for "the nodes `ekr resolve` would answer
//! with" (`crates/ekr-sdk/src/extraction.rs`, `Plan::of`).

use std::path::PathBuf;

use ekr_sdk::binary::EkrBinary;
use ekr_sdk::document::{
    EvidenceId, GraphRootId, NodeDraft, NodeId, NodeSupersession, Operation, TypeId,
};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use serde_json::Value;

const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
const LEGAL_NAME: &str = "00000000-0000-4000-8000-000000000801";

/// The example seed's own types: Globex is new, with the legal name Globex Corporation, citing
/// item 0403 (`crates/ekr/tests/extraction_cli.rs`, `KNOWN_TYPES`, its Organization fact).
const GLOBEX: &str = "format: ekr.extraction-document/1
facts:
- !Property
  subject: {node_type: Organization, aliases: [Globex]}
  property: legal_name
  value: {value_kind: String, value: Globex Corporation}
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

    fn json(&self, args: &[&str]) -> Value {
        let output = ekr()
            .arg("--host")
            .arg(self.file("host.json"))
            .arg("--store")
            .arg(self.file("store"))
            .args(["--backend", self.backend])
            .args(args)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{args:?}: stderr {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    fn apply(&self, document: &str) -> Value {
        let path = self.file("extraction.yaml");
        std::fs::write(&path, document).unwrap();
        self.json(&["apply-extraction", path.to_str().unwrap()])
    }

    fn commit(&self, operations: Vec<Operation>) {
        let binary = EkrBinary::open(binary()).unwrap();
        let mut session =
            ProcessSession::start(&binary, self.config(), SessionOptions::default()).unwrap();
        let report = ekr_sdk::batch::Batcher::new(OPERATOR.parse().unwrap())
            .commit(&mut session, &[operations])
            .unwrap();
        session.close().unwrap();
        assert_eq!(report.committed.len(), 1, "{report:?}");
    }

    /// The active legal names of the node `node`.
    fn legal_names(&self, node: &str) -> Vec<Value> {
        let snapshot = self.json(&["snapshot"]);
        snapshot["graph"]["graph"]["assertions"]
            .as_object()
            .unwrap()
            .values()
            .filter(|assertion| {
                assertion["subject"]["Node"] == node
                    && assertion["predicate"]["Property"] == LEGAL_NAME
                    && assertion["lifecycle"] == "Active"
            })
            .map(|assertion| assertion["object"]["Value"]["value"].clone())
            .collect()
    }
}

#[test]
fn adv_an_extraction_replacing_a_value_under_a_superseded_key_replaces_the_kept_nodes_value() {
    for backend in ["file", "sqlite"] {
        let world = World::new(backend);
        world.apply(GLOBEX);
        let snapshot = world.json(&["snapshot"]);
        let graph = &snapshot["graph"]["graph"];
        let (globex, organization) = graph["nodes"]
            .as_object()
            .unwrap()
            .iter()
            .find(|(_, node)| node["canonical_name"] == "Globex")
            .map(|(id, node)| (id.clone(), node["type_id"].as_str().unwrap().to_owned()))
            .unwrap();
        let root: GraphRootId = graph["root"]["id"].as_str().unwrap().parse().unwrap();
        let type_id: TypeId = organization.parse().unwrap();

        // The duplicate an import wrote, then its supersession by Globex.
        let mut duplicate = NodeDraft::new(root, type_id, "Globex Corp.");
        duplicate.aliases = vec!["globex-corp".to_owned()];
        let duplicate_id = duplicate.id;
        world.commit(vec![Operation::CreateNode(duplicate)]);
        let evidence: EvidenceId = "00000000-0000-4000-8000-000000000403".parse().unwrap();
        world.commit(vec![Operation::SupersedeNode(NodeSupersession::new(
            duplicate_id,
            globex.parse::<NodeId>().unwrap(),
            "imported twice",
            [evidence],
        ))]);

        // `ekr resolve` of the old key answers the kept node.
        let reference = world.file("reference.yaml");
        std::fs::write(
            &reference,
            format!("type_id: {organization}\naliases:\n- globex-corp\n"),
        )
        .unwrap();
        let resolved = world.json(&["resolve", reference.to_str().unwrap()]);
        assert_eq!(resolved["kind"], "Resolved", "{backend}: {resolved}");
        assert_eq!(
            resolved["node_id"],
            globex.as_str(),
            "{backend}: {resolved}"
        );

        // The next run names the old key and replaces the legal name.
        let payload = b"Globex Corp. is now Globex Inc.".to_vec();
        let bytes: Vec<String> = payload.iter().map(u8::to_string).collect();
        let replacing = format!(
            "format: ekr.extraction-document/1
facts:
- !Property
  subject: {{node_type: Organization, aliases: [globex-corp]}}
  property: legal_name
  replaces: true
  value: {{value_kind: String, value: Globex Inc}}
  evidence: [00000000-0000-4000-8000-000000000410]
evidence:
- evidence:
    id: 00000000-0000-4000-8000-000000000410
    source: !HumanStatement
      identity: Registry extract
    content_hash: {}
    extracted_by: {OPERATOR}
    observed_at: 1775001600000
    confidence: 9000
  payload: [{}]
",
            ekr_sdk::document::payload_hash(&payload),
            bytes.join(", ")
        );
        let report = world.apply(&replacing);
        assert_eq!(
            report["rejected"],
            serde_json::json!([]),
            "{backend}: the replacement under the old key is refused: {report}"
        );
        assert_eq!(
            world.legal_names(&globex),
            vec![Value::String("Globex Inc".into())],
            "{backend}: {report}"
        );
    }
}
