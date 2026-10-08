//! Conformance checks for the `password_file` contract of `ekr.postgres/1`, written by the
//! security review of story `postgres-password-file`.
//!
//! Every password here is assembled at run time around a fresh sentinel, so a sentinel found in a
//! diagnostic can only have come from the password file. The hosted case needs the docker fixture
//! the implementor used: `EKR_TEST_POSTGRES_OWNER` (schema-management configuration) and
//! `EKR_REVIEW_POSTGRES_CONTAINER` (the container name, for role administration through `psql`);
//! `EKR_REQUIRE_POSTGRES=1` makes their absence a failure.

mod postgres_password_file_review {
    use std::{
        io::Write,
        os::fd::{AsRawFd, OwnedFd},
        path::{Path, PathBuf},
        process::{Command, Stdio},
    };

    use ekr_core::TypeId;
    use ekr_store::{postgres::PostgresConfiguration, PostgresStore, StoreError};
    use serde_json::{json, Value};
    use tempfile::TempDir;

    fn sentinel() -> String {
        format!("{}{}", ["rv", "sent", ""].join("-"), TypeId::mint())
    }

    fn document(password: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({ "password": password })).unwrap()
    }

    fn configuration(directory: &Path, connection: &str, extra: Value) -> PathBuf {
        std::fs::write(directory.join("connection.dsn"), connection).unwrap();
        let mut value = json!({
            "format": "ekr.postgres/1",
            "connection_file": "connection.dsn",
            "ca_file": "ca.pem",
            "schema": "public",
            "database_connections": 32,
            "replicas": 2,
            "reserved_connections": 4,
        });
        for (key, field) in extra.as_object().unwrap() {
            value[key] = field.clone();
        }
        let path = directory.join("postgres.json");
        std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        path
    }

    fn shown(error: &StoreError) -> String {
        format!("{error}\n{error:?}")
    }

    /// The refusal `config` reaches through the schema entry point, which builds the provider
    /// configuration before it connects. The configuration's own `Debug` is included, since it is
    /// what a caller would print beside the error.
    fn refusal(config: &PostgresConfiguration) -> String {
        let error = PostgresStore::postgres_schema(config).expect_err("refused before connecting");
        format!("{}\n{config:?}", shown(&error))
    }

    const PASSWORD_PRESENT: &str =
        "postgres-configuration: the connection file carries a password and password_file is set";
    const INVALID_DOCUMENT: &str = "postgres-configuration: invalid password file";
    const UNREADABLE: &str = "postgres-configuration: cannot read referenced file";
    const OVERSIZED: &str = "postgres-configuration: referenced file exceeds its byte limit";
    const NOT_APPLICABLE: &str =
        "postgres-configuration: password_file cannot be applied to this connection file";
    /// The password stage passed; the empty CA file is the next thing refused.
    const PAST_PASSWORD: &str = "postgres-configuration: invalid connection, schema or trust roots";

    /// Passwords carrying every character the two connection-string syntaxes give meaning to, and
    /// parameter assignments that would redirect the connection if they escaped their value.
    fn adversarial(secret: &str) -> Vec<String> {
        let q = '\'';
        let b = '\\';
        vec![
            format!("{secret}{q} host=203.0.113.1 dbname=absent port=1 sslmode=disable {q}"),
            format!("{secret}{b}{q} user=postgres"),
            format!("{secret}{b}"),
            format!("{secret}{b}{b}{q}{b}"),
            format!("{q}{secret}"),
            format!("{secret}&host=203.0.113.1&dbname=absent&sslmode=disable#frag"),
            format!("{secret}%27%20host%3Devil%25%00%"),
            format!("{secret}\n host=evil\r\tdbname=absent\n"),
            format!("{secret} = = == ?&#/@:"),
            format!("  {secret}  "),
            format!("password={q}{secret}{q}"),
            format!("{secret} password={secret}x"),
            format!("{secret}\u{e9}\u{20ac}\u{1f600}\u{2028}\u{a0}"),
        ]
    }

    const FORMS: [&str; 4] = [
        "host=localhost user=ekr_app dbname=ekr sslmode=require",
        "postgresql://ekr_app@localhost:5432/ekr",
        "postgres://ekr_app@localhost/ekr?sslmode=require&application_name=review",
        "postgres://localhost",
    ];

    /// Invariants 1 and 2 without a database: every adversarial password, NUL included, reads back
    /// as exactly itself in both syntaxes, and no part of it is shown.
    #[test]
    fn adversarial_passwords_reach_the_next_stage_in_both_forms_unechoed() {
        let secret = sentinel();
        let directory = TempDir::new().unwrap();
        std::fs::write(directory.path().join("ca.pem"), b"").unwrap();
        let mut passwords = adversarial(&secret);
        passwords.push(format!("{secret}\0 host=evil\0"));
        for password in &passwords {
            std::fs::write(directory.path().join("password.json"), document(password)).unwrap();
            for connection in FORMS {
                let path = configuration(
                    directory.path(),
                    connection,
                    json!({ "password_file": "password.json" }),
                );
                let config = PostgresConfiguration::read(&path).unwrap();
                let text = refusal(&config);
                assert!(
                    text.contains(PAST_PASSWORD),
                    "{connection} / {:?}: {text}",
                    password.replace(&secret, "<sentinel>")
                );
                assert!(!text.contains(&secret), "echoed: {text}");
            }
        }
    }

    /// Invariant 1 when the reparse check is what refuses: connection files that parse alone but
    /// no longer read back the password once it is added.
    #[test]
    fn a_failed_reparse_is_refused_with_a_fixed_category_and_unechoed() {
        let secret = sentinel();
        let directory = TempDir::new().unwrap();
        std::fs::write(directory.path().join("ca.pem"), b"").unwrap();
        let b = '\\';
        let connections = [
            format!("host=localhost user=ekr_app dbname=ekr{b}"),
            "postgres://ek?r_app@localhost/ekr".to_owned(),
            "postgres://ekr_app@localhost/ekr?".to_owned(),
        ];
        for password in adversarial(&secret) {
            std::fs::write(directory.path().join("password.json"), document(&password)).unwrap();
            for connection in &connections {
                let path = configuration(
                    directory.path(),
                    connection,
                    json!({ "password_file": "password.json" }),
                );
                let config = PostgresConfiguration::read(&path).unwrap();
                let text = refusal(&config);
                assert!(text.contains(NOT_APPLICABLE), "{connection}: {text}");
                assert!(!text.contains(&secret), "echoed: {text}");
            }
        }
    }

    /// Invariant 3: every form of a password in the connection file is refused, before the
    /// password file is read, and nothing of either password is shown.
    #[test]
    fn every_form_of_a_connection_password_is_refused() {
        let secret = sentinel();
        let other = sentinel();
        let directory = TempDir::new().unwrap();
        std::fs::write(directory.path().join("ca.pem"), b"").unwrap();
        std::fs::write(directory.path().join("password.json"), document(&other)).unwrap();
        let present = [
            format!("host=localhost user=ekr_app password={secret} dbname=ekr"),
            format!("password = '{secret}' host=localhost"),
            "host=localhost password='' dbname=ekr".to_owned(),
            format!("host=localhost\npassword={secret}\ndbname=ekr\n"),
            format!("postgresql://ekr_app:{secret}@localhost/ekr"),
            "postgresql://ekr_app:@localhost/ekr".to_owned(),
            format!("postgres://ekr_app@localhost/ekr?password={secret}"),
            "postgres://ekr_app@localhost/ekr?sslmode=require&password=".to_owned(),
            format!("postgres://ekr_app@localhost/ekr?pass%77ord={secret}"),
        ];
        for connection in &present {
            let path = configuration(
                directory.path(),
                connection,
                json!({ "password_file": "password.json" }),
            );
            let config = PostgresConfiguration::read(&path).unwrap();
            let text = refusal(&config);
            assert!(text.contains(PASSWORD_PRESENT), "{connection}: {text}");
            assert!(!text.contains(&secret) && !text.contains(&other), "{text}");
        }
        // Forms the connection-string parser does not know are refused as an invalid connection.
        let unknown = [
            format!("PGPASSWORD={secret} host=localhost dbname=ekr"),
            format!("host=localhost PASSWORD={secret}"),
            format!("host=localhost passfile={secret}"),
            format!("postgres://ekr_app@localhost/ekr?PGPASSWORD={secret}"),
        ];
        for connection in &unknown {
            let path = configuration(
                directory.path(),
                connection,
                json!({ "password_file": "password.json" }),
            );
            let config = PostgresConfiguration::read(&path).unwrap();
            let text = refusal(&config);
            assert!(
                text.contains("postgres-configuration: invalid connection file"),
                "{connection}: {text}"
            );
            assert!(!text.contains(&secret) && !text.contains(&other), "{text}");
        }
    }

    fn sealed(bytes: &[u8]) -> OwnedFd {
        use rustix::fs::{fcntl_add_seals, memfd_create, MemfdFlags, SealFlags};
        let fd = memfd_create(
            "ekr-review",
            MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
        )
        .unwrap();
        let mut file = std::fs::File::from(fd);
        file.write_all(bytes).unwrap();
        fcntl_add_seals(
            &file,
            SealFlags::SEAL | SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE,
        )
        .unwrap();
        file.into()
    }

    /// Invariant 4 without a database: the same sealed memfd, named through `/proc/self/fd/N`,
    /// passes the password stage on each of three provider constructions.
    #[test]
    fn a_sealed_memfd_is_read_whole_on_every_construction() {
        let secret = sentinel();
        let fd = sealed(&document(&format!("{secret}' x")));
        let directory = TempDir::new().unwrap();
        std::fs::write(directory.path().join("ca.pem"), b"").unwrap();
        let path = configuration(
            directory.path(),
            FORMS[0],
            json!({ "password_file": format!("/proc/self/fd/{}", fd.as_raw_fd()) }),
        );
        let config = PostgresConfiguration::read(&path).unwrap();
        for attempt in 0..3 {
            let text = refusal(&config);
            assert!(text.contains(PAST_PASSWORD), "attempt {attempt}: {text}");
            assert!(!text.contains(&secret), "{text}");
        }
        drop(fd);
    }

    /// Invariant 5: each failing file is refused with its fixed category, at the byte limit's
    /// exact boundary as well.
    #[test]
    fn failing_password_files_are_refused_with_fixed_categories() {
        use std::os::unix::fs::PermissionsExt;
        let secret = sentinel();
        let directory = TempDir::new().unwrap();
        let root = directory.path();
        std::fs::write(root.join("ca.pem"), b"").unwrap();
        let at_limit = {
            let mut bytes = document(&secret);
            let pad = 65536 - bytes.len();
            bytes.truncate(bytes.len() - 2);
            bytes.extend(std::iter::repeat_n(b'y', pad));
            bytes.extend(b"\"}");
            assert_eq!(bytes.len(), 65536);
            bytes
        };
        let over_limit = [at_limit.clone(), b" ".to_vec()].concat();
        let invalid_utf8 = [
            br#"{"password": ""#.to_vec(),
            secret.as_bytes().to_vec(),
            vec![0xff, 0xfe],
            br#""}"#.to_vec(),
        ]
        .concat();
        let lone_surrogate = format!(r#"{{"password": "{secret}\udc80"}}"#).into_bytes();
        let bom = [vec![0xef, 0xbb, 0xbf], document(&secret)].concat();
        std::fs::write(root.join("unreadable.json"), document(&secret)).unwrap();
        std::fs::set_permissions(
            root.join("unreadable.json"),
            std::fs::Permissions::from_mode(0o000),
        )
        .unwrap();
        std::fs::create_dir(root.join("directory.json")).unwrap();
        let cases: Vec<(&str, Option<Vec<u8>>, &str)> = vec![
            ("at-limit.json", Some(at_limit), PAST_PASSWORD),
            ("over-limit.json", Some(over_limit), OVERSIZED),
            ("absent.json", None, UNREADABLE),
            ("unreadable.json", None, UNREADABLE),
            ("directory.json", None, UNREADABLE),
            ("empty.json", Some(Vec::new()), INVALID_DOCUMENT),
            ("invalid-utf8.json", Some(invalid_utf8), INVALID_DOCUMENT),
            (
                "lone-surrogate.json",
                Some(lone_surrogate),
                INVALID_DOCUMENT,
            ),
            ("bom.json", Some(bom), INVALID_DOCUMENT),
            ("array.json", Some(b"[]".to_vec()), INVALID_DOCUMENT),
            (
                "nested.json",
                Some(serde_json::to_vec(&json!({ "password": { "value": secret } })).unwrap()),
                INVALID_DOCUMENT,
            ),
        ];
        let mut by_category: std::collections::BTreeMap<&str, Vec<String>> = Default::default();
        for (name, bytes, expected) in cases {
            if let Some(bytes) = bytes {
                std::fs::write(root.join(name), bytes).unwrap();
            }
            let path = configuration(root, FORMS[0], json!({ "password_file": name }));
            let config = PostgresConfiguration::read(&path).unwrap();
            let error = PostgresStore::postgres_schema(&config).expect_err("refused");
            let text = shown(&error);
            assert!(text.contains(expected), "{name}: {text}");
            assert!(!text.contains(&secret), "{name}: echoed: {text}");
            by_category
                .entry(expected)
                .or_default()
                .push(error.to_string());
        }
        for (category, texts) in by_category {
            assert!(
                texts.windows(2).all(|pair| pair[0] == pair[1]),
                "{category} is not one fixed text: {texts:?}"
            );
        }
    }

    /// Invariant 6: a document without the field reads as no password file, and the closed
    /// document still refuses a near-miss of the new field's name.
    #[test]
    fn the_field_is_optional_and_the_document_stays_closed() {
        let directory = TempDir::new().unwrap();
        let path = configuration(directory.path(), FORMS[0], json!({}));
        let config = PostgresConfiguration::read(&path).unwrap();
        assert!(config.password_file.is_none());
        let path = configuration(
            directory.path(),
            FORMS[0],
            json!({ "passwordfile": "p.json" }),
        );
        let error = PostgresConfiguration::read(&path).expect_err("unknown field refused");
        assert!(
            error
                .to_string()
                .contains("invalid ekr.postgres/1 document"),
            "{error}"
        );
    }

    /// Invariant 2 against a server: the role's password is set to each adversarial value, and
    /// the store opens in each connection syntax. The server checks the password by SCRAM, so an
    /// open proves the password arrived exactly; an escaped `host`, `port` or `dbname` assignment
    /// would point the connection at an address or database that does not exist.
    #[test]
    fn adversarial_passwords_open_the_hosted_store_as_the_configured_role() {
        let (Some(owner), Some(container)) = (
            std::env::var_os("EKR_TEST_POSTGRES_OWNER"),
            std::env::var("EKR_REVIEW_POSTGRES_CONTAINER").ok(),
        ) else {
            assert_ne!(
                std::env::var("EKR_REQUIRE_POSTGRES").as_deref(),
                Ok("1"),
                "required real PostgreSQL fixture is missing"
            );
            eprintln!("SKIP hosted review case: fixture unset");
            return;
        };
        let owner = PostgresConfiguration::read(Path::new(&owner)).unwrap();
        PostgresStore::postgres_schema(&owner).unwrap();
        let fixture = std::fs::read_to_string(&owner.connection_file).unwrap();
        let parsed: tokio_postgres::Config = fixture.trim().parse().unwrap();
        let port = parsed.get_ports().first().copied().unwrap_or(5432);
        let dbname = parsed.get_dbname().unwrap().to_owned();
        let role = "ekr_review";
        psql(
            &container,
            &dbname,
            &format!(
                "DO $$ BEGIN IF EXISTS (SELECT FROM pg_roles WHERE rolname = '{role}') \
                 THEN EXECUTE 'DROP OWNED BY {role}'; END IF; END $$; \
                 DROP ROLE IF EXISTS {role}; \
                 CREATE ROLE {role} LOGIN CONNECTION LIMIT 8; \
                 GRANT USAGE ON SCHEMA public TO {role}; \
                 GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO {role}; \
                 GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO {role};"
            ),
            None,
        );
        let forms = [
            format!("host=localhost port={port} user={role} dbname={dbname} sslmode=require"),
            format!("postgresql://{role}@localhost:{port}/{dbname}"),
            format!("postgres://{role}@localhost:{port}/{dbname}?application_name=review"),
        ];
        let directory = TempDir::new().unwrap();
        let secret = sentinel();
        for password in adversarial(&secret) {
            psql(
                &container,
                &dbname,
                &format!("ALTER ROLE {role} PASSWORD :'pw';"),
                Some(&password),
            );
            std::fs::write(directory.path().join("password.json"), document(&password)).unwrap();
            for connection in &forms {
                let path = configuration(
                    directory.path(),
                    connection,
                    json!({
                        "ca_file": owner.ca_file,
                        "schema": owner.schema,
                        "password_file": "password.json",
                    }),
                );
                let config = PostgresConfiguration::read(&path).unwrap();
                let tenant = format!("review-{}", TypeId::mint());
                if let Err(error) = PostgresStore::postgres(&config, &tenant, true) {
                    let text = shown(&error);
                    assert!(!text.contains(&secret), "echoed: {text}");
                    panic!(
                        "{connection} / {:?}: {text}",
                        password.replace(&secret, "<sentinel>")
                    );
                }
            }
        }
        psql(
            &container,
            &dbname,
            &format!("DROP OWNED BY {role}; DROP ROLE {role};"),
            None,
        );
    }

    /// Runs `sql` as the container's superuser, with `pw` bound as a psql variable.
    fn psql(container: &str, dbname: &str, sql: &str, pw: Option<&str>) {
        let mut command = Command::new("docker");
        command.args([
            "exec", "-i", container, "psql", "-q", "-U", "postgres", "-d", dbname,
        ]);
        command.args(["-v", "ON_ERROR_STOP=1"]);
        if let Some(pw) = pw {
            command.arg("-v").arg(format!("pw={pw}"));
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(sql.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "psql failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
