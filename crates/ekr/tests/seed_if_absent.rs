//! `ekr seed --if-absent` (`story:seed-if-absent`): a seed only if the store holds none. Any seed
//! already there — the identical document included — refuses it as `ekr.kernel.AlreadySeeded`
//! (exit 2) and writes nothing; without the flag an identical re-seed still exits 0 with the first
//! seed's result. The PostgreSQL case, two concurrent processes on one tenant, is
//! `postgres_cli.rs`.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{json, Value};

const BACKENDS: [&str; 2] = ["file", "sqlite"];

fn ekr() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for var in ["EKR_HOST", "EKR_STORE", "EKR_BACKEND", "EKR_FULL_REPLAY"] {
        command.env_remove(var);
    }
    command
}

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

struct Store {
    directory: tempfile::TempDir,
    host: PathBuf,
    store: PathBuf,
    backend: &'static str,
}

impl Store {
    fn new(backend: &'static str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let host = directory.path().join("host.json");
        std::fs::write(&host, text(&["example", "ekr.cli-host/1"])).unwrap();
        std::fs::write(
            directory.path().join("seed.yaml"),
            text(&["example", "ekr-seed/2"]),
        )
        .unwrap();
        let store = match backend {
            "file" => directory.path().join("store"),
            _ => directory.path().join("state.db"),
        };
        Self {
            directory,
            host,
            store,
            backend,
        }
    }

    fn seed_path(&self) -> PathBuf {
        self.directory.path().join("seed.yaml")
    }

    fn command(&self) -> Command {
        let mut command = ekr();
        command
            .arg("--host")
            .arg(&self.host)
            .arg("--store")
            .arg(&self.store)
            .args(["--backend", self.backend]);
        command
    }

    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }

    fn seed(&self, flags: &[&str]) -> Output {
        let seed = self.seed_path();
        let mut args = vec!["seed"];
        args.extend_from_slice(flags);
        args.push(seed.to_str().unwrap());
        self.run(&args)
    }
}

fn document(output: &Output) -> Value {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn refused_already_seeded(output: &Output, what: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{what}: {stderr}");
    assert!(
        stderr.contains("ekr.kernel.AlreadySeeded"),
        "{what}: {stderr}"
    );
    assert!(output.stdout.is_empty(), "{what}: printed a result");
}

fn tree(at: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    fn walk(at: &Path, into: &mut Vec<(PathBuf, Vec<u8>)>) {
        let mut entries: Vec<_> = std::fs::read_dir(at)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, into);
            } else {
                let bytes = std::fs::read(&path).unwrap();
                into.push((path, bytes));
            }
        }
    }
    let mut all = Vec::new();
    walk(at, &mut all);
    all
}

#[test]
fn a_second_if_absent_seed_of_the_identical_document_is_refused_as_already_seeded() {
    for backend in BACKENDS {
        let store = Store::new(backend);
        let first = document(&store.seed(&["--if-absent"]));
        assert_eq!(first["result"]["revision"], 0, "{backend}");
        let head = document(&store.run(&["head"]));
        // The file provider's directory is compared byte for byte; the SQLite file is not, since
        // opening it may touch pages without writing a record. `head` stands for both.
        let before = (backend == "file").then(|| tree(&store.store));
        refused_already_seeded(
            &store.seed(&["--if-absent"]),
            &format!("{backend}: second identical --if-absent seed"),
        );
        if let Some(before) = before {
            assert_eq!(tree(&store.store), before, "{backend}: the refusal wrote");
        }
        assert_eq!(document(&store.run(&["head"])), head, "{backend}");
        // Without the flag, the identical re-seed answers exactly as before this story.
        assert_eq!(document(&store.seed(&[])), first, "{backend}: plain retry");
    }
}

#[test]
fn an_if_absent_seed_on_a_store_seeded_without_the_flag_is_refused() {
    for backend in BACKENDS {
        let store = Store::new(backend);
        let first = document(&store.seed(&[]));
        refused_already_seeded(
            &store.seed(&["--if-absent"]),
            &format!("{backend}: --if-absent after a plain seed"),
        );
        assert_eq!(document(&store.seed(&[])), first, "{backend}");
    }
}

/// `ekr session --create` serves `seed` with the arguments `ekr seed` takes, and the SDK drives
/// the binary through it: `--if-absent` answers there exactly as the one-shot verb does.
#[test]
fn a_create_session_serves_seed_if_absent_as_the_one_shot_verb_does() {
    for backend in BACKENDS {
        let store = Store::new(backend);
        let seed = store.seed_path();
        let request = json!({"argv": ["seed", "--if-absent", seed.to_str().unwrap()]});
        let mut session = store
            .command()
            .args(["session", "--create"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        {
            let mut input = session.stdin.take().unwrap();
            writeln!(input, "{request}").unwrap();
            writeln!(input, "{request}").unwrap();
        }
        let output = session.wait_with_output().unwrap();
        assert_eq!(
            output.status.code(),
            Some(0),
            "{backend}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let answers: Vec<Value> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert_eq!(answers.len(), 2, "{backend}: {answers:?}");
        assert_eq!(answers[0]["exit"], 0, "{backend}: {answers:?}");
        assert_eq!(
            answers[0]["stdout"]["result"]["revision"], 0,
            "{backend}: {answers:?}"
        );
        assert_eq!(answers[1]["exit"], 2, "{backend}: {answers:?}");
        assert_eq!(answers[1]["stdout"], Value::Null, "{backend}: {answers:?}");
        assert!(
            answers[1]["stderr"]
                .as_str()
                .unwrap()
                .contains("ekr.kernel.AlreadySeeded"),
            "{backend}: {answers:?}"
        );
        assert_eq!(
            document(&store.seed(&[])),
            answers[0]["stdout"],
            "{backend}: the session's seed is the lineage"
        );
    }
}

/// Adversary (story:seed-if-absent, pass 1): `docs/cli.md` says two callers seeding one store at
/// once, "on any provider", cannot both exit 0 and "the other caller is refused", and the
/// CHANGELOG says exactly one exits 0. Here the callers are separate `ekr seed --if-absent`
/// processes on a store path that holds no store yet — the way a host creates a store — so the
/// race includes creating the store. Every loser must be the named refusal: exit 2,
/// `ekr.kernel.AlreadySeeded`.
#[test]
fn adversary_concurrent_if_absent_processes_on_a_fresh_store_refuse_every_loser_as_already_seeded()
{
    const CALLERS: usize = 6;
    const ROUNDS: usize = 5;
    for backend in BACKENDS {
        for round in 0..ROUNDS {
            let store = Store::new(backend);
            let seed = store.seed_path();
            let callers: Vec<_> = (0..CALLERS)
                .map(|_| {
                    store
                        .command()
                        .args(["seed", "--if-absent", seed.to_str().unwrap()])
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .spawn()
                        .unwrap()
                })
                .collect();
            let outputs: Vec<Output> = callers
                .into_iter()
                .map(|caller| caller.wait_with_output().unwrap())
                .collect();
            let described: Vec<_> = outputs
                .iter()
                .map(|output| {
                    (
                        output.status.code(),
                        String::from_utf8_lossy(&output.stderr).into_owned(),
                    )
                })
                .collect();
            let created = outputs.iter().filter(|o| o.status.success()).count();
            assert_eq!(created, 1, "{backend} round {round}: {described:#?}");
            for output in outputs.iter().filter(|o| !o.status.success()) {
                refused_already_seeded(output, &format!("{backend} round {round}"));
            }
        }
    }
}

/// Adversary (story:seed-if-absent, pass 1): a plain `ekr seed` racing an `ekr seed --if-absent`
/// of the identical document on a fresh store. The plain seed keeps its contract (exit 0, the
/// lineage's result); the if-absent caller either created that same lineage or is refused as
/// `AlreadySeeded`.
#[test]
fn adversary_a_plain_seed_racing_an_if_absent_seed_keeps_both_contracts() {
    const ROUNDS: usize = 8;
    for backend in BACKENDS {
        for round in 0..ROUNDS {
            let store = Store::new(backend);
            let seed = store.seed_path();
            let spawn = |flags: &[&str]| {
                let mut command = store.command();
                command.arg("seed").args(flags).arg(&seed);
                command
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .unwrap()
            };
            let (conditional, plain) = if round % 2 == 0 {
                let c = spawn(&["--if-absent"]);
                (c, spawn(&[]))
            } else {
                let p = spawn(&[]);
                (spawn(&["--if-absent"]), p)
            };
            let conditional = conditional.wait_with_output().unwrap();
            let plain = plain.wait_with_output().unwrap();
            let what = format!(
                "{backend} round {round}: conditional {:?} {} / plain {:?} {}",
                conditional.status.code(),
                String::from_utf8_lossy(&conditional.stderr),
                plain.status.code(),
                String::from_utf8_lossy(&plain.stderr)
            );
            assert_eq!(plain.status.code(), Some(0), "{what}");
            let lineage = document(&plain);
            if conditional.status.success() {
                assert_eq!(document(&conditional), lineage, "{what}");
            } else {
                refused_already_seeded(&conditional, &what);
            }
            assert_eq!(
                document(&store.run(&["head"]))["root"],
                lineage["result"],
                "{what}"
            );
        }
    }
}
