//! `task:sdk-types-the-check-reads`: the SDK's typed `quality`, `rejections` and `code-names`
//! reads over `ekr session` and one-shot `ekr`.
//!
//! * Each typed value is the verb's document: read through a session and as a one-shot `ekr`
//!   process it is one value, and that value written back is exactly the JSON the session
//!   answered and the one-shot process printed, on the file and the SQLite provider.
//! * Every `ekr.store-quality/1` and `ekr.code-names/1` document the `ekr-views` conformance
//!   fixture stores render — each built through the real kernel by
//!   `crates/ekr-views/tests/support/fixtures.rs`, at every revision — reads exactly into its
//!   typed value, and so does the `ekr.rejections/1` document real `ekr` prints. A field one of
//!   the formats gains without an SDK update is left out of that write, so it fails here by its
//!   JSON pointer; the same field is ignored when read, so a newer `ekr` does not break an older
//!   consumer.
//! * `code-names` through the SDK reports a planted store name with its file and line, and flags
//!   a store name that is also a runtime word.
//!
//! Every case that runs `ekr` drives the binary built from this checkout (`ekr_path`), by path.

#![cfg(unix)]

#[path = "../../ekr-views/tests/support/fixtures.rs"]
#[allow(dead_code, clippy::all, clippy::pedantic)]
mod fixtures;

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fmt::Debug;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ekr_core::{NodeId, RevisionNumber, TransactionId};
use ekr_sdk::binary::EkrBinary;
use ekr_sdk::read::{
    AssertionQuality, CodeNameFinding, CodeNameKind, CodeNameMatch, CodeNameMode, CodeNames,
    CodeNamesMeta, OneShotReader, PropertyQuality, QualityMeta, ReadError, Reader,
    RejectedTransaction, RejectionIssue, Rejections, SharedName, StoreQuality,
};
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{Request, Transport};
use ekr_views::SourceText;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

const ROOT: &str = "00000000-0000-4000-8000-000000000002";
const ORGANIZATION: &str = "00000000-0000-4000-8000-000000000202";
const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
/// The example seed's Acme, an Organization.
const ACME: &str = "00000000-0000-4000-8000-000000000303";
/// The Organization committed as revision 1, named with a runtime word.
const PLANTED: &str = "00000000-0000-4000-8000-00000000f301";
/// The transaction that commits it.
const KEPT: &str = "00000000-0000-4000-8000-00000000f103";
/// Rejected against revision 0: aliases for two nodes no revision holds, so two issues.
const EARLY: &str = "00000000-0000-4000-8000-00000000f101";
/// Rejected against revision 1: a retraction of an assertion no revision holds.
const LATE: &str = "00000000-0000-4000-8000-00000000f102";
/// A store name that is also one of the runtime's own words (an assessment kind).
const RUNTIME_WORD: &str = "Accepted";

/// The consumer source `code-names` reads: `Acme` quoted on line 2, the runtime word on line 3.
const READER_TS: &str = "// reads the store\nconst company = \"Acme\";\nconst state = \
                         'Accepted';\nconst plain = \"not a store name\";\n";
/// A second file, `Alice` on line 1.
const OTHER_RS: &str = "let person = \"Alice\";\n";
/// A file whose path starts like a flag, `Bob` on line 1.
const DASHED: &str = "`Bob`\n";

#[test]
fn typed_word_mode_preserves_comments_identifiers_and_transport_parity() {
    for backend in [Backend::File, Backend::Sqlite] {
        let world = World::planted(backend);
        world.file(
            "words.rs",
            "// Acme\nlet Acme = AcmeSuffix;\nlet Accepted = 1;\n",
        );
        let mut session = world.session();
        let mut one_shot = OneShotReader::new(&binary(), world.store(), world.options());
        let typed = Reader::new(&mut session)
            .code_names_with_mode(["words.rs"], None, CodeNameMode::Words)
            .unwrap();
        assert_eq!(typed.meta.mode, Some(CodeNameMode::Words));
        assert_eq!(typed.meta.findings, 3);
        assert_eq!(typed.meta.runtime_word_findings, 1);
        let verb = vec!["code-names".into(), "--words".into(), "words.rs".into()];
        one_value(
            "whole words across transports",
            &typed,
            &one_shot
                .code_names_with_mode(["words.rs"], None, CodeNameMode::Words)
                .unwrap(),
            &ok(&mut session, &verb),
            &world.one_shot(&verb),
        );
        let literals = Reader::new(&mut session)
            .code_names(["words.rs"], None)
            .unwrap();
        assert_eq!(literals.meta.findings, 0);
        assert_eq!(literals.meta.mode, None);
    }
}

// ---- the binary and a store under the example host ---------------------------------------------

/// The workspace root, found at run time from the directory holding `Cargo.lock`.
fn workspace_root() -> PathBuf {
    let manifest = std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    manifest
        .ancestors()
        .find(|directory| directory.join("Cargo.lock").is_file())
        .expect("a workspace root above the crate")
        .to_path_buf()
}

/// The `ekr` binary of this checkout, built once per test process through cargo.
fn ekr_path() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let cargo = std::env::var_os("CARGO").unwrap_or_else(|| OsString::from("cargo"));
        let output = std::process::Command::new(cargo)
            .arg("build")
            .arg("--manifest-path")
            .arg(workspace_root().join("Cargo.toml"))
            .args(["--locked", "-p", "ekr", "--bin", "ekr"])
            .arg("--message-format=json-render-diagnostics")
            .stderr(std::process::Stdio::inherit())
            .output()
            .expect("running cargo build -p ekr");
        assert!(output.status.success(), "cargo build -p ekr failed");
        String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|message| {
                message["reason"] == "compiler-artifact" && message["target"]["name"] == "ekr"
            })
            .find_map(|message| message["executable"].as_str().map(PathBuf::from))
            .expect("cargo named the ekr executable it built")
    })
}

fn binary() -> EkrBinary {
    EkrBinary::open(ekr_path()).expect("the built ekr meets the SDK's minimum")
}

/// One provider in its own directory: the example host, the seed, two rejections, one commit
/// and the consumer's source files.
struct World {
    directory: tempfile::TempDir,
    backend: Backend,
}

impl World {
    fn planted(backend: Backend) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        world.file(
            "host.json",
            &world.text(&["example", "ekr.cli-host/1"], false),
        );
        world.file("seed.yaml", &world.text(&["example", "ekr-seed/2"], false));
        world.run(&["seed", "seed.yaml"], true);

        world.propose(
            EARLY,
            "  - !AddAlias\n    node: 00000000-0000-4000-8000-00000000f398\n    alias: \
             first-unknown\n  - !AddAlias\n    node: 00000000-0000-4000-8000-00000000f399\n    \
             alias: second-unknown\n",
        );
        assert_eq!(world.validate(EARLY, 0)["kind"], "Rejected");
        world.propose(
            KEPT,
            &format!(
                "  - !CreateNode\n    id: {PLANTED}\n    root_id: {ROOT}\n    type_id: \
                 {ORGANIZATION}\n    canonical_name: {RUNTIME_WORD}\n    properties: {{}}\n    \
                 aliases: []\n"
            ),
        );
        assert_eq!(world.validate(KEPT, 0)["kind"], "Validated");
        world.run(&["commit", KEPT], true);
        world.propose(
            LATE,
            "  - !RetractAssertion\n    assertion: 00000000-0000-4000-8000-00000000f599\n    \
             reason: no such assertion\n",
        );
        assert_eq!(world.validate(LATE, 1)["kind"], "Rejected");

        std::fs::create_dir_all(world.directory.path().join("src")).unwrap();
        std::fs::create_dir_all(world.directory.path().join("lib")).unwrap();
        world.file("src/reader.ts", READER_TS);
        world.file("lib/other.rs", OTHER_RS);
        world.file("-dashed.ts", DASHED);
        world
    }

    fn file(&self, name: &str, contents: &str) {
        std::fs::write(self.directory.path().join(name), contents).unwrap();
    }

    fn store(&self) -> StoreConfig {
        let store = match self.backend {
            Backend::File => "store",
            Backend::Sqlite => "state.db",
        };
        StoreConfig {
            host: self.directory.path().join("host.json"),
            store: self.directory.path().join(store),
            backend: self.backend,
        }
    }

    fn options(&self) -> SessionOptions {
        SessionOptions {
            current_dir: Some(self.directory.path().to_path_buf()),
            ..SessionOptions::default()
        }
    }

    fn session(&self) -> ProcessSession {
        ProcessSession::start(&binary(), self.store(), self.options()).unwrap()
    }

    /// `ekr [store options] <args>` in this world's directory, exit 0: its stdout.
    fn run(&self, args: &[&str], on_store: bool) -> Vec<u8> {
        let mut command = std::process::Command::new(ekr_path());
        command.env_clear().current_dir(self.directory.path());
        if on_store {
            let store = self.store();
            command
                .arg("--host")
                .arg(&store.host)
                .arg("--store")
                .arg(&store.store)
                .args(["--backend", store.backend.as_str()]);
        }
        let output = command.args(args).output().unwrap();
        assert!(
            output.status.success(),
            "{args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        output.stdout
    }

    fn text(&self, args: &[&str], on_store: bool) -> String {
        String::from_utf8(self.run(args, on_store)).unwrap()
    }

    /// `ekr <verb>` on this world's store, one-shot: its stdout as JSON.
    fn one_shot(&self, verb: &[String]) -> Value {
        let verb: Vec<&str> = verb.iter().map(String::as_str).collect();
        serde_json::from_slice(&self.run(&verb, true)).unwrap()
    }

    fn propose(&self, id: &str, operations: &str) {
        let document = format!(
            "format: ekr.transaction-document/2\ntransaction:\n  id: {id}\n  proposer: \
             {OPERATOR}\n  operations:\n{operations}  evidence: []\n"
        );
        let file = format!("{id}.yaml");
        self.file(&file, &document);
        self.run(&["propose", &file], true);
    }

    fn validate(&self, id: &str, against: u64) -> Value {
        serde_json::from_slice(
            &self.run(&["validate", id, "--against", &against.to_string()], true),
        )
        .unwrap()
    }
}

/// The document of an exit-0 answer.
fn ok(transport: &mut dyn Transport, argv: &[String]) -> Value {
    let reply = transport.request(&Request::new(argv.to_vec())).unwrap();
    assert_eq!(reply.exit, 0, "{argv:?}: {}", reply.stderr);
    reply.document.unwrap()
}

/// `verb` with `--name value` for each given value.
fn argv(verb: &str, flags: &[(&str, Option<u64>)]) -> Vec<String> {
    let mut argv = vec![verb.to_owned()];
    for (name, value) in flags {
        if let Some(value) = value {
            argv.extend([format!("--{name}"), value.to_string()]);
        }
    }
    argv
}

// ---- exactness: read, and written back unchanged ----------------------------------------------

/// Every leaf of `value` by its JSON pointer.
fn leaves(value: &Value, at: &str, into: &mut BTreeMap<String, Value>) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (key, item) in map {
                let key = key.replace('~', "~0").replace('/', "~1");
                leaves(item, &format!("{at}/{key}"), into);
            }
        }
        Value::Array(items) if !items.is_empty() => {
            for (index, item) in items.iter().enumerate() {
                leaves(item, &format!("{at}/{index}"), into);
            }
        }
        leaf => {
            into.insert(at.to_owned(), leaf.clone());
        }
    }
}

/// `document` read as `T`, or the pointers at which `T` written back differs from it — a field
/// `T` does not know among them — or why it does not read.
fn exactly<T: DeserializeOwned + Serialize>(document: &Value) -> Result<T, Vec<String>> {
    let typed: T =
        serde_json::from_value(document.clone()).map_err(|error| vec![error.to_string()])?;
    let back = serde_json::to_value(&typed).map_err(|error| vec![error.to_string()])?;
    let (mut left, mut right) = (BTreeMap::new(), BTreeMap::new());
    leaves(document, "", &mut left);
    leaves(&back, "", &mut right);
    let drift: Vec<String> = left
        .keys()
        .chain(right.keys())
        .filter(|at| left.get(*at) != right.get(*at))
        .cloned()
        .collect::<BTreeSet<String>>()
        .into_iter()
        .collect();
    if drift.is_empty() {
        Ok(typed)
    } else {
        Err(drift)
    }
}

/// [`exactly`], or a failure naming `what` and the drift.
fn exact<T: DeserializeOwned + Serialize>(what: &str, bytes: &[u8]) -> T {
    let document: Value = serde_json::from_slice(bytes).unwrap();
    exactly::<T>(&document).unwrap_or_else(|drift| {
        panic!(
            "{what}: not read exactly as {}: {drift:?}",
            std::any::type_name::<T>()
        )
    })
}

/// The one value a session and a one-shot read returned is exactly both documents `ekr`
/// answered for the same argv.
fn one_value<T>(what: &str, session: &T, one_shot: &T, session_raw: &Value, one_shot_raw: &Value)
where
    T: DeserializeOwned + Serialize + PartialEq + Debug,
{
    assert_eq!(
        session, one_shot,
        "{what}: a session and a one-shot read differ"
    );
    for (how, raw) in [("session", session_raw), ("one-shot", one_shot_raw)] {
        match exactly::<T>(raw) {
            Ok(read) => assert_eq!(&read, session, "{what}: the {how} document"),
            Err(drift) => panic!("{what}: the {how} document does not read exactly: {drift:?}"),
        }
    }
}

// ---- through a session and one-shot, on both providers ------------------------------------------

/// Acceptance 1 and 3: `quality`, `rejections` and `code-names` each return one typed value
/// through a session and as a one-shot `ekr` process, which written back is the verb's JSON, on
/// both providers; `code-names` reports the planted names with file and line and flags the
/// runtime word.
#[test]
fn each_check_read_is_the_verbs_document_through_a_session_and_one_shot_on_both_providers() {
    for backend in [Backend::File, Backend::Sqlite] {
        let world = World::planted(backend);
        let mut session = world.session();
        let mut one_shot = OneShotReader::new(&binary(), world.store(), world.options());
        let on = backend.as_str();

        for revision in [None, Some(0), Some(1)] {
            let what = format!("{on} quality {revision:?}");
            let typed: StoreQuality = Reader::new(&mut session).quality(revision).unwrap();
            let verb = argv("quality", &[("revision", revision)]);
            one_value(
                &what,
                &typed,
                &one_shot.quality(revision).unwrap(),
                &ok(&mut session, &verb),
                &world.one_shot(&verb),
            );
            let meta: &QualityMeta = &typed.meta;
            assert_eq!(meta.format, "ekr.store-quality/1", "{what}");
            assert_eq!(meta.revision, revision.unwrap_or(1), "{what}");
            let assertions: &AssertionQuality = &typed.assertions;
            assert!(assertions.active > 0, "{what}");
            assert_eq!(assertions.with_evidence_share, Some(10_000), "{what}");
            let properties: &PropertyQuality = &typed.properties;
            assert!(properties.declared > 0, "{what}");
            let shared: &[SharedName] = &typed.shared_names;
            let listed: BTreeSet<NodeId> =
                shared.iter().flat_map(|name| name.nodes.clone()).collect();
            assert_eq!(listed.len() as u64, typed.sharing_nodes, "{what}");
        }

        let (early, late): (TransactionId, TransactionId) =
            (EARLY.parse().unwrap(), LATE.parse().unwrap());
        for (from, to, expected) in [
            (None, None, vec![(early, 0, 2), (late, 1, 1)]),
            (Some(0), Some(0), vec![(early, 0, 2)]),
            (Some(1), None, vec![(late, 1, 1)]),
            (None, Some(0), vec![(early, 0, 2)]),
            (Some(1), Some(0), vec![]),
        ] {
            let what = format!("{on} rejections {from:?}..{to:?}");
            let typed: Rejections = Reader::new(&mut session).rejections(from, to).unwrap();
            let verb = argv("rejections", &[("from", from), ("to", to)]);
            one_value(
                &what,
                &typed,
                &one_shot.rejections(from, to).unwrap(),
                &ok(&mut session, &verb),
                &world.one_shot(&verb),
            );
            assert_eq!(typed.format, "ekr.rejections/1", "{what}");
            assert_eq!((typed.from, typed.to), (from, to), "{what}");
            let listed: Vec<(TransactionId, u64, usize)> = typed
                .rejections
                .iter()
                .map(|entry: &RejectedTransaction| {
                    (entry.transaction_id, entry.against, entry.issues.len())
                })
                .collect();
            assert_eq!(listed, expected, "{what}");
            for entry in &typed.rejections {
                assert_eq!(entry.proposer.to_string(), OPERATOR, "{what}");
                for issue in &entry.issues {
                    let issue: &RejectionIssue = issue;
                    assert_eq!(issue.transaction_id, entry.transaction_id, "{what}");
                    assert!(
                        !issue.validator.is_empty() && !issue.code.is_empty(),
                        "{what}"
                    );
                }
            }
        }

        let files = ["src/reader.ts", "lib/other.rs", "-dashed.ts"];
        for at in [None, Some(0)] {
            let what = format!("{on} code-names {at:?}");
            let typed: CodeNames = Reader::new(&mut session).code_names(files, at).unwrap();
            let mut verb = argv("code-names", &[("at", at)]);
            verb.push("--".to_owned());
            verb.extend(files.map(str::to_owned));
            one_value(
                &what,
                &typed,
                &one_shot.code_names(files, at).unwrap(),
                &ok(&mut session, &verb),
                &world.one_shot(&verb),
            );
            let meta: &CodeNamesMeta = &typed.meta;
            assert_eq!(meta.format, "ekr.code-names/1", "{what}");
            assert_eq!(meta.revision, at.unwrap_or(1), "{what}");
            assert_eq!(meta.files, 3, "{what}");
            assert_eq!(meta.findings, typed.findings.len() as u64, "{what}");
            let flagged = typed.findings.iter().filter(|f| f.runtime_word).count();
            assert_eq!(meta.runtime_word_findings, flagged as u64, "{what}");

            let found = |literal: &str| -> Option<&CodeNameFinding> {
                typed.findings.iter().find(|f| f.literal == literal)
            };
            let acme = found("Acme").unwrap_or_else(|| panic!("{what}: {typed:?}"));
            assert_eq!(
                (
                    acme.file.as_str(),
                    acme.line,
                    acme.column,
                    acme.runtime_word
                ),
                ("src/reader.ts", 2, 17, false),
                "{what}"
            );
            let named: &CodeNameMatch = &acme.names[0];
            assert_eq!(
                (
                    named.kind,
                    named.id.as_str(),
                    named.type_id.map(|id| id.to_string()),
                    named.type_name.as_deref()
                ),
                (
                    CodeNameKind::CanonicalName,
                    ACME,
                    Some(ORGANIZATION.to_owned()),
                    Some("Organization")
                ),
                "{what}"
            );
            let alice = found("Alice").unwrap_or_else(|| panic!("{what}: {typed:?}"));
            assert_eq!((alice.file.as_str(), alice.line), ("lib/other.rs", 1));
            let bob = found("Bob").unwrap_or_else(|| panic!("{what}: {typed:?}"));
            assert_eq!((bob.file.as_str(), bob.line), ("-dashed.ts", 1));
            let mut in_order: Vec<(&str, u64)> = typed
                .findings
                .iter()
                .map(|f| (f.file.as_str(), f.line))
                .collect();
            in_order.dedup_by_key(|(file, _)| *file);
            assert_eq!(
                in_order,
                [("-dashed.ts", 1), ("lib/other.rs", 1), ("src/reader.ts", 2)],
                "{what}: findings in file, then line order"
            );

            match at {
                None => {
                    let word = found(RUNTIME_WORD).unwrap_or_else(|| panic!("{what}: {typed:?}"));
                    assert_eq!(
                        (word.file.as_str(), word.line, word.runtime_word),
                        ("src/reader.ts", 3, true),
                        "{what}"
                    );
                    assert_eq!(word.names[0].id, PLANTED, "{what}");
                    assert_eq!(meta.runtime_word_findings, 1, "{what}");
                }
                Some(_) => assert!(
                    found(RUNTIME_WORD).is_none(),
                    "{what}: revision 0 holds no node named {RUNTIME_WORD}: {typed:?}"
                ),
            }
        }

        for refused in [
            Reader::new(&mut session).quality(Some(99)).map(drop),
            one_shot.quality(Some(99)).map(drop),
            Reader::new(&mut session)
                .code_names(["src/reader.ts"], Some(99))
                .map(drop),
            one_shot.code_names(["src/reader.ts"], Some(99)).map(drop),
        ] {
            assert!(
                matches!(&refused, Err(ReadError::Refused { refusal, .. })
                    if refusal.code == "ekr.views.RevisionNotFound"),
                "{on}: {refused:?}"
            );
        }
    }
}

// ---- every conformance fixture document ---------------------------------------------------------

const FIXTURES: [&str; 12] = [
    "seed-only",
    "seeded-evidence",
    "edge-assertion",
    "retracted-assertion",
    "schema-evolution",
    "store",
    "hub",
    "timeline",
    "growth",
    "subjects",
    "changes",
    "quality",
];

/// A source quoting every name `loaded` holds, one literal a line: every type's and property's
/// name, every node's canonical name and alias.
fn every_name(loaded: &ekr_views::LoadedRevision) -> SourceText {
    let ontology = loaded.graph.ontology.to_document();
    let mut names: Vec<String> = Vec::new();
    for declared in &ontology.node_types {
        names.push(declared.name.clone());
        names.extend(declared.properties.values().map(|p| p.name.clone()));
    }
    for declared in &ontology.edge_types {
        names.push(declared.name.clone());
        names.extend(declared.properties.values().map(|p| p.name.clone()));
    }
    for node in loaded.graph.nodes.values() {
        names.push(node.canonical_name.clone());
        names.extend(node.aliases.iter().cloned());
    }
    let text: String = names
        .iter()
        .filter(|name| !name.contains(['"', '\\', '\n']))
        .map(|name| format!("\"{name}\"\n"))
        .collect();
    SourceText {
        path: "names.txt".to_owned(),
        text,
    }
}

/// Acceptance 2, over the engine: every quality and code-names document each conformance
/// fixture store renders at every revision reads exactly into its typed value, and the documents
/// between them carry each optional field and leave it out, so a field either format gains is in
/// what this reads.
#[test]
fn every_conformance_fixture_quality_and_code_names_document_reads_exactly() {
    let mut seen: BTreeMap<&str, usize> = BTreeMap::new();
    for name in FIXTURES {
        let directory = tempfile::tempdir().unwrap();
        let runtime = fixtures::open(directory.path(), fixtures::Provider::File);
        fixtures::Fixture::named(name)
            .unwrap_or_else(|| panic!("no fixture {name}"))
            .build(&runtime);
        let head = runtime.head().unwrap().unwrap().revision.get();
        for revision in 0..=head {
            let at = format!("{name}@{revision}");
            let revision = Some(RevisionNumber::new(revision));

            let answer = ekr_views::report_quality(&runtime, revision).unwrap();
            let quality: StoreQuality = exact(&at, &answer.bytes);
            *seen.entry("quality").or_default() += 1;
            let assertions = &quality.assertions;
            for (field, present) in [
                (
                    "with_evidence_share",
                    assertions.with_evidence_share.is_some(),
                ),
                (
                    "with_item_evidence_share",
                    assertions.with_item_evidence_share.is_some(),
                ),
                (
                    "constrained_share",
                    quality.properties.constrained_share.is_some(),
                ),
            ] {
                let key = match (field, present) {
                    ("with_evidence_share", true) => "with_evidence_share present",
                    ("with_evidence_share", false) => "with_evidence_share absent",
                    ("with_item_evidence_share", true) => "with_item_evidence_share present",
                    ("with_item_evidence_share", false) => "with_item_evidence_share absent",
                    (_, true) => "constrained_share present",
                    (_, false) => "constrained_share absent",
                };
                *seen.entry(key).or_default() += 1;
            }
            if !quality.shared_names.is_empty() {
                *seen.entry("shared_names").or_default() += 1;
            }

            let loaded = ekr_views::load(&runtime, revision).unwrap();
            let sources = [every_name(&loaded)];
            let answer = ekr_views::find_code_names(&runtime, revision, &sources).unwrap();
            let names: CodeNames = exact(&at, &answer.bytes);
            *seen.entry("code-names").or_default() += 1;
            for finding in &names.findings {
                if finding.runtime_word {
                    *seen.entry("runtime_word").or_default() += 1;
                }
                for named in &finding.names {
                    let key = match (named.type_id.is_some(), named.type_name.is_some()) {
                        (true, true) => "type_id and type_name",
                        (false, false) => "no type_id",
                        _ => "type_id without type_name",
                    };
                    *seen.entry(key).or_default() += 1;
                }
            }
        }
    }
    for key in [
        "quality",
        "code-names",
        "with_evidence_share present",
        "with_evidence_share absent",
        "with_item_evidence_share present",
        "with_item_evidence_share absent",
        "constrained_share present",
        "constrained_share absent",
        "shared_names",
        "runtime_word",
        "type_id and type_name",
        "no type_id",
    ] {
        assert!(
            seen.get(key).copied().unwrap_or(0) > 0,
            "no fixture document carries {key}: {seen:?}"
        );
    }
}

// ---- a field a format gains ---------------------------------------------------------------------

/// The JSON pointer of every object in `value`, the document's own (`""`) first.
fn objects(value: &Value, at: &str, into: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            into.push(at.to_owned());
            for (key, item) in map {
                let key = key.replace('~', "~0").replace('/', "~1");
                objects(item, &format!("{at}/{key}"), into);
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                objects(item, &format!("{at}/{index}"), into);
            }
        }
        _ => {}
    }
}

/// With a field inserted into each object of `document` in turn: `T` still reads it — a newer
/// `ekr` does not break an older consumer — and the exact read fails on that field alone.
fn every_object_gains_a_field<T>(what: &str, document: &Value) -> usize
where
    T: DeserializeOwned + Serialize + Debug,
{
    assert!(
        exactly::<T>(document).is_ok(),
        "{what}: {:?}",
        exactly::<T>(document).err()
    );
    let mut pointers = Vec::new();
    objects(document, "", &mut pointers);
    for pointer in &pointers {
        let mut grown = document.clone();
        grown
            .pointer_mut(pointer)
            .and_then(Value::as_object_mut)
            .unwrap()
            .insert("added_by_a_newer_ekr".to_owned(), Value::from(1));
        if let Err(error) = serde_json::from_value::<T>(grown.clone()) {
            panic!("{what}: a field at {pointer:?} breaks the read: {error}");
        }
        assert_eq!(
            exactly::<T>(&grown).map(drop),
            Err(vec![format!("{pointer}/added_by_a_newer_ekr")]),
            "{what}"
        );
    }
    pointers.len()
}

/// Acceptance 2: a field added to any object of `ekr.store-quality/1`, `ekr.code-names/1` or
/// `ekr.rejections/1` is ignored by the typed read and fails the exact read by its pointer.
#[test]
fn a_field_added_to_a_check_format_is_ignored_on_read_and_fails_the_exact_read() {
    let directory = tempfile::tempdir().unwrap();
    let runtime = fixtures::open(directory.path(), fixtures::Provider::File);
    fixtures::Fixture::named("quality").unwrap().build(&runtime);
    let quality: Value =
        serde_json::from_slice(&ekr_views::report_quality(&runtime, None).unwrap().bytes).unwrap();
    let loaded = ekr_views::load(&runtime, None).unwrap();
    let names: Value = serde_json::from_slice(
        &ekr_views::find_code_names(&runtime, None, &[every_name(&loaded)])
            .unwrap()
            .bytes,
    )
    .unwrap();

    let world = World::planted(Backend::File);
    let rejections: Value =
        world.one_shot(&argv("rejections", &[("from", Some(0)), ("to", Some(1))]));

    // Each document reaches every object its format declares: the shared names, findings with
    // their matches, rejections with their issues.
    assert!(quality["shared_names"][0].is_object(), "{quality}");
    assert!(names["findings"][0]["names"][0].is_object(), "{names}");
    assert!(
        rejections["rejections"][0]["issues"][0].is_object(),
        "{rejections}"
    );

    let grown = [
        every_object_gains_a_field::<StoreQuality>("quality", &quality),
        every_object_gains_a_field::<CodeNames>("code-names", &names),
        every_object_gains_a_field::<Rejections>("rejections", &rejections),
    ];
    assert!(grown.iter().all(|objects| *objects >= 4), "{grown:?}");

    // A kind of store name the format gains reads as `Other` and fails the exact read by its
    // pointer.
    let mut new_kind = names.clone();
    new_kind["findings"][0]["names"][0]["kind"] = Value::from("OperationName");
    let read: CodeNames = serde_json::from_value(new_kind.clone()).unwrap();
    assert_eq!(read.findings[0].names[0].kind, CodeNameKind::Other);
    assert_eq!(
        exactly::<CodeNames>(&new_kind).map(drop),
        Err(vec!["/findings/0/names/0/kind".to_owned()])
    );
}
