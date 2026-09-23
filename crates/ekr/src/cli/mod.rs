//! The clap surface of the `ekr` binary and its thin dispatch to the kernel's `Runtime`.
//!
//! Verbs carry the `ekr.kernel` ESS wire names. Each opens the configured provider through
//! `Runtime::file` or `Runtime::sqlite` under the trusted host document and calls exactly one
//! kernel handler; nothing here applies, validates or persists anything itself.

mod commit;
mod input;
mod propose;
mod seed;
mod validate;

use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use ekr_core::Timestamp;
use ekr_kernel::Runtime;
use serde::Serialize;

use crate::exit::Failure;
use crate::host::CliHostConfigurationV1;

/// The command-line surface of the Epistemic Knowledge Runtime.
#[derive(Debug, Parser)]
#[command(name = "ekr", version, about)]
pub struct Cli {
    /// Trusted host configuration: an `ekr.cli-host/1` JSON document.
    #[arg(long)]
    pub host: PathBuf,
    /// The provider location: a directory for `file`, a database file for `sqlite`.
    #[arg(long)]
    pub store: PathBuf,
    /// The native provider.
    #[arg(long, value_enum)]
    pub backend: Backend,
    /// The command.
    #[command(subcommand)]
    pub command: Command,
}

/// The configured local provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Backend {
    /// The file provider.
    File,
    /// The SQLite provider.
    Sqlite,
}

/// The P1 kernel verbs, by their ESS wire names.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Load the seed (`ekr.kernel.Seed`).
    Seed {
        /// An `ekr-seed/2` YAML document, or `-` for stdin.
        document: PathBuf,
    },
    /// Propose a transaction (`ekr.kernel.Propose`) as the host operator.
    Propose {
        /// An `ekr.transaction-document/1` YAML document, or `-` for stdin.
        document: PathBuf,
    },
    /// Validate a transaction (`ekr.kernel.Validate`) as the host's profile validator.
    Validate {
        /// The retained transaction's id.
        transaction_id: ekr_core::TransactionId,
        /// The committed revision to validate against.
        #[arg(long)]
        against: u64,
    },
    /// Commit a validated transaction (`ekr.kernel.Commit`) as the host operator.
    Commit {
        /// The retained transaction's id.
        transaction_id: ekr_core::TransactionId,
    },
}

/// The system clock in milliseconds since the Unix epoch, for a new decision only.
#[must_use]
pub fn system_time() -> Timestamp {
    let now = std::time::SystemTime::now();
    let millis = match now.duration_since(std::time::UNIX_EPOCH) {
        Ok(after) => i64::try_from(after.as_millis()).unwrap_or(i64::MAX),
        Err(before) => i64::try_from(before.duration().as_millis()).map_or(i64::MIN, |m| -m),
    };
    Timestamp::from_millis(millis)
}

/// Parses `argv` and executes it. The host seam: `now` is the clock, `stdin` is `-`.
///
/// # Errors
///
/// A clap usage refusal, a named kernel refusal or an operational/configuration fault.
pub fn run<I, T>(
    argv: I,
    now: &dyn Fn() -> Timestamp,
    stdin: &mut dyn Read,
) -> Result<String, Failure>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::try_parse_from(argv).map_err(|error| Failure::Usage {
        message: error.render().to_string(),
    })?;
    execute(cli, now, stdin)
}

/// Executes one parsed command and renders its actual result as JSON.
///
/// # Errors
///
/// A named kernel refusal or an operational/configuration fault.
pub fn execute(
    cli: Cli,
    now: &dyn Fn() -> Timestamp,
    stdin: &mut dyn Read,
) -> Result<String, Failure> {
    let (host, store, backend) = (cli.host, cli.store, cli.backend);
    let host = std::fs::read(&host)
        .map_err(|error| Failure::fault(format!("reading host {}: {error}", host.display())))
        .and_then(|bytes| {
            CliHostConfigurationV1::from_json(&bytes)
                .map_err(|error| Failure::fault(format!("host configuration: {error}")))
        })?;
    let open = || -> Result<Runtime, Failure> {
        let CliHostConfigurationV1 {
            tenant,
            context,
            authority,
            ..
        } = host.clone();
        match backend {
            Backend::File => Runtime::file(&store, &tenant, context, authority),
            Backend::Sqlite => Runtime::sqlite(&store, &tenant, context, authority),
        }
        .map_err(|error| Failure::fault(format!("opening the provider: {error}")))
    };
    match cli.command {
        Command::Seed { document } => render(&seed::run(&document, stdin, open, now)?),
        Command::Propose { document } => render(&propose::run(
            &document,
            stdin,
            open,
            host.context.operator,
            now,
        )?),
        Command::Validate {
            transaction_id,
            against,
        } => render(&validate::run(open()?, transaction_id, against, now)?),
        Command::Commit { transaction_id } => render(&commit::run(
            open()?,
            transaction_id,
            host.context.operator,
            now,
        )?),
    }
}

fn render(result: &impl Serialize) -> Result<String, Failure> {
    let mut text = serde_json::to_string_pretty(result).map_err(Failure::fault)?;
    text.push('\n');
    Ok(text)
}
