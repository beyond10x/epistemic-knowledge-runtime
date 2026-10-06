//! Hosted PostgreSQL configuration. Only file references cross command-line boundaries.
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};

use eventlog_postgres::{PoolOptions, PostgresConfig};
use rustls::pki_types::{pem::PemObject, CertificateDer};
use serde::Deserialize;

use crate::StoreError;

/// Operational provider configuration, read from an `ekr.postgres/1` JSON file.
/// Relative file references resolve beside that configuration file. Credential bytes are never
/// retained in this value or included in diagnostics.
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostgresConfiguration {
    /// Exactly `ekr.postgres/1`.
    pub format: String,
    /// File containing the connection string, including its password unless `password_file`
    /// supplies it.
    pub connection_file: PathBuf,
    /// File containing PEM-encoded trusted CA certificates.
    pub ca_file: PathBuf,
    /// Optional file holding exactly the JSON document `{"password": string}`, whose password
    /// is used for the connection. The connection string must then carry no password. An
    /// absolute path, such as `/proc/self/fd/3` for a document handed over on a descriptor, is
    /// used as written.
    #[serde(default)]
    pub password_file: Option<PathBuf>,
    /// Existing owner schema. Schema creation and role grants remain operator responsibilities.
    pub schema: String,
    /// Total admitted database connection budget.
    pub database_connections: usize,
    /// Number of simultaneously deployed application replicas.
    pub replicas: usize,
    /// Connections reserved for administration and other users.
    pub reserved_connections: usize,
    /// Per-process connection and waiting bounds.
    #[serde(default)]
    pub pool: PostgresPool,
}

/// Bounded provider resources. Every duration is milliseconds, positive and at most one day.
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PostgresPool {
    /// Maximum simultaneous connections.
    pub max_connections: usize,
    /// Maximum callers waiting for a connection.
    pub max_waiters: usize,
    /// Maximum acquisition wait.
    pub acquisition_timeout_ms: u64,
    /// Maximum connection establishment time.
    pub connect_timeout_ms: u64,
    /// Maximum statement execution time.
    pub statement_timeout_ms: u64,
    /// Maximum lock wait.
    pub lock_timeout_ms: u64,
    /// Maximum complete provider operation time.
    pub transaction_timeout_ms: u64,
    /// Maximum pool shutdown time.
    pub shutdown_timeout_ms: u64,
}

impl Default for PostgresPool {
    fn default() -> Self {
        Self {
            max_connections: 4,
            max_waiters: 32,
            acquisition_timeout_ms: 2000,
            connect_timeout_ms: 2000,
            statement_timeout_ms: 5000,
            lock_timeout_ms: 2000,
            transaction_timeout_ms: 10000,
            shutdown_timeout_ms: 5000,
        }
    }
}

fn refusal(detail: &str) -> StoreError {
    StoreError::Backend(format!("postgres-configuration: {detail}"))
}

fn bounded_file(path: &Path, limit: u64) -> Result<Vec<u8>, StoreError> {
    let file = std::fs::File::open(path).map_err(|_| refusal("cannot read referenced file"))?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| refusal("cannot read referenced file"))?;
    if bytes.len() as u64 > limit {
        return Err(refusal("referenced file exceeds its byte limit"));
    }
    Ok(bytes)
}

/// The whole of a password file: the document a saved PostgreSQL connection holds. It has no
/// `Debug`, so no diagnostic can print it.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PasswordDocument {
    password: String,
}

/// `connection` with the password read from `file` added to it, in the connection string's own
/// syntax. A connection that already carries a password is refused, and so is any result the
/// connection-string parser does not read back to exactly that password.
fn with_password(connection: &str, file: &Path) -> Result<String, StoreError> {
    let parsed: tokio_postgres::Config = connection
        .parse()
        .map_err(|_| refusal("invalid connection file"))?;
    if parsed.get_password().is_some() {
        return Err(refusal(
            "the connection file carries a password and password_file is set",
        ));
    }
    let bytes = bounded_file(file, 65536)?;
    let PasswordDocument { password } =
        serde_json::from_slice(&bytes).map_err(|_| refusal("invalid password file"))?;
    let combined =
        if connection.starts_with("postgres://") || connection.starts_with("postgresql://") {
            let separator = if connection.contains('?') { '&' } else { '?' };
            let mut encoded = String::with_capacity(password.len() * 3);
            for byte in password.bytes() {
                if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                    encoded.push(char::from(byte));
                } else {
                    encoded.push_str(&format!("%{byte:02X}"));
                }
            }
            format!("{connection}{separator}password={encoded}")
        } else {
            let quoted = password.replace('\\', "\\\\").replace('\'', "\\'");
            format!("{connection} password='{quoted}'")
        };
    let applied: tokio_postgres::Config = combined
        .parse()
        .map_err(|_| refusal("password_file cannot be applied to this connection file"))?;
    if applied.get_password() != Some(password.as_bytes()) {
        return Err(refusal(
            "password_file cannot be applied to this connection file",
        ));
    }
    Ok(combined)
}

impl PostgresConfiguration {
    /// Reads bounded configuration bytes without echoing any supplied value on failure.
    /// # Errors
    /// Invalid, oversized or unreadable configuration.
    pub fn read(path: &Path) -> Result<Self, StoreError> {
        let bytes = bounded_file(path, 65536)?;
        let mut config: Self = serde_json::from_slice(&bytes)
            .map_err(|_| refusal("invalid ekr.postgres/1 document"))?;
        if config.format != "ekr.postgres/1" {
            return Err(refusal("expected ekr.postgres/1"));
        }
        let directory = path.parent().unwrap_or(Path::new("."));
        config.connection_file = directory.join(&config.connection_file);
        config.ca_file = directory.join(&config.ca_file);
        config.password_file = config.password_file.map(|file| directory.join(file));
        Ok(config)
    }

    pub(crate) fn provider(&self) -> Result<PostgresConfig, StoreError> {
        let bytes = bounded_file(&self.connection_file, 65536)?;
        let connection = std::str::from_utf8(&bytes)
            .map_err(|_| refusal("invalid connection file"))?
            .trim();
        let connection = match &self.password_file {
            None => connection.to_owned(),
            Some(file) => with_password(connection, file)?,
        };
        let certificates = bounded_file(&self.ca_file, 1024 * 1024)?;
        let mut roots = rustls::RootCertStore::empty();
        for certificate in CertificateDer::pem_slice_iter(&certificates) {
            roots
                .add(certificate.map_err(|_| refusal("invalid CA file"))?)
                .map_err(|_| refusal("invalid CA file"))?;
        }
        PostgresConfig::verified(&connection, &self.schema, "ekr", roots)
            .map_err(|_| refusal("invalid connection, schema or trust roots"))
    }

    pub(crate) fn options(&self) -> PoolOptions {
        let p = &self.pool;
        PoolOptions {
            max_connections: p.max_connections,
            max_waiters: p.max_waiters,
            acquisition_timeout: Duration::from_millis(p.acquisition_timeout_ms),
            connect_timeout: Duration::from_millis(p.connect_timeout_ms),
            statement_timeout: Duration::from_millis(p.statement_timeout_ms),
            lock_timeout: Duration::from_millis(p.lock_timeout_ms),
            transaction_timeout: Duration::from_millis(p.transaction_timeout_ms),
            shutdown_timeout: Duration::from_millis(p.shutdown_timeout_ms),
        }
    }
}
