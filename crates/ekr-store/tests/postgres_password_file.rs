//! An `ekr.postgres/1` configuration takes its database password from a separate `password_file`
//! holding exactly `{"password": string}` — the document a saved connection hands a launched
//! program on a file descriptor — so neither the connection file nor the configuration directory
//! holds a secret.
//!
//! The opening cases need a real hosted PostgreSQL: set `EKR_TEST_POSTGRES_CONFIG` (application
//! role) and `EKR_TEST_POSTGRES_OWNER` (schema-management role) to `ekr.postgres/1` files whose
//! connection files carry their passwords; `EKR_REQUIRE_POSTGRES=1` makes their absence a failure.
//! Every other case runs without a database: each refusal it checks is decided before a
//! connection is attempted. Every password here is assembled at run time.

mod postgres_password_file {
    use std::{
        io::Write,
        os::fd::{AsRawFd, OwnedFd},
        path::{Path, PathBuf},
    };

    use ekr_core::TypeId;
    use ekr_store::{postgres::PostgresConfiguration, PostgresStore, StoreError};
    use serde_json::{json, Value};
    use tempfile::TempDir;

    /// A password that appears nowhere in this source.
    fn sentinel() -> String {
        format!("{}-{}", ["pw", "sentinel"].join("-"), TypeId::mint())
    }

    fn password_document(password: &str) -> Vec<u8> {
        serde_json::to_vec(&json!({ "password": password })).unwrap()
    }

    /// Writes `connection` beside an `ekr.postgres/1` file and returns the file's path. `extra`
    /// fields are merged in last, so they may replace any default.
    fn configuration(directory: &Path, connection: &str, extra: Value) -> PathBuf {
        std::fs::write(directory.join("connection.dsn"), connection).unwrap();
        let mut document = json!({
            "format": "ekr.postgres/1",
            "connection_file": "connection.dsn",
            "ca_file": "ca.pem",
            "schema": "public",
            "database_connections": 32,
            "replicas": 2,
            "reserved_connections": 4,
        });
        for (key, value) in extra.as_object().unwrap() {
            document[key] = value.clone();
        }
        let path = directory.join("postgres.json");
        std::fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
        path
    }

    /// Everything a diagnostic can show of a refusal.
    fn shown(error: &StoreError) -> String {
        format!("{error}\n{error:?}")
    }

    /// The provider-construction refusal for `config`, through the schema-management entry point,
    /// which builds the provider configuration before it connects anywhere.
    fn refusal(config: &PostgresConfiguration) -> String {
        shown(&PostgresStore::postgres_schema(config).expect_err("refused before connecting"))
    }

    const PASSWORD_PRESENT: &str =
        "postgres-configuration: the connection file carries a password and password_file is set";
    const INVALID_DOCUMENT: &str = "postgres-configuration: invalid password file";
    /// The password stage passed: an empty CA file is the next thing refused.
    const PAST_PASSWORD: &str = "postgres-configuration: invalid connection, schema or trust roots";

    #[test]
    fn a_connection_that_carries_a_password_is_refused_with_a_password_file() {
        let secret = sentinel();
        let directory = TempDir::new().unwrap();
        std::fs::write(directory.path().join("ca.pem"), b"").unwrap();
        std::fs::write(
            directory.path().join("password.json"),
            password_document(&sentinel()),
        )
        .unwrap();
        for connection in [
            format!("host=localhost user=ekr_app password={secret} dbname=ekr"),
            format!("host=localhost user=ekr_app password='{secret}' dbname=ekr"),
            format!("postgresql://ekr_app:{secret}@localhost/ekr"),
            format!("postgres://ekr_app@localhost/ekr?sslmode=require&password={secret}"),
        ] {
            let path = configuration(
                directory.path(),
                &connection,
                json!({ "password_file": "password.json" }),
            );
            let config = PostgresConfiguration::read(&path).unwrap();
            let shown = refusal(&config);
            assert!(shown.contains(PASSWORD_PRESENT), "{shown}");
            assert!(!shown.contains(&secret), "the password was echoed: {shown}");
        }
    }

    #[test]
    fn malformed_password_documents_are_refused_without_echoing_the_value() {
        let secret = sentinel();
        let directory = TempDir::new().unwrap();
        std::fs::write(directory.path().join("ca.pem"), b"").unwrap();
        let oversized = {
            let mut document = password_document(&secret);
            document.truncate(document.len() - 2);
            document.extend(std::iter::repeat_n(b'x', 65536));
            document.extend(b"\"}");
            document
        };
        let cases: Vec<(&str, Vec<u8>, &str)> = vec![
            (
                "unknown field",
                serde_json::to_vec(&json!({ "password": secret, "user": "ekr_app" })).unwrap(),
                INVALID_DOCUMENT,
            ),
            (
                "non-string password",
                format!("{{\"password\": [\"{secret}\"]}}").into_bytes(),
                INVALID_DOCUMENT,
            ),
            (
                "numeric password",
                b"{\"password\": 4711}".to_vec(),
                INVALID_DOCUMENT,
            ),
            (
                "null password",
                b"{\"password\": null}".to_vec(),
                INVALID_DOCUMENT,
            ),
            ("missing password", b"{}".to_vec(), INVALID_DOCUMENT),
            (
                "duplicate password",
                format!("{{\"password\": \"{secret}\", \"password\": \"{secret}\"}}").into_bytes(),
                INVALID_DOCUMENT,
            ),
            (
                "a bare string",
                serde_json::to_vec(&json!(secret)).unwrap(),
                INVALID_DOCUMENT,
            ),
            (
                "a second document",
                [password_document(&secret), password_document(&secret)].concat(),
                INVALID_DOCUMENT,
            ),
            (
                "not JSON",
                format!("password={secret}").into_bytes(),
                INVALID_DOCUMENT,
            ),
            (
                "oversized",
                oversized,
                "postgres-configuration: referenced file exceeds its byte limit",
            ),
        ];
        for (case, document, expected) in cases {
            std::fs::write(directory.path().join("password.json"), &document).unwrap();
            let path = configuration(
                directory.path(),
                "host=localhost user=ekr_app dbname=ekr",
                json!({ "password_file": "password.json" }),
            );
            let config = PostgresConfiguration::read(&path).unwrap();
            let shown = refusal(&config);
            assert!(shown.contains(expected), "{case}: {shown}");
            assert!(
                !shown.contains(&secret),
                "{case}: the value was echoed: {shown}"
            );
        }
    }

    #[test]
    fn a_readable_document_passes_the_password_stage_on_both_connection_forms() {
        // Characters that need quoting in a key/value connection string and percent-encoding in
        // a URL, and surrounding whitespace that must not be trimmed away.
        let secret = format!(" {}'\\ &?=%/@:# ", sentinel());
        let directory = TempDir::new().unwrap();
        std::fs::write(directory.path().join("ca.pem"), b"").unwrap();
        std::fs::write(
            directory.path().join("password.json"),
            [password_document(&secret), b"\n".to_vec()].concat(),
        )
        .unwrap();
        for connection in [
            "host=localhost user=ekr_app dbname=ekr sslmode=require\n",
            "postgresql://ekr_app@localhost:5432/ekr",
            "postgres://ekr_app@localhost/ekr?sslmode=require",
        ] {
            let path = configuration(
                directory.path(),
                connection,
                json!({ "password_file": "password.json" }),
            );
            let config = PostgresConfiguration::read(&path).unwrap();
            let shown = refusal(&config);
            assert!(shown.contains(PAST_PASSWORD), "{connection}: {shown}");
            assert!(!shown.contains(secret.trim()), "{connection}: {shown}");
        }
    }

    #[test]
    fn a_relative_password_file_resolves_beside_the_configuration() {
        let directory = TempDir::new().unwrap();
        let nested = directory.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("ca.pem"), b"").unwrap();
        // Only the file beside the configuration is a document the reader can refuse by content;
        // an unresolved relative path is unreadable instead.
        std::fs::write(nested.join("password.json"), b"{}").unwrap();
        let path = configuration(
            &nested,
            "host=localhost user=ekr_app dbname=ekr",
            json!({ "password_file": "password.json" }),
        );
        let config = PostgresConfiguration::read(&path).unwrap();
        let shown = refusal(&config);
        assert!(shown.contains(INVALID_DOCUMENT), "{shown}");
        std::fs::remove_file(nested.join("password.json")).unwrap();
        let shown = refusal(&config);
        assert!(
            shown.contains("postgres-configuration: cannot read referenced file"),
            "{shown}"
        );
    }

    /// Every `ekr.postgres/1` document this repository shows a reader, plus the minimal one,
    /// parses unchanged; one without `password_file` still takes its password from the connection
    /// file, so the refusal it reaches is the next stage's.
    #[test]
    fn configurations_without_the_field_still_parse() {
        let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("../..");
        let page = std::fs::read_to_string(root.join("docs/cli.md")).unwrap();
        let mut documents: Vec<String> = page
            .split("```json\n")
            .skip(1)
            .filter_map(|block| block.split("\n```").next())
            .filter(|body| body.contains("\"ekr.postgres/1\""))
            .map(str::to_owned)
            .collect();
        let documented = documents.len();
        assert!(
            documented >= 1,
            "docs/cli.md shows no ekr.postgres/1 document"
        );
        documents.push(
            json!({
                "format": "ekr.postgres/1",
                "connection_file": "connection.dsn",
                "ca_file": "ca.pem",
                "schema": "public",
                "database_connections": 8,
                "replicas": 1,
                "reserved_connections": 4,
            })
            .to_string(),
        );
        let mut without = 0;
        for document in documents {
            let directory = TempDir::new().unwrap();
            let path = directory.path().join("postgres.json");
            std::fs::write(&path, &document).unwrap();
            PostgresConfiguration::read(&path)
                .unwrap_or_else(|error| panic!("{error}: refused\n{document}"));
            if document.contains("password_file") {
                continue;
            }
            without += 1;
            let value: Value = serde_json::from_str(&document).unwrap();
            let connection = directory
                .path()
                .join(value["connection_file"].as_str().unwrap());
            std::fs::create_dir_all(connection.parent().unwrap()).unwrap();
            std::fs::write(
                &connection,
                format!(
                    "host=localhost user=ekr_app password={} dbname=ekr",
                    sentinel()
                ),
            )
            .unwrap();
            let ca = directory.path().join(value["ca_file"].as_str().unwrap());
            std::fs::create_dir_all(ca.parent().unwrap()).unwrap();
            std::fs::write(&ca, b"").unwrap();
            let config = PostgresConfiguration::read(&path).unwrap();
            let shown = refusal(&config);
            assert!(shown.contains(PAST_PASSWORD), "{shown}\n{document}");
        }
        assert!(without >= 2, "{without} documents without password_file");
    }

    /// The hosted fixture, or `None` when it is not configured and not required.
    fn hosted() -> Option<PostgresConfiguration> {
        let Some(path) = std::env::var_os("EKR_TEST_POSTGRES_CONFIG") else {
            assert_ne!(
                std::env::var("EKR_REQUIRE_POSTGRES").as_deref(),
                Ok("1"),
                "required real PostgreSQL fixture is missing"
            );
            eprintln!("SKIP real PostgreSQL: EKR_TEST_POSTGRES_CONFIG is unset");
            return None;
        };
        static SCHEMA: std::sync::OnceLock<()> = std::sync::OnceLock::new();
        SCHEMA.get_or_init(|| {
            let owner =
                std::env::var_os("EKR_TEST_POSTGRES_OWNER").expect("owner configuration reference");
            PostgresStore::postgres_schema(
                &PostgresConfiguration::read(Path::new(&owner)).unwrap(),
            )
            .unwrap();
        });
        Some(PostgresConfiguration::read(Path::new(&path)).unwrap())
    }

    /// The fixture's connection without its password, and that password.
    fn split_connection(fixture: &PostgresConfiguration) -> (String, String) {
        let text = std::fs::read_to_string(&fixture.connection_file).unwrap();
        let parsed: tokio_postgres::Config = text.trim().parse().unwrap();
        let password = String::from_utf8(parsed.get_password().unwrap().to_vec()).unwrap();
        let host = match &parsed.get_hosts()[0] {
            tokio_postgres::config::Host::Tcp(host) => host.clone(),
            other => panic!("the fixture must use a TCP host: {other:?}"),
        };
        let connection = format!(
            "host={host} port={} user={} dbname={}",
            parsed.get_ports().first().copied().unwrap_or(5432),
            parsed.get_user().unwrap(),
            parsed.get_dbname().unwrap(),
        );
        (connection, password)
    }

    /// An `ekr.postgres/1` file for the fixture's database whose connection file holds no
    /// password and whose `password_file` is `password_file` as written.
    fn password_less(
        directory: &Path,
        fixture: &PostgresConfiguration,
        password_file: &str,
    ) -> PostgresConfiguration {
        let (connection, _) = split_connection(fixture);
        let path = configuration(
            directory,
            &connection,
            json!({
                "ca_file": fixture.ca_file,
                "schema": fixture.schema,
                "database_connections": fixture.database_connections,
                "replicas": fixture.replicas,
                "reserved_connections": fixture.reserved_connections,
                "password_file": password_file,
            }),
        );
        PostgresConfiguration::read(&path).unwrap()
    }

    fn tenant() -> String {
        format!("password-file-{}", TypeId::mint())
    }

    #[test]
    fn a_regular_password_file_opens_a_hosted_store() {
        let Some(fixture) = hosted() else { return };
        let (_, password) = split_connection(&fixture);
        let directory = TempDir::new().unwrap();
        std::fs::write(
            directory.path().join("password.json"),
            password_document(&password),
        )
        .unwrap();
        let config = password_less(directory.path(), &fixture, "password.json");
        let connection = std::fs::read_to_string(directory.path().join("connection.dsn")).unwrap();
        assert!(!connection.contains("password"), "{connection}");
        for reading in [false, true] {
            PostgresStore::postgres(&config, &tenant(), reading)
                .unwrap_or_else(|error| panic!("reading={reading}: {}", shown(&error)));
        }
        // The file's password is the one used: a wrong one is refused by the server, unechoed.
        let wrong = sentinel();
        std::fs::write(
            directory.path().join("password.json"),
            password_document(&wrong),
        )
        .unwrap();
        let error = PostgresStore::postgres(&config, &tenant(), false)
            .err()
            .expect("a wrong password must not open the store");
        assert!(!shown(&error).contains(&wrong), "{}", shown(&error));
    }

    fn sealed(document: &[u8]) -> OwnedFd {
        use rustix::fs::{fcntl_add_seals, memfd_create, MemfdFlags, SealFlags};
        let fd = memfd_create(
            "ekr-password-file",
            MemfdFlags::CLOEXEC | MemfdFlags::ALLOW_SEALING,
        )
        .unwrap();
        let mut file = std::fs::File::from(fd);
        file.write_all(document).unwrap();
        fcntl_add_seals(
            &file,
            SealFlags::SEAL | SealFlags::SHRINK | SealFlags::GROW | SealFlags::WRITE,
        )
        .unwrap();
        file.into()
    }

    #[test]
    fn a_sealed_memfd_named_through_proc_self_fd_opens_a_hosted_store() {
        let Some(fixture) = hosted() else { return };
        let (_, password) = split_connection(&fixture);
        let fd = sealed(&password_document(&password));
        let path = format!("/proc/self/fd/{}", fd.as_raw_fd());
        let directory = TempDir::new().unwrap();
        let config = password_less(directory.path(), &fixture, &path);
        // Opening twice reads the document twice: each open of the descriptor's path starts at
        // its first byte, so a writer and a reader handle both see the whole document.
        for reading in [false, true] {
            PostgresStore::postgres(&config, &tenant(), reading)
                .unwrap_or_else(|error| panic!("reading={reading}: {}", shown(&error)));
        }
        drop(fd);
    }
}
