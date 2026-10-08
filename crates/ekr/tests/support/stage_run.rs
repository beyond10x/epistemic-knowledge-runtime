//! The `ekr stage` cases of design § 107.10 for `task:stage-cli`, each run against one provider:
//! `stage_cli.rs` runs every one on SQLite and `postgres_cli.rs` on PostgreSQL where the
//! PostgreSQL tests run (`story:a-run-is-staged-and-published-whole`, its acceptance).
//!
//! A run is what a consumer does between `ekr stage begin` and `ekr stage publish` or
//! `ekr stage abandon`: separate one-shot `ekr` processes, each joined to the stage by `EKR_STAGE`
//! alone, then a gate on what the stage holds. Every case drives the built binary. The one case
//! that needs a publication interrupted after its append runs that first attempt through the same
//! CLI code in this process (`ekr::cli::run`), interrupted by the kernel's stage hook, and retries
//! it through the binary. What a stage's tenant still holds is read through
//! `Runtime::stage_tenant_events`, which opens that tenant and decides nothing.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

/// Where a lab's store is.
pub enum Provider<'a> {
    /// A SQLite database in the lab's directory. Only `stage_cli.rs` constructs it.
    #[allow(dead_code)]
    Sqlite,
    /// The hosted PostgreSQL store an `ekr.postgres/1` application configuration names, under a
    /// tenant of its own. Only `postgres_cli.rs` constructs it.
    #[allow(dead_code)]
    Postgres(&'a Path),
}

/// One store, seeded from `ekr example ekr-seed/2` under a validation profile v2 host, so a run
/// can grow the schema with `ekr apply-extraction`.
pub struct Lab {
    directory: tempfile::TempDir,
    backend: &'static str,
    store: PathBuf,
}

/// The `ekr` binary with no configuration from this process's environment.
fn bare() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_ekr"));
    for variable in [
        "EKR_HOST",
        "EKR_STORE",
        "EKR_BACKEND",
        "EKR_FULL_REPLAY",
        "EKR_STAGE",
    ] {
        command.env_remove(variable);
    }
    command
}

fn text(args: &[&str]) -> String {
    let output = bare().args(args).output().unwrap();
    assert!(output.status.success(), "ekr {args:?}");
    String::from_utf8(output.stdout).unwrap()
}

/// The stage id a JSON stage result names.
pub fn stage_of(result: &Value) -> String {
    let id = result["stage_id"].as_str().expect("a stage id").to_owned();
    id.parse::<ekr_kernel::StageId>()
        .unwrap_or_else(|error| panic!("{id:?} is not a stage id: {error}"));
    id
}

impl Lab {
    /// A seeded store on `provider`.
    pub fn new(provider: Provider<'_>) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let (backend, store, tenant) = match provider {
            Provider::Sqlite => ("sqlite", directory.path().join("store.db"), None),
            Provider::Postgres(config) => (
                "postgres",
                config.to_owned(),
                Some(format!("stage-cli-{}", ekr_core::NodeId::mint())),
            ),
        };
        let lab = Self {
            directory,
            backend,
            store,
        };
        let mut host: Value = serde_json::from_str(&text(&["example", "ekr.cli-host/1"])).unwrap();
        let profile = &mut host["authority"]["validation_profile"];
        profile["ruleset"] = "ekr.p2-deterministic/1".into();
        profile["application"] = "ekr.p2-apply/1".into();
        if let Some(tenant) = tenant {
            host["tenant"] = tenant.into();
        }
        std::fs::write(lab.file("host.json"), serde_json::to_vec(&host).unwrap()).unwrap();
        for (name, format) in [
            ("seed.yaml", "ekr-seed/2"),
            ("transaction.yaml", "ekr.transaction-document/2"),
            ("extraction.yaml", "ekr.extraction-document/1"),
            ("reference.yaml", "typed-reference"),
        ] {
            std::fs::write(lab.file(name), text(&["example", format])).unwrap();
        }
        lab.json(None, &["seed", "seed.yaml"]);
        lab
    }

    /// A file in the lab's directory.
    pub fn file(&self, name: &str) -> PathBuf {
        self.directory.path().join(name)
    }

    /// The configuration flags every store verb of this lab takes.
    pub fn configuration(&self) -> Vec<String> {
        vec![
            "--host".into(),
            self.file("host.json").to_str().unwrap().into(),
            "--store".into(),
            self.store.to_str().unwrap().into(),
            "--backend".into(),
            self.backend.into(),
        ]
    }

    /// `ekr <configuration> <args>` in the lab's directory, joined to `stage` by `EKR_STAGE` alone
    /// when one is given.
    pub fn command(&self, stage: Option<&str>, args: &[&str]) -> Command {
        let mut command = bare();
        command
            .current_dir(self.directory.path())
            .args(self.configuration())
            .args(args);
        if let Some(stage) = stage {
            command.env("EKR_STAGE", stage);
        }
        command
    }

    pub fn run(&self, stage: Option<&str>, args: &[&str]) -> Output {
        self.command(stage, args).output().unwrap()
    }

    /// The JSON document a verb that exits 0 prints.
    pub fn json(&self, stage: Option<&str>, args: &[&str]) -> Value {
        let output = self.run(stage, args);
        assert_eq!(
            output.status.code(),
            Some(0),
            "ekr {args:?} (stage {stage:?}): {}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    }

    /// Asserts the verb is refused `name` with exit `code`, printed as `ekr: <name>: <reason>`,
    /// and prints nothing on stdout.
    pub fn refused(&self, stage: Option<&str>, args: &[&str], name: &str, code: i32) {
        let output = self.run(stage, args);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(
            output.status.code(),
            Some(code),
            "ekr {args:?} (stage {stage:?}) refuses {name} with exit {code}: {stderr}"
        );
        assert!(
            stderr.starts_with(&format!("ekr: {name}: ")),
            "ekr {args:?} (stage {stage:?}) names {name}: {stderr}"
        );
        assert!(output.stdout.is_empty(), "a refusal prints nothing");
    }

    /// Asserts the verb is a usage error: exit 2, nothing on stdout, `needle` on stderr.
    pub fn usage(&self, stage: Option<&str>, args: &[&str], needle: &str) {
        let output = self.run(stage, args);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "ekr {args:?}: {stderr}");
        assert!(stderr.contains(needle), "ekr {args:?}: {stderr}");
        assert!(output.stdout.is_empty(), "a usage error prints nothing");
    }

    /// The store's own runtime, opened in this process to read what a stage's tenant holds.
    fn runtime(&self) -> ekr_kernel::Runtime {
        let host = ekr::host::CliHostConfigurationV1::from_json(
            &std::fs::read(self.file("host.json")).unwrap(),
        )
        .unwrap();
        match self.backend {
            "sqlite" => ekr_kernel::Runtime::sqlite_existing(
                &self.store,
                &host.tenant,
                host.context,
                host.authority,
            ),
            _ => ekr_kernel::Runtime::postgres(
                &ekr_kernel::runtime::PostgresConfiguration::read(&self.store).unwrap(),
                &host.tenant,
                host.context,
                host.authority,
                true,
            ),
        }
        .unwrap()
    }

    /// How many events the tenant of `stage` holds.
    pub fn held(&self, stage: &str) -> usize {
        self.runtime()
            .stage_tenant_events(stage.parse().unwrap())
            .unwrap()
    }

    /// A fresh transaction of the example's shape, its ids minted by `ekr mint` run joined to
    /// `stage`, which opens no store and ignores it. With `refused`, its relation names a type the
    /// ontology does not declare, so its validation rejects it (`unknown-type`).
    pub fn transaction(&self, stage: Option<&str>, refused: bool) -> PathBuf {
        let mint = |kind: &str| {
            self.json(stage, &["mint", kind])["id"]
                .as_str()
                .unwrap()
                .to_owned()
        };
        let mut body = std::fs::read_to_string(self.file("transaction.yaml")).unwrap();
        for (old, kind) in [
            ("00000000-0000-4000-8000-000000000601", "transaction"),
            ("00000000-0000-4000-8000-000000000501", "assertion"),
        ] {
            assert!(body.contains(old));
            body = body.replace(old, &mint(kind));
        }
        if refused {
            let relation = "!Relation 00000000-0000-4000-8000-000000000203";
            assert!(body.contains(relation));
            body = body.replace(relation, &format!("!Relation {}", mint("type")));
        }
        let path = self.file(&format!("{}.yaml", mint("transaction")));
        std::fs::write(&path, body).unwrap();
        path
    }

    /// Proposes, validates and commits a fresh transaction, joined to `stage` when one is given,
    /// and returns the commit's outcome.
    pub fn commit(&self, stage: Option<&str>) -> Value {
        let document = self.transaction(stage, false);
        let proposed = self.json(stage, &["propose", document.to_str().unwrap()]);
        let id = proposed["transaction_id"].as_str().unwrap().to_owned();
        let validated = self.json(stage, &["validate", &id]);
        assert_eq!(validated["kind"], "Validated", "{validated}");
        let committed = self.json(stage, &["commit", &id]);
        assert_eq!(committed["kind"], "Committed", "{committed}");
        committed
    }

    /// The gate a consumer runs on a stage before it publishes: the run's validations rejected
    /// nothing, and its quality reads.
    pub fn gate(&self, stage: &str) -> bool {
        let rejections = self.json(Some(stage), &["rejections"]);
        let quality = self.run(Some(stage), &["quality"]);
        rejections["rejections"].as_array().unwrap().is_empty() && quality.status.success()
    }

    /// The stage `ekr stage list` prints for `stage`.
    pub fn listed(&self, stage: &str) -> Value {
        self.json(None, &["stage", "list"])
            .as_array()
            .expect("ekr stage list prints a list")
            .iter()
            .find(|listed| listed["stage_id"] == stage)
            .unwrap_or_else(|| panic!("ekr stage list lists {stage}"))
            .clone()
    }

    /// The store's own tenant, as the host names it.
    pub fn tenant(&self) -> String {
        let host: Value =
            serde_json::from_slice(&std::fs::read(self.file("host.json")).unwrap()).unwrap();
        host["tenant"].as_str().unwrap().to_owned()
    }
}

/// The revision number `ekr head` printed, as `--expect-head` takes it.
fn revision(head: &Value) -> String {
    head["revision"].as_u64().unwrap().to_string()
}

/// Two `ekr head` documents name the same state: the same revision and the same knowledge,
/// evidence, ontology and authority roots. A stage's own addresses — the seed envelope its copy
/// claims, a record of its lineage — are its own and not compared (design § 107.9).
fn assert_same_state(left: &Value, right: &Value) {
    assert_eq!(left["revision"], right["revision"], "{left} {right}");
    for root in [
        "revision",
        "ontology_root",
        "knowledge_root",
        "evidence_root",
        "agent_root",
    ] {
        assert_eq!(
            left["root"][root], right["root"][root],
            "{root}: {left} {right}"
        );
    }
}

/// `ekr stage begin` prints the stage it began: an id it minted, Begun at the store's head. A
/// second begin mints another. Both are listed, the store's head and history do not move, and the
/// stage's tenant holds the store's copy.
pub fn stage_begin_prints_the_minted_id(provider: Provider<'_>) {
    let lab = Lab::new(provider);
    let head = lab.json(None, &["head"]);
    let history = lab.json(None, &["transactions"]);
    let first = lab.json(None, &["stage", "begin"]);
    let second = lab.json(None, &["stage", "begin"]);
    let (one, two) = (stage_of(&first), stage_of(&second));
    assert_ne!(one, two, "each begin mints its own id");
    for begun in [&first, &second] {
        assert_eq!(begun["state"], "Begun", "{begun}");
        assert_eq!(begun["base"], head["revision"], "{begun}");
        assert_eq!(begun["published_last"], Value::Null, "{begun}");
    }
    for id in [&one, &two] {
        let listed = lab.listed(id);
        assert_eq!(listed["state"], "Begun", "{listed}");
        assert_eq!(listed["store"], lab.tenant().as_str(), "{listed}");
        assert_eq!(listed["base"], head["revision"], "{listed}");
        assert_eq!(listed["published_revisions"], serde_json::json!([]));
        assert!(lab.held(id) > 0, "the stage's tenant holds the copy");
    }
    assert_eq!(lab.json(None, &["head"]), head, "begin moves no head");
    assert_eq!(lab.json(None, &["transactions"]), history);
    // What `ekr stage list` prints is the kernel's own listing of the store's records.
    let runtime = lab.runtime();
    assert_eq!(runtime.store_tenant(), lab.tenant());
    let listings: Vec<ekr_kernel::StageListing> = runtime.stages().unwrap();
    let mut kernel: Vec<String> = listings
        .iter()
        .map(|listing| listing.stage.stage_id.to_string())
        .collect();
    let mut printed: Vec<String> = lab
        .json(None, &["stage", "list"])
        .as_array()
        .unwrap()
        .iter()
        .map(|listed| listed["stage_id"].as_str().unwrap().to_owned())
        .collect();
    kernel.sort();
    printed.sort();
    assert_eq!(kernel, printed);
    assert!(listings
        .iter()
        .all(|listing| listing.state == ekr_kernel::StageState::Begun));
    drop(runtime);
    for id in [&one, &two] {
        lab.json(None, &["stage", "abandon", id]);
    }
}

/// A run whose gate fails is abandoned, and `ekr head` afterwards prints what it printed before
/// the run. The run's commit was read back joined; the store never held it, and the stage's
/// tenant holds nothing once it is abandoned.
pub fn a_failed_run_in_a_stage_leaves_ekr_head_where_it_was(provider: Provider<'_>) {
    let lab = Lab::new(provider);
    let before = lab.json(None, &["head"]);
    let history = lab.json(None, &["transactions"]);
    let stage = stage_of(&lab.json(None, &["stage", "begin"]));
    let stage = stage.as_str();

    lab.commit(Some(stage));
    let refused = lab.transaction(Some(stage), true);
    let proposed = lab.json(Some(stage), &["propose", refused.to_str().unwrap()]);
    let rejected = lab.json(
        Some(stage),
        &["validate", proposed["transaction_id"].as_str().unwrap()],
    );
    assert_eq!(rejected["kind"], "Rejected", "{rejected}");
    let joined = lab.json(Some(stage), &["head"]);
    assert_eq!(
        joined["revision"].as_u64(),
        before["revision"].as_u64().map(|base| base + 1),
        "the run reads its own commit"
    );
    assert_eq!(
        lab.json(None, &["head"]),
        before,
        "the store holds none of it"
    );
    assert!(!lab.gate(stage), "the run's gate fails on its rejection");

    let abandoned = lab.json(None, &["stage", "abandon", stage]);
    assert_eq!(abandoned["state"], "Abandoned", "{abandoned}");
    assert_eq!(
        lab.json(None, &["head"]),
        before,
        "ekr head is where it was"
    );
    assert_eq!(lab.json(None, &["transactions"]), history);
    assert_eq!(lab.held(stage), 0, "the stage's tenant holds nothing");
    assert_eq!(lab.listed(stage)["state"], "Abandoned");
    lab.refused(Some(stage), &["head"], "stage-already-abandoned", 2);
    lab.refused(
        None,
        &[
            "stage",
            "publish",
            stage,
            "--expect-head",
            &revision(&before),
        ],
        "stage-already-abandoned",
        2,
    );
    assert_eq!(
        lab.json(None, &["stage", "abandon", stage]),
        abandoned,
        "a retried abandon answers the original result"
    );
}

/// A run whose gate passes is published: `ekr head` is the stage's last revision, derived again
/// for the store — the same revision, identity, graph and roots — and the store's whole history
/// replays from its seed. The stage's tenant holds nothing and its record lists the revisions.
pub fn a_passed_run_is_published_and_ekr_head_is_the_stages_last_revision(provider: Provider<'_>) {
    let lab = Lab::new(provider);
    let before = lab.json(None, &["head"]);
    let begun = lab.json(None, &["stage", "begin"]);
    let stage = stage_of(&begun);
    let stage = stage.as_str();
    lab.commit(Some(stage));
    lab.commit(Some(stage));
    assert!(lab.gate(stage), "the run's gate passes");
    let staged_head = lab.json(Some(stage), &["head"]);
    let staged = lab.json(Some(stage), &["snapshot"]);
    let committed: Vec<Value> = lab
        .json(Some(stage), &["transactions"])
        .as_array()
        .unwrap()
        .clone();

    let published = lab.json(
        None,
        &[
            "stage",
            "publish",
            stage,
            "--expect-head",
            &revision(&before),
        ],
    );
    assert_eq!(published["state"], "Published", "{published}");
    assert_eq!(published["base"], before["revision"], "{published}");
    assert_eq!(published["published_last"], staged["revision_id"]);

    let head = lab.json(None, &["head"]);
    assert_same_state(&head, &staged_head);
    let snapshot = lab.json(None, &["snapshot"]);
    assert_eq!(snapshot["revision_id"], staged["revision_id"]);
    assert_eq!(snapshot["graph"], staged["graph"]);
    assert_eq!(
        lab.json(None, &["--full-replay", "head"]),
        head,
        "the store replays from its seed to the published head"
    );
    let history = lab.json(None, &["transactions"]);
    for transaction in &committed {
        assert!(
            history.as_array().unwrap().contains(transaction),
            "{transaction} is in the store: {history}"
        );
    }
    assert_eq!(lab.held(stage), 0, "the stage's tenant holds nothing");
    let listed = lab.listed(stage);
    assert_eq!(listed["state"], "Published", "{listed}");
    assert_eq!(listed["published_revisions"].as_array().unwrap().len(), 2);
    lab.refused(Some(stage), &["head"], "stage-already-published", 2);
    lab.refused(
        None,
        &["stage", "abandon", stage],
        "stage-already-published",
        2,
    );
}

/// Every verb of a run works with the stage named only by `EKR_STAGE`: the store verbs read and
/// write the stage, `ekr mint` opens no store and ignores it, and the store itself moves for none
/// of them.
pub fn every_store_verb_joins_the_stage_named_by_ekr_stage(provider: Provider<'_>) {
    let lab = Lab::new(provider);
    let head = lab.json(None, &["head"]);
    let history = lab.json(None, &["transactions"]);
    let ontology = lab.json(None, &["ontology"]);
    let id = stage_of(&lab.json(None, &["stage", "begin"]));
    let stage = Some(id.as_str());

    assert_eq!(
        lab.json(stage, &["ontology"]),
        ontology,
        "the stage at its base"
    );
    assert_same_state(&lab.json(stage, &["head"]), &head);
    let report = lab.json(stage, &["apply-extraction", "extraction.yaml"]);
    assert_eq!(report["rejected"], serde_json::json!([]), "{report}");
    assert_eq!(report["committed"].as_array().unwrap().len(), 3, "{report}");
    let grown = lab.json(stage, &["ontology"]).to_string();
    assert!(grown.contains("\"Project\"") && grown.contains("\"LEADS\""));
    let committed = lab.commit(stage);
    let reads_the_run = lab
        .json(stage, &["snapshot"])
        .to_string()
        .contains("\"Apollo\"");
    assert!(reads_the_run, "the joined snapshot reads the run's nodes");
    let joined = lab.json(stage, &["head"]);
    assert_eq!(
        joined["revision"].as_u64(),
        head["revision"].as_u64().map(|base| base + 4)
    );
    assert_eq!(
        committed["result"]["revision"].as_u64(),
        joined["revision"].as_u64()
    );
    assert!(lab.json(stage, &["quality"]).is_object());
    assert_eq!(
        lab.json(stage, &["transactions"]).as_array().unwrap().len(),
        history.as_array().unwrap().len() + 4
    );
    assert!(lab.json(stage, &["rejections"])["rejections"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(lab.json(stage, &["resolve", "reference.yaml"]).is_object());
    let session = lab
        .command(stage, &["session"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut session = session;
    {
        use std::io::Write as _;
        let mut input = session.stdin.take().unwrap();
        writeln!(input, "{{\"argv\": [\"head\"]}}").unwrap();
    }
    let answered = session.wait_with_output().unwrap();
    assert!(answered.status.success());
    let answer: Value = serde_json::from_slice(&answered.stdout).unwrap();
    assert_eq!(answer["exit"], 0, "{answer}");
    assert_eq!(answer["stdout"]["revision"], joined["revision"], "{answer}");

    assert_eq!(lab.json(None, &["head"]), head, "the store did not move");
    assert_eq!(lab.json(None, &["transactions"]), history);
    assert_eq!(lab.json(None, &["ontology"]), ontology);
    lab.json(None, &["stage", "abandon", &id]);
}

/// `ekr stage publish` with an expected head that is not the store's head, or after the store's
/// head moved since the stage's base, is refused `stage-head-moved` by name, exit 2, and changes
/// nothing: the store's head and history, and the stage, which stays Begun and joinable.
pub fn a_publish_against_a_moved_head_is_refused_by_name_from_the_cli(provider: Provider<'_>) {
    let lab = Lab::new(provider);
    let before = lab.json(None, &["head"]);
    let stage = stage_of(&lab.json(None, &["stage", "begin"]));
    let stage = stage.as_str();
    lab.commit(Some(stage));
    let joined = lab.json(Some(stage), &["head"]);
    let wrong = (before["revision"].as_u64().unwrap() + 5).to_string();
    lab.refused(
        None,
        &["stage", "publish", stage, "--expect-head", &wrong],
        "stage-head-moved",
        2,
    );
    assert_eq!(lab.json(None, &["head"]), before, "nothing was published");
    assert_eq!(lab.listed(stage)["state"], "Begun", "nothing was sealed");
    assert_eq!(lab.json(Some(stage), &["head"]), joined);

    lab.commit(None);
    let moved = lab.json(None, &["head"]);
    let history = lab.json(None, &["transactions"]);
    for expected in [revision(&before), revision(&moved)] {
        lab.refused(
            None,
            &["stage", "publish", stage, "--expect-head", &expected],
            "stage-head-moved",
            2,
        );
        assert_eq!(lab.json(None, &["head"]), moved);
        assert_eq!(lab.json(None, &["transactions"]), history);
        assert_eq!(lab.listed(stage)["state"], "Begun");
    }
    lab.json(None, &["stage", "abandon", stage]);
    assert_eq!(lab.held(stage), 0);
}

/// Design § 107.8, the row "publish, after the append, before the forgetting", from the CLI
/// (spec review B1): a publication whose group was appended and whose process stopped before it
/// forgot the stage's tenant leaves the stage Published and its tenant held. `ekr stage publish`
/// retried with the same expected head runs PublishStage alone, returns the original result,
/// appends nothing and empties the tenant; another expected head is refused
/// `stage-already-published`.
pub fn a_publish_retried_from_the_cli_after_its_append_returns_the_original_result_and_empties_the_tenant(
    provider: Provider<'_>,
) {
    let lab = Lab::new(provider);
    let before = lab.json(None, &["head"]);
    let stage = stage_of(&lab.json(None, &["stage", "begin"]));
    let stage = stage.as_str();
    lab.commit(Some(stage));
    let staged = lab.json(Some(stage), &["snapshot"]);
    let expect = revision(&before);

    let interrupted = {
        let _hook = ekr_kernel::on_stage_point(|point| {
            if point == ekr_kernel::StagePoint::PublishAppended {
                Err(ekr_kernel::PersistenceError::Backend(
                    "interrupted after the append".into(),
                ))
            } else {
                Ok(())
            }
        });
        let mut argv = vec!["ekr".to_owned()];
        argv.extend(lab.configuration());
        argv.extend(["stage", "publish", stage, "--expect-head", &expect].map(str::to_owned));
        ekr::cli::run(argv, &ekr::cli::system_time, &mut std::io::empty())
    };
    let failure = interrupted.expect_err("the first attempt stops after its append");
    assert!(
        failure.to_string().contains("interrupted after the append"),
        "{failure}"
    );
    assert_eq!(lab.listed(stage)["state"], "Published");
    let held = lab.held(stage);
    assert!(held > 0, "the forgetting never ran");
    let appended = lab.json(None, &["head"]);
    let history = lab.json(None, &["transactions"]);
    assert_eq!(appended["revision"], staged["root"]["revision"]);
    lab.refused(Some(stage), &["head"], "stage-already-published", 2);

    let retried = lab.run(None, &["stage", "publish", stage, "--expect-head", &expect]);
    assert_eq!(
        retried.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&retried.stderr)
    );
    let result: Value = serde_json::from_slice(&retried.stdout).unwrap();
    assert_eq!(result["state"], "Published", "{result}");
    assert_eq!(result["published_last"], staged["revision_id"], "{result}");
    assert_eq!(lab.held(stage), 0, "the retry emptied the tenant");
    assert_eq!(lab.json(None, &["head"]), appended, "it appended nothing");
    assert_eq!(lab.json(None, &["transactions"]), history);
    let again = lab.run(None, &["stage", "publish", stage, "--expect-head", &expect]);
    assert_eq!(again.stdout, retried.stdout, "the original result, again");
    let other = (before["revision"].as_u64().unwrap() + 1).to_string();
    lab.refused(
        None,
        &["stage", "publish", stage, "--expect-head", &other],
        "stage-already-published",
        2,
    );
}

/// Unit L's decisions on what joins (design § 107.13): the stage verbs run on the store, ignore
/// `EKR_STAGE` and refuse `--stage` as a usage error; `--stage` and `EKR_STAGE` naming different
/// stages is a usage error, and so is a stage named to `seed` or `migrate`; a malformed id is one.
/// A host whose tenant carries the stage marker is refused `stage-tenant-reserved`, exit 1.
pub fn the_stage_verbs_run_on_the_store_and_a_conflicting_stage_is_a_usage_error(
    provider: Provider<'_>,
) {
    let lab = Lab::new(provider);
    let one = stage_of(&lab.json(None, &["stage", "begin"]));
    let begun = lab.json(Some(&one), &["stage", "begin"]);
    let two = stage_of(&begun);
    assert_eq!(begun["state"], "Begun", "begin ignores EKR_STAGE");
    lab.json(Some(&one), &["stage", "list"]);
    lab.usage(None, &["--stage", &one, "stage", "list"], "--stage");
    lab.usage(Some(&one), &["--stage", &two, "head"], "EKR_STAGE");
    assert_eq!(
        lab.json(Some(&one), &["--stage", &one, "head"]),
        lab.json(Some(&one), &["head"])
    );
    lab.usage(Some(&one), &["seed", "seed.yaml"], "seed");
    let copy = lab.file("copy.db");
    lab.usage(
        Some(&one),
        &["migrate", "--to", copy.to_str().unwrap()],
        "migrate",
    );
    assert!(!copy.exists());
    lab.usage(Some("not-a-stage"), &["head"], "EKR_STAGE");
    lab.json(Some(&two), &["stage", "abandon", &one]);
    lab.json(None, &["stage", "abandon", &two]);
    lab.refused(
        None,
        &["stage", "abandon", &ekr_kernel::StageId::mint().to_string()],
        "stage-not-found",
        2,
    );
    lab.refused(
        Some(&ekr_kernel::StageId::mint().to_string()),
        &["head"],
        "stage-not-found",
        2,
    );

    let mut host: Value =
        serde_json::from_slice(&std::fs::read(lab.file("host.json")).unwrap()).unwrap();
    host["tenant"] = format!("ekr.stage:{}", lab.tenant()).into();
    std::fs::write(lab.file("host.json"), serde_json::to_vec(&host).unwrap()).unwrap();
    for verb in [&["head"][..], &["stage", "list"], &["seed", "seed.yaml"]] {
        lab.refused(None, verb, "stage-tenant-reserved", 1);
    }
}
