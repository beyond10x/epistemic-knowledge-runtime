//! Adversary, wave 2026-10-20a unit U1, pass 1: `story:supersede-node-review-fixes` D6 driven
//! through both surfaces it names — `ekr apply-extraction` and `ekr_sdk::extraction::apply_with` —
//! against the contract the unit wrote for them.
//!
//! Design § 110 (Extraction), as this unit wrote it: "A refusal names the superseded node, its
//! type and the kept node only where that end caused it: the property its type does not declare,
//! or the end of a relation its edge type does not connect." The SDK's module documentation:
//! "`ApplyOptions::strict` refuses the whole document for it instead … — but a replacement with
//! nothing to replace, which depends on the store, is skipped under it too."
//!
//! Every case drives fresh binary processes against a store seeded from `ekr example ekr-seed/2`
//! with aliases edited into its nodes, on both providers. Invented fixtures only.

use std::path::PathBuf;
use std::process::Output;

use serde_json::Value;

const BACKENDS: [&str; 2] = ["file", "sqlite"];
const ALICE: &str = "00000000-0000-4000-8000-000000000301";
const ACME: &str = "00000000-0000-4000-8000-000000000303";
/// The example seed's evidence entry "Alice is CEO of Acme."
const STATEMENT: &str = "00000000-0000-4000-8000-000000000401";
/// The evidence item every fact of [`extraction`] cites.
const FACT_EVIDENCE: &str = "00000000-0000-4000-8000-000000000421";

/// A fresh `ekr` process with no inherited `EKR_*` configuration.
fn ekr() -> std::process::Command {
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
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

fn edit(document: &str, from: &str, to: &str) -> String {
    assert!(document.contains(from), "{from:?} is not in {document}");
    document.replacen(from, to, 1)
}

/// The example seed with aliases on its three nodes: Alice and Bob are people, Acme an
/// organization known as `acme`.
fn aliased_seed() -> String {
    let seed = text(&["example", "ekr-seed/2"]);
    let seed = edit(
        &seed,
        "canonical_name: Alice\n        aliases: []\n",
        "canonical_name: Alice\n        aliases: [alice]\n",
    );
    let seed = edit(
        &seed,
        "canonical_name: Bob\n        aliases: []\n",
        "canonical_name: Bob\n        aliases: [bob]\n",
    );
    edit(
        &seed,
        "canonical_name: Acme\n        aliases: []\n",
        "canonical_name: Acme\n        aliases: [acme]\n",
    )
}

fn operator() -> String {
    let host: Value = serde_json::from_str(&text(&["example", "ekr.cli-host/1"])).unwrap();
    host["context"]["operator"].as_str().unwrap().to_owned()
}

/// An extraction document of `entities` (list items, or empty) and `facts` (list items), every
/// fact citing [`FACT_EVIDENCE`].
fn extraction(entities: &str, facts: &str) -> String {
    let payload = b"Carol runs the company the registry lists.".to_vec();
    let bytes: Vec<String> = payload.iter().map(u8::to_string).collect();
    let entities = if entities.is_empty() {
        String::new()
    } else {
        format!("entities:\n{entities}")
    };
    format!(
        "format: ekr.extraction-document/1\n{entities}facts:\n{facts}evidence:\n- evidence:\n    \
         id: {FACT_EVIDENCE}\n    source: !HumanStatement\n      identity: Registry extract\n    \
         content_hash: {}\n    extracted_by: {}\n    observed_at: 1775001600000\n    \
         confidence: 9000\n  payload: [{}]\n",
        ekr_sdk::document::payload_hash(&payload),
        operator(),
        bytes.join(", ")
    )
}

struct World {
    directory: tempfile::TempDir,
    backend: &'static str,
}

impl World {
    fn seeded(backend: &'static str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("host.json"),
            text(&["example", "ekr.cli-host/1"]),
        )
        .unwrap();
        let world = Self { directory, backend };
        world.file("seed.yaml", &aliased_seed());
        world.ok(&["seed", "seed.yaml"]);
        world
    }

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().join("store"),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn file(&self, name: &str, contents: &str) -> String {
        std::fs::write(self.directory.path().join(name), contents).unwrap();
        name.to_owned()
    }

    fn run(&self, verb: &[&str]) -> Output {
        let mut command = ekr();
        command
            .current_dir(self.directory.path())
            .arg("--host")
            .arg(self.directory.path().join("host.json"))
            .arg("--store")
            .arg(self.store())
            .args(["--backend", self.backend])
            .args(verb);
        command.output().unwrap()
    }

    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// Commits `!SupersedeNode {node, by}` on the example statement.
    fn supersede(&self, node: &str, by: &str) {
        let minted: Value = serde_json::from_str(&text(&["mint", "transaction"])).unwrap();
        let id = minted["id"].as_str().unwrap().to_owned();
        let document = self.file(
            "supersede.yaml",
            &format!(
                "format: ekr.transaction-document/2\ntransaction:\n  id: {id}\n  proposer: {}\n  \
                 operations:\n  - !SupersedeNode\n    node: {node}\n    by: {by}\n    reason: \
                 recorded twice\n    evidence:\n    - {STATEMENT}\n  evidence:\n  - {STATEMENT}\n",
                operator()
            ),
        );
        self.ok(&["propose", &document]);
        assert_eq!(self.ok(&["validate", &id])["kind"], "Validated");
        assert_eq!(self.ok(&["commit", &id])["kind"], "Committed");
    }

    /// `document` applied through `ekr_sdk::extraction::apply_with` under `options`, over a child
    /// `ekr session` of this store.
    fn apply_through_the_sdk(
        &self,
        document: &str,
        options: &ekr_sdk::extraction::ApplyOptions,
    ) -> Result<ekr_sdk::extraction::ExtractionReport, String> {
        let binary = ekr_sdk::binary::EkrBinary::open(env!("CARGO_BIN_EXE_ekr")).unwrap();
        let store = ekr_sdk::session::StoreConfig {
            host: self.directory.path().join("host.json"),
            store: self.store(),
            backend: match self.backend {
                "file" => ekr_sdk::session::Backend::File,
                _ => ekr_sdk::session::Backend::Sqlite,
            },
        };
        let mut session = ekr_sdk::session::ProcessSession::start(
            &binary,
            store,
            ekr_sdk::session::SessionOptions::default(),
        )
        .unwrap();
        let mirrored = ekr_sdk::document::ExtractionDocument::from_yaml(document).unwrap();
        let report = ekr_sdk::extraction::apply_with(
            &mut session,
            &mirrored,
            operator().parse().unwrap(),
            options,
        )
        .map_err(|error| error.to_string());
        session.close().unwrap();
        report
    }
}

/// D6, the note: Acme, an organization, is superseded by Alice, a person. A relation from the
/// organization `acme` as `CEO_OF` the person `bob` lands its subject on Alice, a Person, which
/// `CEO_OF` takes as its source: the subject end connects, and the relation is refused for its
/// object, Bob, whom no supersession reaches. The unit's own case
/// (`resolve_cli.rs::a_refusal_names_a_supersession_only_where_that_end_caused_it`) shows the SDK
/// naming no supersession for it. `ekr apply-extraction` reads the same document with the engine's
/// reader (`check_facts_at`), hands its refusal to the SDK as `ApplyOptions::refused`, and the
/// SDK's `named_by_cause` notes the subject's supersession because the key's *own* type,
/// Organization, does not connect (`crates/ekr-sdk/src/extraction.rs`, the `!connects(own) ||`
/// disjunct) — though the refusal is typed by the kept one, which does. Same store, same document:
/// the verb and the SDK must answer one refusal.
#[test]
fn the_verb_names_no_supersession_where_its_end_connects() {
    let document = extraction(
        "",
        "- !Relation\n  subject: {node_type: Organization, aliases: [acme]}\n  relation: CEO_OF\n  \
         object: {node_type: Person, aliases: [bob]}\n  evidence: [00000000-0000-4000-8000-000000000421]\n",
    );
    for backend in BACKENDS {
        let world = World::seeded(backend);
        world.supersede(ACME, ALICE);
        let sdk = world
            .apply_through_the_sdk(&document, &ekr_sdk::extraction::ApplyOptions::default())
            .unwrap();
        assert_eq!(sdk.rejected.len(), 1, "{backend}: {sdk:?}");
        let by_sdk = sdk.rejected[0].refusal.clone().unwrap_or_default();

        let file = world.file("extraction.yaml", &document);
        let report = world.ok(&["apply-extraction", &file]);
        let rejected = report["rejected"].as_array().unwrap();
        assert_eq!(rejected.len(), 1, "{backend}: {report}");
        let by_verb = rejected[0]["refusal"]
            .as_str()
            .unwrap_or_default()
            .to_owned();

        assert!(
            by_verb.starts_with("extraction-relation-ends"),
            "{backend}: {by_verb}"
        );
        assert_eq!(
            by_verb, by_sdk,
            "{backend}: `ekr apply-extraction` and `ekr_sdk::extraction::apply` refuse one fact of \
             one document against one store with different refusals; the verb names the \
             supersession of a subject end that connects (Alice is a Person, CEO_OF's source)"
        );
    }
}

/// D6 under `ApplyOptions::strict`: a fact whose key alone resolves to no node (`acme-holdings`)
/// but whose group — joined through the entity `[acme-holdings, acme]` — resolves through Acme's
/// supersession to Alice, a Person, which does not declare `legal_name`. The SDK refuses the fact
/// before anything is written (the regrouped check this unit added), and under `strict` it skips
/// it and applies the rest instead of refusing the document, though the module documentation
/// exempts only a replacement with nothing to replace from `strict`. A per-key landing of the
/// same kind refuses the document under `strict` (`Plan::of`, the strict branch); the regrouped
/// one does not.
#[test]
fn strict_refuses_the_document_for_a_fact_its_group_lands_on_another_type() {
    let document = extraction(
        "- {node_type: Organization, aliases: [acme-holdings, acme]}\n",
        "- !Property\n  subject: {node_type: Organization, aliases: [acme-holdings]}\n  property: \
         legal_name\n  value: {value_kind: String, value: Acme Holdings}\n  evidence: \
         [00000000-0000-4000-8000-000000000421]\n",
    );
    for backend in BACKENDS {
        let world = World::seeded(backend);
        world.supersede(ACME, ALICE);
        let strict = ekr_sdk::extraction::ApplyOptions::default().strict();
        let answer = world.apply_through_the_sdk(&document, &strict);
        assert!(
            answer.is_err(),
            "{backend}: `strict` skipped a fact the SDK refused before writing and applied the \
             rest: {answer:?}"
        );
    }
}
