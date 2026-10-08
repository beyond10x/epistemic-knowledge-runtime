//! Adversary pass on `task:sdk-types-the-check-reads`: the SDK's typed `quality`, `rejections`
//! and `code-names` reads against `docs/cli.md` § `ekr quality`, `ekr rejections`,
//! `ekr code-names` and `docs/sdk.md` § Typed reads.
//!
//! * The argv each call sends is the one the verb documents, at the bounds: no revision, revision
//!   `u64::MAX`, an empty file list, paths that start with `-`, hold a space or non-ASCII text, or
//!   repeat.
//! * Those paths and bounds reach real `ekr` through a session and one-shot, and read as the CLI
//!   printed them; the failures are the `ReadError` variants `docs/sdk.md` names.
//! * A store whose schema declares no property prints `properties` without `constrained_share`;
//!   the unit reads that only from the `hub` fixture at revision 0, never from real `ekr`.
//! * The typed value, written back through `serde_json::Value`, is the CLI's bytes, keys sorted.
//! * `checks.rs` promises "a newer `ekr` does not break an older consumer"; a `CodeNameKind` a
//!   newer `ekr` adds breaks the whole read.
//!
//! Every case that runs `ekr` drives the binary built from this checkout, by path.

#![cfg(unix)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use ekr_sdk::binary::EkrBinary;
use ekr_sdk::read::{CodeNames, OneShotReader, ReadError, Reader, StoreQuality};
use ekr_sdk::reply::Reply;
use ekr_sdk::session::{Backend, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{Request, Transport, TransportError};
use serde_json::Value;

const OPERATOR: &str = "00000000-0000-4000-8000-000000000101";
/// Rejected against revision 0: an alias for a node no revision holds.
const EARLY: &str = "00000000-0000-4000-8000-00000000f101";
/// A path with a space and non-ASCII text in both its directory and its name.
const UNICODE_PATH: &str = "a dir/ü名 file.ts";
/// `Acme` quoted after two non-ASCII characters: the quote is character 12, byte 15.
const UNICODE_TS: &str = "const ü名 = \"Acme\";\n";

// ---- the binary and a store under the example host ---------------------------------------------

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

/// One provider in its own directory under the example host.
struct World {
    directory: tempfile::TempDir,
    backend: Backend,
}

impl World {
    /// The host only: no store at `--store`.
    fn empty(backend: Backend) -> Self {
        let world = Self {
            directory: tempfile::tempdir().unwrap(),
            backend,
        };
        let host = world.stdout(&["example", "ekr.cli-host/1"], false);
        world.file("host.json", &host);
        world
    }

    /// The example seed, `EARLY` rejected against revision 0, and the consumer sources.
    fn seeded(backend: Backend) -> Self {
        let world = Self::empty(backend);
        let seed = world.stdout(&["example", "ekr-seed/2"], false);
        world.file("seed.yaml", &seed);
        world.stdout(&["seed", "seed.yaml"], true);
        world.propose(
            EARLY,
            "  - !AddAlias\n    node: 00000000-0000-4000-8000-00000000f398\n    alias: \
             first-unknown\n",
        );
        let validated = world.stdout(&["validate", EARLY, "--against", "0"], true);
        assert!(validated.contains("Rejected"), "{validated}");
        world.file(UNICODE_PATH, UNICODE_TS);
        world.file("lib/other.rs", "let person = \"Alice\";\n");
        world
    }

    /// The example seed with the one property it declares removed: `properties.declared` is 0.
    fn without_properties(backend: Backend) -> Self {
        let world = Self::empty(backend);
        let seed = world.stdout(&["example", "ekr-seed/2"], false);
        let start = seed
            .find("    properties:\n      ")
            .expect("the example seed declares a property");
        let end = start
            + seed[start..]
                .find("    abstract_type:")
                .expect("the declaring type goes on");
        let seed = format!("{}    properties: {{}}\n{}", &seed[..start], &seed[end..]);
        world.file("seed.yaml", &seed);
        world.stdout(&["seed", "seed.yaml"], true);
        world
    }

    fn file(&self, name: &str, contents: &str) {
        let path = self.directory.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn store(&self) -> StoreConfig {
        let store = match self.backend {
            Backend::File => "store",
            Backend::Sqlite => "state.db",
            Backend::Postgres => panic!("local fixture requires a filesystem backend"),
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

    fn one_shot(&self) -> OneShotReader {
        OneShotReader::new(&binary(), self.store(), self.options())
    }

    /// `ekr [store options] <args>` in this world's directory, exit 0: its stdout, as text.
    fn stdout(&self, args: &[&str], on_store: bool) -> String {
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
        String::from_utf8(output.stdout).unwrap()
    }

    fn propose(&self, id: &str, operations: &str) {
        let document = format!(
            "format: ekr.transaction-document/2\ntransaction:\n  id: {id}\n  proposer: \
             {OPERATOR}\n  operations:\n{operations}  evidence: []\n"
        );
        let file = format!("{id}.yaml");
        self.file(&file, &document);
        self.stdout(&["propose", &file], true);
    }
}

/// `value` written back through `serde_json::Value`, as `ekr` prints a document.
fn printed<T: serde::Serialize>(value: &T) -> String {
    let mut text = serde_json::to_string_pretty(&serde_json::to_value(value).unwrap()).unwrap();
    text.push('\n');
    text
}

fn is_revision_not_found<V: std::fmt::Debug>(result: &Result<V, ReadError>) -> bool {
    matches!(result, Err(ReadError::Refused { refusal, .. })
        if refusal.code == "ekr.views.RevisionNotFound")
}

// ---- the argv, without a process -----------------------------------------------------------------

/// Records every argv and answers each with a refusal, so no value is read.
#[derive(Default)]
struct Recording(Vec<Vec<String>>);

impl Transport for Recording {
    fn request(&mut self, request: &Request) -> Result<Reply, TransportError> {
        self.0.push(request.argv.clone());
        Ok(Reply {
            exit: 2,
            document: None,
            stderr: "ekr: adversary.Recorded: nothing to read\n".to_owned(),
        })
    }
}

/// `docs/cli.md`: `ekr quality [--revision N]`, `ekr rejections [--from N] [--to M]`,
/// `ekr code-names <file>... [--at N]`. Each call sends that argv and nothing else, at the
/// bounds, with every path after `--` exactly as given, in the order given, repeats kept.
#[test]
fn each_call_sends_exactly_the_argv_its_verb_documents() {
    let mut recording = Recording::default();
    {
        let mut reader = Reader::new(&mut recording);
        let _ = reader.quality(None);
        let _ = reader.quality(Some(0));
        let _ = reader.quality(Some(u64::MAX));
        let _ = reader.rejections(None, None);
        let _ = reader.rejections(Some(0), None);
        let _ = reader.rejections(None, Some(u64::MAX));
        let _ = reader.rejections(Some(3), Some(2));
        let _ = reader.code_names(Vec::<String>::new(), None);
        let _ = reader.code_names(
            ["-x", "--at", UNICODE_PATH, UNICODE_PATH, "-", "--"],
            Some(7),
        );
        let _ = reader.code_names(vec![String::from("src/a.ts")], Some(u64::MAX));
    }
    let max = u64::MAX.to_string();
    let expected: Vec<Vec<&str>> = vec![
        vec!["quality"],
        vec!["quality", "--revision", "0"],
        vec!["quality", "--revision", &max],
        vec!["rejections"],
        vec!["rejections", "--from", "0"],
        vec!["rejections", "--to", &max],
        vec!["rejections", "--from", "3", "--to", "2"],
        vec!["code-names", "--"],
        vec![
            "code-names",
            "--at",
            "7",
            "--",
            "-x",
            "--at",
            UNICODE_PATH,
            UNICODE_PATH,
            "-",
            "--",
        ],
        vec!["code-names", "--at", &max, "--", "src/a.ts"],
    ];
    assert_eq!(recording.0, expected);
}

// ---- the argv, through real ekr ------------------------------------------------------------------

/// `docs/sdk.md`: "no file is a `ReadError::Usage`", through a session and one-shot.
#[test]
fn an_empty_file_list_is_a_usage_error_through_a_session_and_one_shot() {
    let world = World::seeded(Backend::File);
    let mut session = world.session();
    let mut one_shot = world.one_shot();
    for (how, result) in [
        (
            "session",
            Reader::new(&mut session).code_names(Vec::<String>::new(), None),
        ),
        ("one-shot", one_shot.code_names(Vec::<String>::new(), None)),
    ] {
        match result {
            Err(ReadError::Usage { verb, message }) => {
                assert_eq!(verb, "code-names", "{how}");
                assert!(message.contains("<FILE>"), "{how}: {message}");
            }
            other => panic!("{how}: {other:?}"),
        }
    }
}

/// `docs/cli.md`: a file that does not read or is not UTF-8 is a fault naming it, exit 1; so is
/// a directory.
/// `docs/sdk.md`: a `ReadError::Fault`, through a session and one-shot, on both providers.
#[test]
fn a_directory_or_a_missing_file_is_a_fault_naming_it_through_a_session_and_one_shot() {
    for backend in [Backend::File, Backend::Sqlite] {
        let world = World::seeded(backend);
        let mut session = world.session();
        let mut one_shot = world.one_shot();
        std::fs::write(world.directory.path().join("latin1.ts"), b"\"caf\xe9\"\n").unwrap();
        for path in ["a dir", "absent ü.ts", "latin1.ts"] {
            for (how, result) in [
                (
                    "session",
                    Reader::new(&mut session).code_names([path], None),
                ),
                ("one-shot", one_shot.code_names([path], None)),
            ] {
                match result {
                    Err(ReadError::Fault { verb, fault }) => {
                        assert_eq!((verb.as_str(), fault.exit), ("code-names", 1), "{how}");
                        assert!(fault.message.contains(path), "{how}: {}", fault.message);
                    }
                    other => panic!("{} {how} {path:?}: {other:?}", backend.as_str()),
                }
            }
        }
    }
}

/// `docs/cli.md`: "no store at `--store`" is a fault, exit 1, for all three verbs; the SDK
/// types it `ReadError::Fault`, through a session and one-shot alike.
#[test]
fn a_store_that_does_not_exist_is_a_fault_through_a_session_and_one_shot() {
    for backend in [Backend::File, Backend::Sqlite] {
        let world = World::empty(backend);
        world.file("lib/other.rs", "let person = \"Alice\";\n");
        let mut session = world.session();
        let mut one_shot = world.one_shot();
        let results = [
            Reader::new(&mut session).quality(None).map(drop),
            one_shot.quality(None).map(drop),
            Reader::new(&mut session).rejections(None, None).map(drop),
            one_shot.rejections(None, None).map(drop),
            Reader::new(&mut session)
                .code_names(["lib/other.rs"], None)
                .map(drop),
            one_shot.code_names(["lib/other.rs"], None).map(drop),
        ];
        for result in results {
            assert!(
                matches!(&result, Err(ReadError::Fault { fault, .. })
                    if fault.exit == 1 && fault.message.contains("store-not-found")),
                "{}: {result:?}",
                backend.as_str()
            );
        }
    }
}

/// Paths with a space, non-ASCII text and a repeat reach `ekr` unchanged through a session and
/// one-shot: one value, the document the CLI prints for that argv. `docs/cli.md`: each file is
/// read once, `file` is the path as given, `column` counts characters.
#[test]
fn spaced_unicode_and_repeated_paths_read_as_the_cli_prints_them() {
    for backend in [Backend::File, Backend::Sqlite] {
        let world = World::seeded(backend);
        let mut session = world.session();
        let mut one_shot = world.one_shot();
        let files = [UNICODE_PATH, "lib/other.rs", UNICODE_PATH];
        let on = backend.as_str();

        let typed: CodeNames = Reader::new(&mut session).code_names(files, None).unwrap();
        assert_eq!(typed, one_shot.code_names(files, None).unwrap(), "{on}");
        let mut argv = vec!["code-names", "--"];
        argv.extend(files);
        let cli = world.stdout(&argv, true);
        assert_eq!(
            printed(&typed),
            cli,
            "{on}: the typed value is not the CLI's bytes"
        );

        assert_eq!(typed.meta.files, 2, "{on}: {typed:?}");
        let acme: Vec<_> = typed
            .findings
            .iter()
            .filter(|finding| finding.literal == "Acme")
            .collect();
        assert_eq!(acme.len(), 1, "{on}: {typed:?}");
        assert_eq!(
            (acme[0].file.as_str(), acme[0].line, acme[0].column),
            (UNICODE_PATH, 1, 12),
            "{on}"
        );
    }
}

/// Revision `u64::MAX` is a revision the store does not hold: `quality` and `code-names` are
/// refused as `ekr.views.RevisionNotFound`, and `rejections` selects by it, echoing it, through
/// a session and one-shot on both providers.
#[test]
fn revision_u64_max_is_refused_or_selected_not_a_fault() {
    for backend in [Backend::File, Backend::Sqlite] {
        let world = World::seeded(backend);
        let mut session = world.session();
        let mut one_shot = world.one_shot();
        let on = backend.as_str();

        let quality = [
            Reader::new(&mut session).quality(Some(u64::MAX)),
            one_shot.quality(Some(u64::MAX)),
        ];
        for result in &quality {
            assert!(is_revision_not_found(result), "{on}: {result:?}");
        }
        let names = [
            Reader::new(&mut session).code_names(["lib/other.rs"], Some(u64::MAX)),
            one_shot.code_names(["lib/other.rs"], Some(u64::MAX)),
        ];
        for result in &names {
            assert!(is_revision_not_found(result), "{on}: {result:?}");
        }

        for (from, to, listed) in [
            (Some(u64::MAX), None, 0),
            (Some(u64::MAX), Some(u64::MAX), 0),
            (Some(0), Some(u64::MAX), 1),
        ] {
            let typed = Reader::new(&mut session).rejections(from, to).unwrap();
            assert_eq!(
                typed,
                one_shot.rejections(from, to).unwrap(),
                "{on} {from:?}..{to:?}"
            );
            assert_eq!((typed.from, typed.to), (from, to), "{on}");
            assert_eq!(typed.rejections.len(), listed, "{on} {from:?}..{to:?}");
        }
    }
}

// ---- a share left out, from real ekr -------------------------------------------------------------

/// `docs/cli.md`: a `_share` "is left out when the whole is `0`". A store whose schema declares
/// no property prints `properties` as `{"constrained": 0, "constrained_types": 0, "declared": 0}`;
/// the typed value reads
/// it with `constrained_share` `None` and writes back exactly the CLI's bytes, through a session
/// and one-shot on both providers. The unit reads this shape from the `hub` fixture at revision 0
/// only, through the engine; this reads it from real `ekr`.
#[test]
fn a_store_declaring_no_property_reads_without_constrained_share() {
    for backend in [Backend::File, Backend::Sqlite] {
        let world = World::without_properties(backend);
        let mut session = world.session();
        let mut one_shot = world.one_shot();
        let on = backend.as_str();

        let typed: StoreQuality = Reader::new(&mut session).quality(None).unwrap();
        assert_eq!(typed, one_shot.quality(None).unwrap(), "{on}");
        let cli = world.stdout(&["quality"], true);
        let document: Value = serde_json::from_str(&cli).unwrap();
        assert_eq!(
            document["properties"],
            serde_json::json!({"constrained": 0, "constrained_types": 0, "declared": 0}),
            "{on}: {cli}"
        );
        assert_eq!(
            (
                typed.properties.declared,
                typed.properties.constrained_types,
                typed.properties.constrained_share
            ),
            (0, 0, None),
            "{on}"
        );
        assert_eq!(
            printed(&typed),
            cli,
            "{on}: the typed value is not the CLI's bytes"
        );
    }
}

/// The CLI prints keys sorted; the typed values of all three verbs, written back through
/// `serde_json::Value`, are its bytes.
#[test]
fn each_typed_value_written_back_is_the_clis_bytes() {
    let world = World::seeded(Backend::File);
    let mut one_shot = world.one_shot();
    assert_eq!(
        printed(&one_shot.quality(Some(0)).unwrap()),
        world.stdout(&["quality", "--revision", "0"], true)
    );
    assert_eq!(
        printed(&one_shot.rejections(Some(0), None).unwrap()),
        world.stdout(&["rejections", "--from", "0"], true)
    );
    assert_eq!(
        printed(&one_shot.code_names(["lib/other.rs"], Some(0)).unwrap()),
        world.stdout(&["code-names", "--at", "0", "--", "lib/other.rs"], true)
    );
}

// ---- a newer ekr ---------------------------------------------------------------------------------

/// Answers one document to every request.
struct Canned(Value);

impl Transport for Canned {
    fn request(&mut self, _: &Request) -> Result<Reply, TransportError> {
        Ok(Reply {
            exit: 0,
            document: Some(self.0.clone()),
            stderr: String::new(),
        })
    }
}

/// `crates/ekr-sdk/src/read/checks.rs` module doc: "A reader ignores a field it does not know,
/// so a newer `ekr` does not break an older consumer." `CodeNameKind` is closed: a newer `ekr`
/// that names one more kind of store name breaks the whole `code-names` read, every finding
/// with it, as `ReadError::Document`.
#[test]
fn a_code_name_kind_a_newer_ekr_adds_does_not_break_the_read() {
    let world = World::seeded(Backend::File);
    let mut document: Value =
        serde_json::from_str(&world.stdout(&["code-names", "--", "lib/other.rs"], true)).unwrap();
    assert!(
        document["findings"][0]["names"][0]["kind"].is_string(),
        "{document}"
    );
    document["findings"][0]["names"][0]["kind"] = Value::from("OperationName");

    let read = Reader::new(Canned(document)).code_names(["lib/other.rs"], None);
    match read {
        Ok(names) => assert_eq!(names.findings.len(), 1, "{names:?}"),
        Err(error) => panic!("a newer ekr breaks the read: {error}"),
    }
}
