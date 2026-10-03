//! Real child-process CLI/SDK observation retention and full-replay reopen on both providers.
use ekr_sdk::binary::EkrBinary;
use ekr_sdk::knowledge::{observation_import, Knowledge};
use ekr_sdk::session::{Backend, Environment, ProcessSession, SessionOptions, StoreConfig};
use ekr_sdk::transport::{Request, Transport};
use std::io::{Read, Write};

fn page(url: &str, method: &str, target: &str) -> String {
    let authority = url.strip_prefix("http://").unwrap().trim_end_matches('/');
    let mut stream = std::net::TcpStream::connect(authority).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(10)))
        .unwrap();
    write!(
        stream,
        "{method} {target} HTTP/1.1\r\nHost: {authority}\r\nConnection: close\r\n\r\n"
    )
    .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    response
}

fn example(format: &str) -> String {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_ekr"))
        .args(["example", format])
        .env_clear()
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn typed_observation_import_retries_and_reads_through_a_real_session() {
    for backend in [Backend::File, Backend::Sqlite] {
        let dir = tempfile::tempdir().unwrap();
        let host = dir.path().join("host.json");
        std::fs::write(&host, example("ekr.cli-host/1")).unwrap();
        let config = StoreConfig {
            host,
            store: dir.path().join("store"),
            backend,
        };
        let binary = EkrBinary::open(env!("CARGO_BIN_EXE_ekr")).unwrap();
        let mut session =
            ProcessSession::start(&binary, config.clone(), SessionOptions::default()).unwrap();
        let seeded = session
            .request(&Request::new(["seed", "-"]).with_stdin(example("ekr-seed/2")))
            .unwrap();
        assert_eq!(seeded.exit, 0, "{seeded:?}");
        let head = session.request(&Request::new(["head"])).unwrap();
        let payload = b"Project Maple health is amber.\r\n<script>fixture()</script>";
        let input = observation_import(
            "manual".into(),
            Some("health-1".into()),
            "1970-01-01T00:00:00.017Z".into(),
            serde_json::from_str("\"FeedItem\"").unwrap(),
            payload,
        );
        let root =
            std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
        let mut schema: serde_json::Value = serde_json::from_slice(
            &std::fs::read(root.join("generated/ekr-contract-data/source.schema.json")).unwrap(),
        )
        .unwrap();
        schema["$ref"] = "#/$defs/ekr.observe.ObservationImport".into();
        let validator = jsonschema::options()
            .should_validate_formats(true)
            .build(&schema)
            .unwrap();
        let mut encoded = serde_json::to_value(&input).unwrap();
        assert!(validator.is_valid(&encoded));
        encoded["observation"]["captured_at"] = "17".into();
        assert!(!validator.is_valid(&encoded));
        let mut knowledge = Knowledge::new(&mut session);
        let first = knowledge.import_observation(&input).unwrap();
        assert!(!first.already_retained);
        assert!(
            knowledge
                .import_observation(&input)
                .unwrap()
                .already_retained
        );
        let id = first.observation_id.0.parse().unwrap();
        assert_eq!(
            knowledge.observations().unwrap(),
            [*input.observation.clone()]
        );
        let shown = knowledge.observation(id).unwrap();
        assert_eq!(ekr_core::bytes::decode(&shown.payload).unwrap(), payload);
        assert!(matches!(
            knowledge.observation(ekr_core::ObservationId::mint()),
            Err(ekr_sdk::read::ReadError::Refused { .. })
        ));
        let document = serde_json::json!({
            "version":{"interpretation_id":ekr_core::NodeId::mint(), "version":1},
            "root_id":ekr_core::GraphRootId::mint(), "observations":[input.observation.observation_id],
            "local_schema":{"node_types":[{"name":"UnmappedProject", "parents":[], "abstract_type":false, "properties":[]}], "edge_types":[]},
            "entities":[{"node_type":"UnmappedProject", "aliases":["Maple"]}], "facts":[], "evidence":[]
        });
        let interpretation: ekr_sdk::contracts::EkrIntegrateInterpretationImport = serde_json::from_value(serde_json::json!({
            "payload":ekr_core::bytes::encode(&serde_json::to_vec_pretty(&document).unwrap()), "document":document,
        })).unwrap();
        schema["$ref"] = "#/$defs/ekr.integrate.InterpretationImport".into();
        assert!(jsonschema::options()
            .should_validate_formats(true)
            .build(&schema)
            .unwrap()
            .is_valid(&serde_json::to_value(&interpretation).unwrap()));
        let imported = knowledge.import_interpretation(&interpretation).unwrap();
        assert!(!imported.already_retained);
        assert_eq!(imported.blockers.len(), 1);
        let parked = knowledge.interpretation(&imported.version).unwrap();
        assert_eq!(parked.document, interpretation.document);
        let questions = knowledge.attention().unwrap();
        assert_eq!(questions.len(), 1);
        assert_eq!(
            questions[0].subject.blocker_id,
            ekr_sdk::contracts::EssPresence::Present(imported.blockers[0].clone())
        );
        assert_eq!(
            questions[0].observations,
            interpretation.document.observations
        );
        assert_eq!(
            knowledge.attention_item(&questions[0].subject).unwrap(),
            questions[0]
        );
        let mut missing = *questions[0].subject.clone();
        missing.blocker_id = ekr_sdk::contracts::EssPresence::Present(Box::new(
            ekr_sdk::contracts::EkrIntegrateIntegrationBlockerId(
                ekr_core::NodeId::mint().to_string(),
            ),
        ));
        assert!(matches!(
            knowledge.attention_item(&missing),
            Err(ekr_sdk::read::ReadError::Refused { .. })
        ));
        assert_eq!(
            knowledge.interpretations().unwrap(),
            [*imported.version.clone()]
        );
        assert_eq!(
            knowledge
                .into_inner()
                .request(&Request::new(["head"]))
                .unwrap(),
            head
        );
        session.close().unwrap();

        let options = SessionOptions {
            environment: Environment::Exact(vec![("EKR_FULL_REPLAY".into(), "true".into())]),
            ..SessionOptions::default()
        };
        let mut reopened = ProcessSession::start(&binary, config.clone(), options).unwrap();
        let mut knowledge = Knowledge::new(&mut reopened);
        assert_eq!(knowledge.interpretation(&imported.version).unwrap(), parked);
        assert_eq!(knowledge.attention().unwrap(), questions);
        let repeated = knowledge.import_interpretation(&interpretation).unwrap();
        assert!(repeated.already_retained);
        assert_eq!(repeated.blockers, imported.blockers);
        assert_eq!(knowledge.observation(id).unwrap(), shown);
        assert!(
            knowledge
                .import_observation(&input)
                .unwrap()
                .already_retained
        );
        assert_eq!(knowledge.observations().unwrap().len(), 1);
        let _ = knowledge.into_inner();
        reopened.close().unwrap();
        let viewer = ekr_sdk::viewer::Viewer::spawn(&binary, &config, 0).unwrap();
        let inbox = page(viewer.url(), "GET", "/inbox");
        assert!(inbox.starts_with("HTTP/1.1 200"), "{inbox}");
        assert!(inbox.contains("Knowledge inbox"));
        assert!(inbox.contains("UnmappedProject"));
        assert!(inbox.contains("Project Maple health is amber."));
        assert!(inbox.contains("&lt;script&gt;fixture()&lt;/script&gt;"));
        assert!(!inbox.contains("<script>"));
        assert!(!inbox.contains("<form"));
        if let Some(directory) = std::env::var_os("EKR_INBOX_CAPTURE_DIR") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                directory.join(format!("{backend:?}.html").to_lowercase()),
                inbox.split_once("\r\n\r\n").unwrap().1,
            )
            .unwrap();
        }
        assert!(page(viewer.url(), "POST", "/inbox").starts_with("HTTP/1.1 405"));
        assert!(page(viewer.url(), "GET", "/inbox?revision=0").starts_with("HTTP/1.1 400"));
        assert!(page(viewer.url(), "GET", "/").contains("href=\"/inbox\""));
        viewer.stop();
    }
}

#[test]
fn observe_is_a_nested_command_and_schema_keeps_its_existing_surface() {
    use clap::Parser;
    use ekr::cli::{
        AttentionCommand, AttentionKind, Cli, Command, IncubateCommand, ObserveCommand,
        UpgradeCommand,
    };
    assert!(matches!(
        Cli::try_parse_from(["ekr", "upgrade", "preview", "-"])
            .unwrap()
            .command,
        Command::Upgrade {
            command: UpgradeCommand::Preview { .. }
        }
    ));
    assert!(matches!(
        Cli::try_parse_from([
            "ekr",
            "attention",
            "show",
            "blocked-integration",
            "00000000-0000-4000-8000-000000000901"
        ])
        .unwrap()
        .command,
        Command::Attention {
            command: AttentionCommand::Show {
                kind: AttentionKind::BlockedIntegration,
                ..
            }
        }
    ));
    assert!(matches!(
        Cli::try_parse_from(["ekr", "incubate", "list"])
            .unwrap()
            .command,
        Command::Incubate {
            command: IncubateCommand::List
        }
    ));
    assert!(matches!(
        Cli::try_parse_from(["ekr", "observe", "list"])
            .unwrap()
            .command,
        Command::Observe {
            command: ObserveCommand::List
        }
    ));
    assert!(matches!(
        Cli::try_parse_from(["ekr", "schema", "ekr-seed/2"])
            .unwrap()
            .command,
        Command::Schema { .. }
    ));
}
