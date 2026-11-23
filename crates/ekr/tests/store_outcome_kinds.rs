//! `story:store-outcome-kinds`: a caller tells an unseeded lineage and an uncertain publication
//! apart by a stable name and exit status, not by the English text on stderr.
//!
//! * A store whose tenant has no seed is `ekr.views.NotSeeded`, exit 2, from every verb that meets
//!   it: `head` and `quality` alike, on File and on SQLite, each keeping its own message.
//! * A publication whose outcome the store cannot say (`StoreError::UnknownCommit`) is
//!   `publication-outcome-unknown`, exit 3, on stderr and in a session's answer, with the store's
//!   message unchanged. The store double is the store's own write hook answering `UnknownCommit`
//!   for the write's group, in this process.

use std::path::{Path, PathBuf};
use std::process::Output;

use ekr::exit::{self, Failure};
use ekr_kernel::{CommitError, PersistenceError, StagePoint};

const BACKENDS: [&str; 2] = ["file", "sqlite"];

/// The transaction `ekr example ekr.transaction-document/2` prints.
const TRANSACTION: &str = "00000000-0000-4000-8000-000000000601";

fn binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_ekr"))
}

fn example(name: &str) -> String {
    let output = std::process::Command::new(binary())
        .args(["example", name])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "example {name}");
    String::from_utf8(output.stdout).unwrap()
}

/// A store seeded from the example seed under the example host, and a second host document for
/// the same store whose tenant has no seed.
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
        let host = example("ekr.cli-host/1");
        std::fs::write(world.path("host.json"), &host).unwrap();
        let tenant = "\"tenant\": \"";
        assert!(host.contains(tenant), "{host}");
        std::fs::write(
            world.path("unseeded.json"),
            host.replacen(tenant, "\"tenant\": \"never-seeded-", 1),
        )
        .unwrap();
        std::fs::write(world.path("seed.yaml"), example("ekr-seed/2")).unwrap();
        std::fs::write(
            world.path("transaction.yaml"),
            example("ekr.transaction-document/2"),
        )
        .unwrap();
        let seeded = world.run("host.json", &["seed", "seed.yaml"]);
        assert_eq!(seeded.status.code(), Some(0), "{}", stderr(&seeded));
        world
    }

    fn path(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    fn configuration(&self, host: &str) -> Vec<String> {
        vec![
            "--host".to_owned(),
            self.path(host).display().to_string(),
            "--store".to_owned(),
            self.path("store").display().to_string(),
            "--backend".to_owned(),
            self.backend.to_owned(),
        ]
    }

    fn run(&self, host: &str, args: &[&str]) -> Output {
        std::process::Command::new(binary())
            .args(self.configuration(host))
            .args(args)
            .current_dir(self.directory.path())
            .env_remove("EKR_HOST")
            .env_remove("EKR_STORE")
            .env_remove("EKR_BACKEND")
            .env_remove("EKR_STAGE")
            .output()
            .unwrap()
    }

    /// `args` in this process, under the seeded host.
    fn in_process(&self, args: &[&str], stdin: &[u8]) -> Result<String, Failure> {
        let mut argv = vec!["ekr".to_owned()];
        argv.extend(self.configuration("host.json"));
        argv.extend(args.iter().map(|arg| (*arg).to_owned()));
        let mut input = stdin;
        ekr::cli::run(argv, &ekr::cli::system_time, &mut input)
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn path_text(path: &Path) -> String {
    path.display().to_string()
}

#[test]
fn an_unseeded_tenant_is_one_named_refusal_with_one_exit_status_on_both_providers() {
    let refusal = format!("ekr: {}: ", exit::NOT_SEEDED);
    for backend in BACKENDS {
        let world = World::new(backend);
        let mut kinds = Vec::new();
        for (args, message) in [
            (vec!["head"], "the lineage has no seed"),
            (vec!["quality"], "the store was never seeded"),
            (vec!["snapshot"], "the lineage has no seed"),
            (vec!["ontology"], "the lineage has no seed"),
            (vec!["transactions"], "the lineage has no seed"),
            (
                vec!["propose", "transaction.yaml"],
                "the lineage has no seed",
            ),
            (vec!["commit", TRANSACTION], "the lineage has no seed"),
        ] {
            let output = world.run("unseeded.json", &args);
            let text = stderr(&output);
            assert!(
                text.starts_with(&refusal),
                "{backend} {args:?}: exit {:?}, stderr {text:?}",
                output.status.code()
            );
            assert!(
                text.contains(message),
                "{backend} {args:?}: the human message changed: {text:?}"
            );
            kinds.push((args[0], output.status.code()));
        }
        assert!(
            kinds.iter().all(|(_, code)| *code == Some(2)),
            "{backend}: {kinds:?}"
        );
        // The seeded tenant of the same store still answers.
        let head = world.run("host.json", &["head"]);
        assert_eq!(head.status.code(), Some(0), "{}", stderr(&head));
    }
    let failure = Failure::from(CommitError::NotSeeded);
    assert_eq!(
        (failure.code(), failure.name()),
        (2, Some(exit::NOT_SEEDED))
    );
    assert_eq!(
        failure.to_string(),
        "ekr: ekr.views.NotSeeded: the lineage has no seed"
    );
    // A lineage whose first record is not a seed is damaged, not empty: still a fault.
    assert_eq!(Failure::store(PersistenceError::NotSeeded).code(), 1);
}

/// The write's group answers `UnknownCommit` once the hook is set: what a provider whose
/// response was lost answers.
fn uncertain() -> ekr_kernel::StageHookGuard {
    ekr_kernel::on_stage_point(|point| match point {
        StagePoint::Group { .. } => Err(PersistenceError::UnknownCommit),
        _ => Ok(()),
    })
}

#[test]
fn an_unknown_publication_outcome_is_its_own_name_and_exit_status() {
    let message = "publication outcome is unknown; resolve the exact retained occurrence";
    let expected = format!("ekr: {}: {message}", exit::PUBLICATION_OUTCOME_UNKNOWN);
    for backend in BACKENDS {
        let world = World::new(backend);
        let transaction = path_text(&world.path("transaction.yaml"));
        world.in_process(&["propose", &transaction], b"").unwrap();
        world.in_process(&["validate", TRANSACTION], b"").unwrap();

        let failure = {
            let _hook = uncertain();
            world.in_process(&["commit", TRANSACTION], b"")
        }
        .expect_err("the commit's outcome is unknown");
        assert!(
            matches!(&failure, Failure::Uncertain { name, .. } if *name == exit::PUBLICATION_OUTCOME_UNKNOWN),
            "{backend}: {failure:?}"
        );
        assert_eq!(failure.code(), exit::UNKNOWN_OUTCOME_EXIT, "{backend}");
        assert_eq!(exit::UNKNOWN_OUTCOME_EXIT, 3);
        assert_eq!(failure.to_string(), expected, "{backend}");
        assert_eq!(failure.name(), None, "{backend}: not a refusal");

        // A session answers it in its envelope: the same exit status and stderr.
        let request = format!("{}\n", serde_json::json!({"argv": ["commit", TRANSACTION]}));
        let answered = {
            let _hook = uncertain();
            world.in_process(&["session"], request.as_bytes())
        }
        .unwrap();
        let answer: serde_json::Value =
            serde_json::from_str(answered.lines().next().unwrap()).unwrap();
        assert_eq!(answer["exit"], 3, "{backend}: {answer}");
        assert_eq!(
            answer["stderr"],
            format!("{expected}\n"),
            "{backend}: {answer}"
        );
    }
    assert_eq!(
        Failure::store(PersistenceError::UnknownCommit).code(),
        exit::UNKNOWN_OUTCOME_EXIT
    );
}
