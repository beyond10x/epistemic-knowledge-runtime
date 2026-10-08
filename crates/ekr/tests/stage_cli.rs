//! `story:a-run-is-staged-and-published-whole`, its acceptance through the `ekr stage` verbs and
//! `EKR_STAGE` (`task:stage-cli`, design § 107.10), on SQLite. The same cases run on PostgreSQL in
//! `postgres_cli.rs`, where the PostgreSQL tests run; each is written once, in
//! `support/stage_run.rs`.

#[path = "support/stage_run.rs"]
mod stage_run;

use stage_run::Provider;

#[test]
fn stage_begin_prints_the_minted_id() {
    stage_run::stage_begin_prints_the_minted_id(Provider::Sqlite);
}

#[test]
fn a_failed_run_in_a_stage_leaves_ekr_head_where_it_was() {
    stage_run::a_failed_run_in_a_stage_leaves_ekr_head_where_it_was(Provider::Sqlite);
}

#[test]
fn a_passed_run_is_published_and_ekr_head_is_the_stages_last_revision() {
    stage_run::a_passed_run_is_published_and_ekr_head_is_the_stages_last_revision(Provider::Sqlite);
}

#[test]
fn every_store_verb_joins_the_stage_named_by_ekr_stage() {
    stage_run::every_store_verb_joins_the_stage_named_by_ekr_stage(Provider::Sqlite);
}

#[test]
fn a_publish_against_a_moved_head_is_refused_by_name_from_the_cli() {
    stage_run::a_publish_against_a_moved_head_is_refused_by_name_from_the_cli(Provider::Sqlite);
}

#[test]
fn a_publish_retried_from_the_cli_after_its_append_returns_the_original_result_and_empties_the_tenant(
) {
    stage_run::a_publish_retried_from_the_cli_after_its_append_returns_the_original_result_and_empties_the_tenant(
        Provider::Sqlite,
    );
}

#[test]
fn the_stage_verbs_run_on_the_store_and_a_conflicting_stage_is_a_usage_error() {
    stage_run::the_stage_verbs_run_on_the_store_and_a_conflicting_stage_is_a_usage_error(
        Provider::Sqlite,
    );
}

/// `ekr stage publish` takes the stage as its argument and the expected head as a number, and the
/// global `--stage` takes a stage id and nothing else.
#[test]
fn the_stage_verbs_parse_as_their_clap_definitions_declare() {
    use clap::Parser as _;
    let stage = ekr_kernel::StageId::mint();
    let id = stage.to_string();
    let cli = ekr::cli::Cli::try_parse_from(["ekr", "stage", "publish", &id, "--expect-head", "7"])
        .unwrap();
    let ekr::cli::Command::Stage { command } = cli.command else {
        panic!("`ekr stage publish` is the stage verb");
    };
    let publish: ekr::cli::StageCommand = command;
    assert!(
        matches!(
            publish,
            ekr::cli::StageCommand::Publish { stage_id, expect_head: 7 } if stage_id == stage
        ),
        "{publish:?}"
    );
    let joined = ekr::cli::Cli::try_parse_from(["ekr", "--stage", &id, "head"]).unwrap();
    assert_eq!(joined.stage, Some(stage));
    assert!(ekr::cli::Cli::try_parse_from(["ekr", "--stage", "not-a-stage", "head"]).is_err());
    assert!(ekr::cli::Cli::try_parse_from(["ekr", "stage", "publish", &id]).is_err());
}

/// The File provider admits no stage: every stage verb that writes, and a verb joined to a stage,
/// is refused `stage-unsupported-provider` (exit 2); listing finds none.
#[test]
fn a_stage_is_refused_by_name_on_the_file_provider() {
    let directory = tempfile::tempdir().unwrap();
    let ekr = |args: &[&str]| {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"));
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
            .current_dir(directory.path())
            .args([
                "--host",
                "host.json",
                "--store",
                "store",
                "--backend",
                "file",
            ])
            .args(args)
            .output()
            .unwrap()
    };
    for (name, format) in [("host.json", "ekr.cli-host/1"), ("seed.yaml", "ekr-seed/2")] {
        std::fs::write(
            directory.path().join(name),
            ekr(&["example", format]).stdout,
        )
        .unwrap();
    }
    assert!(ekr(&["seed", "seed.yaml"]).status.success());
    let stage = ekr_kernel::StageId::mint().to_string();
    for args in [
        &["stage", "begin"][..],
        &["stage", "publish", &stage, "--expect-head", "0"],
        &["stage", "abandon", &stage],
        &["--stage", &stage, "head"],
    ] {
        let output = ekr(args);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "{args:?}: {stderr}");
        assert!(
            stderr.starts_with("ekr: stage-unsupported-provider: "),
            "{args:?}: {stderr}"
        );
    }
    let listed = ekr(&["stage", "list"]);
    assert!(listed.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&listed.stdout).unwrap(),
        serde_json::json!([])
    );
}
