//! Design § 65, the retraction example, driven through fresh `ekr` binary processes on both
//! providers against the real kernel authority (`story:ekr-cli`).
//!
//! Every command below is a new process: nothing survives between two of them except what the
//! provider retained. Observations of retained state that the P1 command surface does not yet
//! expose (`transactions`, the verified graph) are read through the public kernel `Runtime`
//! opened under the same trusted host, never through a raw store.

use std::path::{Path, PathBuf};
use std::process::Output;

use ekr::host::CliHostConfigurationV1;
use ekr_core::{AssertionId, Timestamp, TransactionId};
use ekr_graph::GraphSnapshot;
use ekr_kernel::{Runtime, TransactionState};
use serde_json::Value;

/// 2026-03-12T00:00:00Z, the day Bob became chief executive in the example.
const MARCH_12: i64 = 1_773_273_600_000;
const ALICE: &str = "00000000-0000-4000-8000-000000000501";
const BOB: &str = "00000000-0000-4000-8000-000000000502";
const T_ALICE: &str = "00000000-0000-4000-8000-000000000601";
const T_BOB: &str = "00000000-0000-4000-8000-000000000602";
const T_REJECTED: &str = "00000000-0000-4000-8000-000000000603";
const T_STALE: &str = "00000000-0000-4000-8000-000000000604";
const T_FOREIGN: &str = "00000000-0000-4000-8000-000000000605";
const T_UNKNOWN: &str = "00000000-0000-4000-8000-000000000699";
const BACKENDS: [&str; 2] = ["file", "sqlite"];

fn fixture(name: &str) -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("cargo sets CARGO_MANIFEST_DIR for a test process at run time");
    Path::new(&manifest)
        .join("tests/fixtures/retraction")
        .join(name)
}

/// One provider in its own directory, driven only through fresh binary processes.
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

    fn store(&self) -> PathBuf {
        match self.backend {
            "file" => self.directory.path().to_path_buf(),
            _ => self.directory.path().join("state.db"),
        }
    }

    fn args(&self, verb: &[&str]) -> Vec<String> {
        let mut args = vec![
            "--host".to_owned(),
            fixture("host.json").display().to_string(),
            "--store".to_owned(),
            self.store().display().to_string(),
            "--backend".to_owned(),
            self.backend.to_owned(),
        ];
        args.extend(verb.iter().map(|arg| (*arg).to_owned()));
        args
    }

    fn run_with_stdin(&self, verb: &[&str], stdin: &[u8]) -> Output {
        use std::io::Write;
        let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(self.args(verb))
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(stdin).unwrap();
        child.wait_with_output().unwrap()
    }

    fn run(&self, verb: &[&str]) -> Output {
        self.run_with_stdin(verb, b"")
    }

    /// A declared outcome: exit 0 and the actual result as JSON on stdout.
    fn ok(&self, verb: &[&str]) -> Value {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(0),
            "{} {verb:?}: stderr {}",
            self.backend,
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
            panic!(
                "{verb:?} stdout is not one JSON result ({error}): {}",
                String::from_utf8_lossy(&output.stdout)
            )
        })
    }

    /// A named ESS refusal: exit 2, the refusal's name on stderr, nothing on stdout.
    fn refused(&self, verb: &[&str], name: &str) -> Output {
        let output = self.run(verb);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{} {verb:?}: stdout {} stderr {}",
            self.backend,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(name),
            "{verb:?}: expected {name} in {stderr}"
        );
        assert!(output.stdout.is_empty(), "{verb:?} wrote a result");
        output
    }

    fn fixture_arg(name: &str) -> String {
        fixture(name).display().to_string()
    }

    /// The retained state, observed through the public kernel facade under the same host.
    fn runtime(&self) -> Runtime {
        let host = CliHostConfigurationV1::from_json(&std::fs::read(fixture("host.json")).unwrap())
            .unwrap();
        match self.backend {
            "file" => Runtime::file(&self.store(), &host.tenant, host.context, host.authority),
            _ => Runtime::sqlite(&self.store(), &host.tenant, host.context, host.authority),
        }
        .unwrap()
    }

    /// Every retained transaction and the verified head, which a refusal must leave unchanged.
    fn retained(&self) -> (Vec<(TransactionId, TransactionState)>, Value) {
        let runtime = self.runtime();
        let transactions = runtime
            .transactions()
            .unwrap()
            .into_iter()
            .map(|(id, record)| (id, record.state()))
            .collect();
        let head = serde_json::to_value(runtime.head().unwrap()).unwrap();
        (transactions, head)
    }
}

fn seed(world: &World) -> Value {
    world.ok(&["seed", &World::fixture_arg("seed.yaml")])
}

fn propose(world: &World, name: &str) -> Value {
    world.ok(&["propose", &World::fixture_arg(name)])
}

fn validate(world: &World, transaction: &str, against: u64) -> Value {
    world.ok(&["validate", transaction, "--against", &against.to_string()])
}

fn commit(world: &World, transaction: &str) -> Value {
    world.ok(&["commit", transaction])
}

fn valid_at(world: &World, at: i64) -> Vec<String> {
    let graph = world.runtime().snapshot().unwrap();
    GraphSnapshot::of(&graph)
        .valid_at(Timestamp::from_millis(at))
        .iter()
        .map(|assertion| assertion.id.to_string())
        .collect()
}

#[test]
fn the_retraction_example_runs_through_fresh_processes_on_both_providers() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let seeded = seed(&world);
        assert_eq!(seeded["result"]["revision"], 0, "{seeded}");

        let proposed = propose(&world, "propose-alice.yaml");
        assert_eq!(proposed["transaction_id"], T_ALICE, "{proposed}");
        assert_eq!(
            proposed["document_bytes"],
            serde_json::to_value(std::fs::read(fixture("propose-alice.yaml")).unwrap()).unwrap(),
            "the exact submitted bytes are retained"
        );
        let validated = validate(&world, T_ALICE, 0);
        assert_eq!(validated["kind"], "Validated", "{validated}");
        let first = commit(&world, T_ALICE);
        assert_eq!(first["kind"], "Committed", "{first}");
        assert_eq!(first["result"]["revision"], 1);
        assert_ne!(
            first["result"]["knowledge_root"],
            seeded["result"]["knowledge_root"]
        );

        propose(&world, "propose-bob.yaml");
        assert_eq!(validate(&world, T_BOB, 1)["kind"], "Validated");
        let second = commit(&world, T_BOB);
        assert_eq!(second["kind"], "Committed", "{second}");
        assert_eq!(second["result"]["revision"], 2);
        assert_eq!(second["result"]["parent"], first["result_hash"]);

        assert_eq!(valid_at(&world, MARCH_12 - 1), [ALICE], "{backend}");
        assert_eq!(valid_at(&world, MARCH_12), [BOB], "{backend}");
        let alice: AssertionId = ALICE.parse().unwrap();
        let earlier = world
            .runtime()
            .replay(ekr_core::RevisionNumber::new(1))
            .unwrap();
        assert!(earlier.assertions[&alice].valid_at(Timestamp::from_millis(MARCH_12)));

        // Exact retries, each in a fresh process after restart and later head movement, return
        // the original retained results byte for byte.
        assert_eq!(seed(&world), seeded, "{backend}: seed retry");
        assert_eq!(commit(&world, T_ALICE), first, "{backend}: commit retry");
        assert_eq!(commit(&world, T_BOB), second, "{backend}: commit retry");
    }
}

#[test]
fn recorded_rejection_and_staleness_are_declared_outcomes_that_exit_zero() {
    for backend in BACKENDS {
        let world = World::new(backend);
        seed(&world);
        propose(&world, "propose-alice.yaml");
        validate(&world, T_ALICE, 0);
        commit(&world, T_ALICE);

        propose(&world, "propose-rejected.yaml");
        let rejected = validate(&world, T_REJECTED, 1);
        assert_eq!(rejected["kind"], "Rejected", "{rejected}");
        assert!(!rejected["issues"].as_array().unwrap().is_empty());

        propose(&world, "propose-bob.yaml");
        propose(&world, "propose-stale.yaml");
        validate(&world, T_BOB, 1);
        validate(&world, T_STALE, 1);
        commit(&world, T_BOB);
        let stale = commit(&world, T_STALE);
        assert_eq!(stale["kind"], "Stale", "{stale}");

        let states: Vec<_> = world.retained().0;
        for (id, state) in [
            (T_ALICE, TransactionState::Committed),
            (T_REJECTED, TransactionState::Rejected),
            (T_BOB, TransactionState::Committed),
            (T_STALE, TransactionState::Stale),
        ] {
            let id: TransactionId = id.parse().unwrap();
            assert!(states.contains(&(id, state)), "{backend}: {states:?}");
        }
        for id in [T_REJECTED, T_STALE] {
            world.refused(&["commit", id], "ekr.kernel.TransactionStateConflict");
        }
    }
}

#[test]
fn declared_refusals_exit_two_with_their_name_and_record_nothing() {
    for backend in BACKENDS {
        let world = World::new(backend);
        world.refused(
            &["seed", &World::fixture_arg("seed-malformed.yaml")],
            "ekr.kernel.InvalidSeed",
        );
        assert!(
            world.runtime().head().unwrap().is_none(),
            "{backend}: refused seed wrote"
        );
        seed(&world);
        propose(&world, "propose-alice.yaml");
        let before = world.retained();

        world.refused(
            &["seed", &World::fixture_arg("seed-different.yaml")],
            "ekr.kernel.AlreadySeeded",
        );
        world.refused(
            &["propose", &World::fixture_arg("propose-malformed.yaml")],
            "ekr.kernel.StructurallyInvalid",
        );
        let garbage = world.run_with_stdin(&["propose", "-"], b"format: [unterminated");
        assert_eq!(garbage.status.code(), Some(2), "{backend}: stdin garbage");
        assert!(String::from_utf8_lossy(&garbage.stderr).contains("ekr.kernel.StructurallyInvalid"));
        world.refused(
            &["validate", T_UNKNOWN, "--against", "0"],
            "ekr.kernel.TransactionNotFound",
        );
        world.refused(
            &["validate", T_ALICE, "--against", "9"],
            "ekr.kernel.RevisionNotFound",
        );
        world.refused(&["commit", T_UNKNOWN], "ekr.kernel.TransactionNotFound");
        world.refused(&["commit", T_ALICE], "ekr.kernel.TransactionStateConflict");
        assert_eq!(world.retained(), before, "{backend}: a refusal recorded");

        validate(&world, T_ALICE, 0);
        world.refused(
            &["validate", T_ALICE, "--against", "0"],
            "ekr.kernel.TransactionStateConflict",
        );
    }
}

#[test]
fn the_proposer_is_bound_to_the_host_operator_or_refused() {
    for backend in BACKENDS {
        let world = World::new(backend);
        seed(&world);
        let before = world.retained();
        let output = world.run(&[
            "propose",
            &World::fixture_arg("propose-foreign-proposer.yaml"),
        ]);
        assert_eq!(output.status.code(), Some(2), "{backend}");
        assert!(String::from_utf8_lossy(&output.stderr).contains("ProposalAttribution"));
        assert_eq!(world.retained(), before);
        let foreign: TransactionId = T_FOREIGN.parse().unwrap();
        assert!(!world.retained().0.iter().any(|(id, _)| *id == foreign));

        let alice = std::fs::read(fixture("propose-alice.yaml")).unwrap();
        let output = world.run_with_stdin(&["propose", "-"], &alice);
        assert_eq!(output.status.code(), Some(0), "{backend}: stdin proposal");
        let record: Value = serde_json::from_slice(&output.stdout).unwrap();
        let host: Value =
            serde_json::from_slice(&std::fs::read(fixture("host.json")).unwrap()).unwrap();
        assert_eq!(record["submitter"], host["context"]["operator"], "{record}");
    }
}

#[test]
fn operational_and_configuration_faults_exit_one() {
    for backend in BACKENDS {
        let world = World::new(backend);
        // An unreadable document is an operational fault, not a refusal.
        let output = world.run(&["seed", "/nonexistent/seed.yaml"]);
        assert_eq!(output.status.code(), Some(1), "{backend}");
        // A host document that does not decode is a configuration fault.
        let bad_host = world.directory.path().join("host.json");
        std::fs::write(&bad_host, b"{\"format\": \"ekr.cli-host/2\"}").unwrap();
        let mut args = world.args(&["seed", &World::fixture_arg("seed.yaml")]);
        args[1] = bad_host.display().to_string();
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(&args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1), "{backend}");
        assert!(world.runtime().head().unwrap().is_none());
        // A command before any seed has no lineage to act on.
        let output = world.run(&["commit", T_ALICE]);
        assert_eq!(output.status.code(), Some(1), "{backend}");
    }
}

#[test]
fn help_exposes_the_declared_p1_wire_verbs() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"))
        .arg("--help")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let help = String::from_utf8_lossy(&output.stdout);
    // `snapshot` and `explain` join this list with the kernel's SnapshotResult/ExplanationResult.
    for verb in ["seed", "propose", "validate", "commit"] {
        assert!(
            help.lines().any(|line| line.trim_start().starts_with(verb)),
            "{verb} missing from {help}"
        );
    }
}

/// The host clock seam: exact Seed and Commit retries return retained results without sampling.
#[test]
fn exact_retries_never_sample_the_host_clock() {
    for backend in BACKENDS {
        let world = World::new(backend);
        let seeded = seed(&world);
        propose(&world, "propose-alice.yaml");
        validate(&world, T_ALICE, 0);
        let committed = commit(&world, T_ALICE);
        let no_clock = || -> Timestamp { panic!("an exact retry sampled the host clock") };
        let retry = |verb: &[&str]| -> Value {
            let mut argv = vec!["ekr".to_owned()];
            argv.extend(world.args(verb));
            let out = ekr::cli::run(argv, &no_clock, &mut std::io::empty())
                .unwrap_or_else(|failure| panic!("{verb:?}: {failure}"));
            serde_json::from_str(&out).unwrap()
        };
        assert_eq!(retry(&["seed", &World::fixture_arg("seed.yaml")]), seeded);
        assert_eq!(retry(&["commit", T_ALICE]), committed);
        // The same seam, entered with an already parsed command line as `main` enters it.
        let mut argv = vec!["ekr".to_owned()];
        argv.extend(world.args(&["commit", T_ALICE]));
        let parsed = <ekr::cli::Cli as clap::Parser>::try_parse_from(argv).unwrap();
        let out = ekr::cli::execute(parsed, &no_clock, &mut std::io::empty()).unwrap();
        assert_eq!(serde_json::from_str::<Value>(&out).unwrap(), committed);
        let different = {
            let mut argv = vec!["ekr".to_owned()];
            argv.extend(world.args(&["seed", &World::fixture_arg("seed-different.yaml")]));
            ekr::cli::run(argv, &no_clock, &mut std::io::empty()).unwrap_err()
        };
        assert_eq!(different.code(), 2);
        assert_eq!(different.name(), Some("ekr.kernel.AlreadySeeded"));
        assert!(matches!(
            different,
            ekr::exit::Failure::Refused {
                name: "ekr.kernel.AlreadySeeded",
                ..
            }
        ));
    }
}

/// A new decision samples the real system clock, in milliseconds since the Unix epoch.
#[test]
fn the_host_clock_is_the_system_clock_in_epoch_milliseconds() {
    let millis = |at: std::time::SystemTime| {
        i64::try_from(
            at.duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis(),
        )
        .unwrap()
    };
    let before = millis(std::time::SystemTime::now());
    let sampled = ekr::cli::system_time().millis();
    let after = millis(std::time::SystemTime::now());
    assert!(
        (before..=after).contains(&sampled),
        "{before} <= {sampled} <= {after}"
    );
}
