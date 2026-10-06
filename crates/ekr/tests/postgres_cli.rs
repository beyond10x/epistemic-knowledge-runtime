//! Hosted provider configuration is referenced by path, never supplied as credentials in argv.
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn ekr(args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(args)
        .env_remove("EKR_HOST")
        .env_remove("EKR_STORE")
        .env_remove("EKR_BACKEND")
        .env_remove("EKR_FULL_REPLAY")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn fixture() -> Option<PathBuf> {
    let Some(config) = std::env::var_os("EKR_TEST_POSTGRES_CONFIG") else {
        assert_ne!(
            std::env::var("EKR_REQUIRE_POSTGRES").as_deref(),
            Ok("1"),
            "required PostgreSQL fixture missing"
        );
        eprintln!("SKIP real PostgreSQL: EKR_TEST_POSTGRES_CONFIG is unset");
        return None;
    };
    let owner = std::env::var_os("EKR_TEST_POSTGRES_OWNER").unwrap();
    ekr(&[
        "postgres-schema",
        "--config",
        Path::new(&owner).to_str().unwrap(),
    ]);
    Some(config.into())
}

fn host(directory: &Path) -> PathBuf {
    let mut document = ekr(&["example", "ekr.cli-host/1"]);
    document["tenant"] = format!("cli-hosted-{}", ekr_core::NodeId::mint()).into();
    let path = directory.join("host.json");
    std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
    path
}

#[test]
fn postgres_configuration_errors_never_expose_credentials() {
    let directory =
        std::env::temp_dir().join(format!("ekr-postgres-config-{}", std::process::id()));
    std::fs::create_dir(&directory).unwrap();
    let config = directory.join("postgres.json");
    let credential = ["synthetic", "private", "value"].join("-");
    std::fs::write(&config, format!("{{\"format\":\"{credential}\"}}")).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(["postgres-schema", "--config"])
        .arg(&config)
        .output()
        .unwrap();
    let error = String::from_utf8(output.stderr).unwrap();
    std::fs::remove_file(config).unwrap();
    std::fs::remove_dir(directory).unwrap();
    assert_eq!(output.status.code(), Some(1), "{error}");
    assert!(error.contains("postgres-configuration"), "{error}");
    assert!(!error.contains(&credential));
}

#[test]
fn sqlite_to_postgres_cli_copies_and_reopens_one_history() {
    let Some(config) = fixture() else { return };
    let directory = tempfile::tempdir().unwrap();
    let host = host(directory.path());
    let seed = directory.path().join("seed.yaml");
    let example = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(["example", "ekr-seed/2"])
        .output()
        .unwrap();
    assert!(example.status.success());
    std::fs::write(&seed, example.stdout).unwrap();
    let source = directory.path().join("source.db");
    let local = [
        "--host",
        host.to_str().unwrap(),
        "--backend",
        "sqlite",
        "--store",
        source.to_str().unwrap(),
    ];
    let remote = [
        "--host",
        host.to_str().unwrap(),
        "--backend",
        "postgres",
        "--store",
        config.to_str().unwrap(),
    ];
    ekr(&[local.as_slice(), &["seed", seed.to_str().unwrap()]].concat());
    let before = ekr(&[local.as_slice(), &["snapshot"]].concat());
    let report = ekr(&[
        local.as_slice(),
        &[
            "migrate",
            "--to-backend",
            "postgres",
            "--to",
            config.to_str().unwrap(),
        ],
    ]
    .concat());
    assert_eq!(report["format"], "ekr.store-migration/1");
    let after = ekr(&[remote.as_slice(), &["snapshot"]].concat());
    assert_eq!(before["graph"], after["graph"]);
    assert_eq!(before["revision_id"], after["revision_id"]);
    for field in [
        "revision",
        "ontology_root",
        "knowledge_root",
        "evidence_root",
        "agent_root",
    ] {
        assert_eq!(before["root"][field], after["root"][field]);
    }
    let host_document =
        ekr::host::CliHostConfigurationV1::from_json(&std::fs::read(&host).unwrap()).unwrap();
    let runtime = ekr_kernel::Runtime::postgres(
        &ekr_kernel::runtime::PostgresConfiguration::read(&config).unwrap(),
        &host_document.tenant,
        host_document.context,
        host_document.authority,
        true,
    )
    .unwrap();
    let events = runtime.published_events().unwrap();
    ekr(&[remote.as_slice(), &["head"]].concat());
    assert_eq!(
        runtime.published_events().unwrap(),
        events,
        "CLI reads never checkpoint"
    );
}

#[test]
#[cfg(target_os = "linux")]
fn sdk_postgres_session_passes_only_configuration_paths() {
    use ekr_sdk::{
        binary::EkrBinary,
        session::{Backend, ProcessSession, SessionOptions, StoreConfig},
        transport::{Request, Transport},
    };
    let Some(config) = fixture() else { return };
    let directory = tempfile::tempdir().unwrap();
    let host = host(directory.path());
    let seed = directory.path().join("seed.yaml");
    let example = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(["example", "ekr-seed/2"])
        .output()
        .unwrap();
    assert!(example.status.success());
    std::fs::write(&seed, example.stdout).unwrap();
    ekr(&[
        "--host",
        host.to_str().unwrap(),
        "--backend",
        "postgres",
        "--store",
        config.to_str().unwrap(),
        "seed",
        seed.to_str().unwrap(),
    ]);
    let binary = EkrBinary::open(Path::new(env!("CARGO_BIN_EXE_ekr"))).unwrap();
    let store = StoreConfig {
        host: host.clone(),
        store: config.clone(),
        backend: Backend::Postgres,
    };
    let mut session = ProcessSession::start(&binary, store, SessionOptions::default()).unwrap();
    let reply = session.request(&Request::new(["head"])).unwrap();
    assert_eq!(reply.exit, 0);
    let command = std::fs::read(format!("/proc/{}/cmdline", session.id())).unwrap();
    let args: Vec<_> = command
        .split(|byte| *byte == 0)
        .filter(|part| !part.is_empty())
        .map(|part| std::str::from_utf8(part).unwrap())
        .collect();
    assert_eq!(
        &args[1..],
        [
            "--host",
            host.to_str().unwrap(),
            "--store",
            config.to_str().unwrap(),
            "--backend",
            "postgres",
            "session",
            "--create"
        ]
    );
    session.close().unwrap();
}

#[test]
fn previous_release_reads_ordinary_seeds_and_refuses_claimed_migrations() {
    let Some(previous) = std::env::var_os("EKR_PREVIOUS_BINARY") else {
        assert_ne!(
            std::env::var("EKR_REQUIRE_PREVIOUS_BINARY").as_deref(),
            Ok("1"),
            "required previous-release binary reference missing"
        );
        eprintln!("SKIP previous-release compatibility: EKR_PREVIOUS_BINARY is unset");
        return;
    };
    let directory = tempfile::tempdir().unwrap();
    let host = host(directory.path());
    let seed = directory.path().join("seed.yaml");
    let example = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(["example", "ekr-seed/2"])
        .output()
        .unwrap();
    assert!(example.status.success());
    std::fs::write(&seed, example.stdout).unwrap();
    let source = directory.path().join("source.db");
    let destination = directory.path().join("copy.db");
    let local = [
        "--host",
        host.to_str().unwrap(),
        "--backend",
        "sqlite",
        "--store",
        source.to_str().unwrap(),
    ];
    ekr(&[local.as_slice(), &["seed", seed.to_str().unwrap()]].concat());
    let old_head = Command::new(&previous)
        .args(local)
        .arg("head")
        .output()
        .unwrap();
    assert!(
        old_head.status.success(),
        "ordinary /3 seed remains readable: {}",
        String::from_utf8_lossy(&old_head.stderr)
    );
    let report = ekr(&[
        local.as_slice(),
        &["migrate", "--to", destination.to_str().unwrap()],
    ]
    .concat());
    assert_ne!(report["source_seed_hash"], report["destination_seed_hash"]);
    let assert_old_refuses = || {
        let refused = Command::new(&previous)
            .args([
                "--host",
                host.to_str().unwrap(),
                "--backend",
                "sqlite",
                "--store",
                destination.to_str().unwrap(),
                "head",
            ])
            .output()
            .unwrap();
        assert_eq!(refused.status.code(), Some(1));
        assert!(refused.stdout.is_empty(), "older reader must serve no head");
        let error = String::from_utf8_lossy(&refused.stderr);
        assert!(
            error.contains("seed-decode") || error.contains("unsupported-seed-envelope"),
            "{error}"
        );
    };
    assert_old_refuses();
    let current = ekr(&[
        "--host",
        host.to_str().unwrap(),
        "--backend",
        "sqlite",
        "--store",
        destination.to_str().unwrap(),
        "head",
    ]);
    assert!(current.is_object());
    let remote = [
        "--host",
        host.to_str().unwrap(),
        "--backend",
        "sqlite",
        "--store",
        destination.to_str().unwrap(),
    ];
    let transaction = directory.path().join("transaction.yaml");
    let example = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(["example", "ekr.transaction-document/2"])
        .output()
        .unwrap();
    assert!(example.status.success());
    std::fs::write(&transaction, example.stdout).unwrap();
    let proposal = ekr(&[
        remote.as_slice(),
        &["propose", transaction.to_str().unwrap()],
    ]
    .concat());
    let id = proposal["transaction_id"].as_str().unwrap();
    ekr(&[remote.as_slice(), &["validate", id]].concat());
    ekr(&[remote.as_slice(), &["commit", id]].concat());
    assert_old_refuses();
    // Reopen through checkpoint restore, then publish a newer checkpoint from that admitted
    // state. Its migration binding must survive both restoration and subsequent commits.
    let host_document =
        ekr::host::CliHostConfigurationV1::from_json(&std::fs::read(&host).unwrap()).unwrap();
    let runtime = ekr_kernel::Runtime::sqlite_existing(
        &destination,
        &host_document.tenant,
        host_document.context,
        host_document.authority,
    )
    .unwrap();
    runtime.read(None).unwrap();
    let pointers = || {
        runtime
            .published_events()
            .unwrap()
            .into_iter()
            .filter(|event| event.stream_type == "ekr.checkpoint")
            .collect::<Vec<_>>()
    };
    let before = pointers();
    runtime.retain_checkpoint_at_rest();
    let after = pointers();
    assert!(
        after.len() > before.len(),
        "a later checkpoint must actually be written"
    );
    assert_ne!(
        before.last().unwrap().data["checkpoint_hash"],
        after.last().unwrap().data["checkpoint_hash"]
    );
    drop(runtime);
    assert_old_refuses();
}

/// `story:seed-if-absent`, its acceptance: two concurrent `ekr seed --if-absent` processes with
/// the identical seed on one PostgreSQL tenant. Exactly one exits 0; the other exits 2 refused as
/// `ekr.kernel.AlreadySeeded`, and the tenant holds the first one's seed.
#[test]
fn two_concurrent_if_absent_seeds_on_one_tenant_create_exactly_one() {
    let Some(config) = fixture() else { return };
    let directory = tempfile::tempdir().unwrap();
    let host = host(directory.path());
    let seed = directory.path().join("seed.yaml");
    let example = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(["example", "ekr-seed/2"])
        .output()
        .unwrap();
    assert!(example.status.success());
    std::fs::write(&seed, example.stdout).unwrap();
    let remote = [
        "--host",
        host.to_str().unwrap(),
        "--backend",
        "postgres",
        "--store",
        config.to_str().unwrap(),
    ];
    let spawn = || {
        Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(remote)
            .args(["seed", "--if-absent", seed.to_str().unwrap()])
            .env_remove("EKR_HOST")
            .env_remove("EKR_STORE")
            .env_remove("EKR_BACKEND")
            .env_remove("EKR_FULL_REPLAY")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };
    let callers = [spawn(), spawn()];
    let outputs: Vec<_> = callers
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
    let created: Vec<_> = outputs
        .iter()
        .filter(|output| output.status.success())
        .collect();
    assert_eq!(created.len(), 1, "{described:?}");
    let refused = outputs
        .iter()
        .find(|output| !output.status.success())
        .unwrap();
    let stderr = String::from_utf8_lossy(&refused.stderr);
    assert_eq!(refused.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("ekr.kernel.AlreadySeeded"), "{stderr}");
    let result: Value = serde_json::from_slice(&created[0].stdout).unwrap();
    let head = ekr(&[remote.as_slice(), &["head"]].concat());
    assert_eq!(head["root"], result["result"], "{head} {result}");
    // A later --if-absent seed refuses the same way; the plain seed answers the first result.
    let again = Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(remote)
        .args(["seed", "--if-absent", seed.to_str().unwrap()])
        .env_remove("EKR_HOST")
        .env_remove("EKR_STORE")
        .env_remove("EKR_BACKEND")
        .env_remove("EKR_FULL_REPLAY")
        .output()
        .unwrap();
    assert_eq!(again.status.code(), Some(2));
    assert_eq!(
        ekr(&[remote.as_slice(), &["seed", seed.to_str().unwrap()]].concat()),
        result
    );
}

/// Adversary (story:seed-if-absent, pass 1): three `ekr seed --if-absent` processes and one plain
/// `ekr seed`, identical document, one fresh PostgreSQL tenant. The plain seed exits 0 with the
/// lineage's result; at most one if-absent caller exits 0, and only if the plain seed answered
/// its seed; every other if-absent caller is refused as `ekr.kernel.AlreadySeeded` (exit 2).
#[test]
fn adversary_if_absent_processes_racing_a_plain_seed_on_one_tenant() {
    let Some(config) = fixture() else { return };
    for round in 0..4 {
        let directory = tempfile::tempdir().unwrap();
        let host = host(directory.path());
        let seed = directory.path().join("seed.yaml");
        let example = Command::new(env!("CARGO_BIN_EXE_ekr"))
            .args(["example", "ekr-seed/2"])
            .output()
            .unwrap();
        assert!(example.status.success());
        std::fs::write(&seed, example.stdout).unwrap();
        let remote = [
            "--host",
            host.to_str().unwrap(),
            "--backend",
            "postgres",
            "--store",
            config.to_str().unwrap(),
        ];
        let spawn = |if_absent: bool| {
            let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
            command.args(remote).arg("seed");
            if if_absent {
                command.arg("--if-absent");
            }
            command
                .arg(&seed)
                .env_remove("EKR_HOST")
                .env_remove("EKR_STORE")
                .env_remove("EKR_BACKEND")
                .env_remove("EKR_FULL_REPLAY")
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap()
        };
        let callers = [spawn(true), spawn(false), spawn(true), spawn(true)];
        let outputs: Vec<_> = callers
            .into_iter()
            .map(|caller| caller.wait_with_output().unwrap())
            .collect();
        let described: Vec<_> = outputs
            .iter()
            .map(|o| {
                (
                    o.status.code(),
                    String::from_utf8_lossy(&o.stderr).into_owned(),
                )
            })
            .collect();
        assert_eq!(
            outputs[1].status.code(),
            Some(0),
            "round {round}: {described:#?}"
        );
        let lineage: Value = serde_json::from_slice(&outputs[1].stdout).unwrap();
        let mut created = 0;
        for output in [&outputs[0], &outputs[2], &outputs[3]] {
            if output.status.success() {
                created += 1;
                let mine: Value = serde_json::from_slice(&output.stdout).unwrap();
                assert_eq!(mine, lineage, "round {round}: {described:#?}");
            } else {
                assert_eq!(
                    output.status.code(),
                    Some(2),
                    "round {round}: {described:#?}"
                );
                assert!(
                    String::from_utf8_lossy(&output.stderr).contains("ekr.kernel.AlreadySeeded"),
                    "round {round}: {described:#?}"
                );
            }
        }
        assert!(created <= 1, "round {round}: {described:#?}");
        let head = ekr(&[remote.as_slice(), &["head"]].concat());
        assert_eq!(head["root"], lineage["result"], "round {round}");
    }
}
