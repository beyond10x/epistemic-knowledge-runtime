//! The clap surface of the `ekr` binary and its thin dispatch to the kernel's `Runtime`.
//!
//! Verbs carry the `ekr.kernel` ESS wire names. Each store verb opens the configured provider
//! through `Runtime::file` or `Runtime::sqlite` under the trusted host document and calls exactly
//! one kernel handler or read; nothing here applies, validates or persists anything itself. The
//! agent verbs — `guide`, `operations`, `example`, `mint` — print static, tested text or a fresh id
//! and open no provider.

mod agent;
mod commit;
mod explain;
mod head;
mod input;
mod ontology;
mod propose;
mod seed;
mod snapshot;
mod transactions;
mod validate;

use std::ffi::OsString;
use std::io::Read;
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use ekr_core::Timestamp;
use ekr_kernel::Runtime;
use serde::Serialize;

pub use agent::{ExampleFormat, IdKind, OperationKind};
pub use transactions::StateFilter;

use crate::exit::Failure;
use crate::host::CliHostConfigurationV1;

/// Every verb's help ends here, so an agent that reads any one of them finds the rest.
const SEE: &str = "Start with `ekr guide`. Documents: `ekr example ekr.transaction-document/1`, \
`ekr example ekr-seed/2`, `ekr example ekr.cli-host/1`; operation kinds: `ekr operations`.";

/// The command-line surface of the Epistemic Knowledge Runtime.
#[derive(Debug, Parser)]
#[command(
    name = "ekr",
    version,
    about,
    long_about = "The Epistemic Knowledge Runtime. Agents propose transactions; the kernel \
validates and commits them. Run `ekr guide` for the workflow.",
    after_help = SEE
)]
pub struct Cli {
    /// Trusted host configuration: an `ekr.cli-host/1` JSON document (`ekr example ekr.cli-host/1`).
    #[arg(long, env = "EKR_HOST", global = true)]
    pub host: Option<PathBuf>,
    /// The provider location: a directory for `file`, a database file for `sqlite`.
    #[arg(long, env = "EKR_STORE", global = true)]
    pub store: Option<PathBuf>,
    /// The native provider.
    #[arg(long, value_enum, env = "EKR_BACKEND", global = true)]
    pub backend: Option<Backend>,
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

/// The kernel verbs, by their ESS wire names, and the agent verbs that describe them.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Load the seed (`ekr.kernel.Seed`).
    #[command(after_help = SEE)]
    Seed {
        /// An `ekr-seed/2` YAML document, or `-` for stdin (`ekr example ekr-seed/2`).
        document: PathBuf,
    },
    /// Propose a transaction (`ekr.kernel.Propose`) as the host operator.
    #[command(after_help = SEE)]
    Propose {
        /// An `ekr.transaction-document/1` YAML document, or `-` for stdin
        /// (`ekr example ekr.transaction-document/1`, `ekr operations`).
        document: PathBuf,
    },
    /// Validate a transaction (`ekr.kernel.Validate`) as the host's profile validator.
    #[command(after_help = SEE)]
    Validate {
        /// The retained transaction's id, as `ekr propose` printed it.
        transaction_id: ekr_core::TransactionId,
        /// The committed revision to validate against; the head (`ekr head`) when absent.
        #[arg(long)]
        against: Option<u64>,
    },
    /// Commit a validated transaction (`ekr.kernel.Commit`) as the host operator.
    #[command(after_help = SEE)]
    Commit {
        /// The retained transaction's id.
        transaction_id: ekr_core::TransactionId,
    },
    /// Read a snapshot (`ekr.kernel.Snapshot`) of one verified revision.
    #[command(after_help = SEE)]
    Snapshot {
        /// The committed revision to read; the newest when absent.
        #[arg(long)]
        at: Option<u64>,
        /// Select the assertions valid at this instant: decimal milliseconds or `YYYY-MM-DD`.
        #[arg(long, allow_negative_numbers = true, value_parser = crate::host::parse_valid_at)]
        valid_at: Option<Timestamp>,
    },
    /// Explain an assertion (`ekr.kernel.Explain`) at the newest verified revision.
    #[command(after_help = SEE)]
    Explain {
        /// The assertion's id.
        assertion_id: ekr_core::AssertionId,
    },
    /// Print the workflow: roles, propose → validate → commit, exit codes, where ids come from.
    #[command(after_help = SEE)]
    Guide,
    /// List the `ekr.transaction-document/1` operation kinds, or print one kind's fields and
    /// an example operation.
    #[command(after_help = SEE)]
    Operations {
        /// The operation kind, as its YAML tag without `!`.
        kind: Option<OperationKind>,
    },
    /// Print a complete example document of one input format.
    #[command(after_help = SEE)]
    Example {
        /// The format.
        format: ExampleFormat,
    },
    /// Print a fresh id of one kind as JSON.
    #[command(after_help = SEE)]
    Mint {
        /// The id kind.
        kind: IdKind,
    },
    /// Print the head revision number and root as JSON.
    #[command(after_help = SEE)]
    Head,
    /// List retained transactions: id, state and proposer.
    #[command(after_help = SEE)]
    Transactions {
        /// Only transactions in this state.
        #[arg(long, value_enum, ignore_case = true)]
        state: Option<StateFilter>,
    },
    /// Print node types, edge types and properties, by name and id, at the head.
    #[command(after_help = SEE)]
    Ontology,
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
    let cli = match Cli::try_parse_from(argv) {
        Ok(cli) => cli,
        // The binary exits 0 for these through `Cli::parse`; the seam reports them the same way.
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            return Ok(error.render().to_string());
        }
        Err(error) => {
            return Err(Failure::Usage {
                message: error.render().to_string(),
            })
        }
    };
    execute(cli, now, stdin)
}

/// A store verb's missing configuration: a usage error naming the flag and its variable.
fn required<T>(value: Option<T>, flag: &str, var: &str, verb: &str) -> Result<T, Failure> {
    value.ok_or_else(|| Failure::Usage {
        message: format!("ekr: `{verb}` needs {flag} or {var} (see `ekr guide`)"),
    })
}

/// Executes one parsed command and renders its actual result.
///
/// # Errors
///
/// A usage error, a named kernel refusal or an operational/configuration fault.
pub fn execute(
    cli: Cli,
    now: &dyn Fn() -> Timestamp,
    stdin: &mut dyn Read,
) -> Result<String, Failure> {
    let configured = Configured {
        host: cli.host,
        store: cli.store,
        backend: cli.backend,
    };
    match cli.command {
        Command::Guide => Ok(agent::GUIDE.to_owned()),
        Command::Operations { kind: None } => Ok(agent::operation_list()),
        Command::Operations { kind: Some(kind) } => Ok(agent::operation(kind)),
        Command::Example { format } => Ok(agent::example(format).to_owned()),
        Command::Mint { kind } => render(&agent::mint(kind)),
        Command::Seed { document } => {
            let store = configured.resolve("seed")?;
            render(&seed::run(&document, stdin, || store.open(), now)?)
        }
        Command::Propose { document } => {
            let store = configured.resolve("propose")?;
            let operator = store.host.context.operator;
            render(&propose::run(
                &document,
                stdin,
                || store.open(),
                operator,
                now,
            )?)
        }
        Command::Validate {
            transaction_id,
            against,
        } => {
            let runtime = configured.resolve("validate")?.open()?;
            let against = match against {
                Some(against) => against,
                None => head::root(&runtime)?.revision.get(),
            };
            render(&validate::run(runtime, transaction_id, against, now)?)
        }
        Command::Commit { transaction_id } => {
            let store = configured.resolve("commit")?;
            let operator = store.host.context.operator;
            render(&commit::run(store.open()?, transaction_id, operator, now)?)
        }
        Command::Snapshot { at, valid_at } => {
            let runtime = configured.resolve("snapshot")?.open()?;
            render(&snapshot::run(runtime, at, valid_at)?)
        }
        Command::Explain { assertion_id } => {
            let runtime = configured.resolve("explain")?.open()?;
            render(&explain::run(runtime, assertion_id)?)
        }
        Command::Head => render(&head::run(&configured.resolve("head")?.open()?)?),
        Command::Transactions { state } => {
            let runtime = configured.resolve("transactions")?.open()?;
            render(&transactions::run(&runtime, state)?)
        }
        Command::Ontology => render(&ontology::run(&configured.resolve("ontology")?.open()?)?),
    }
}

/// The provider configuration as given, before a store verb needs it.
struct Configured {
    host: Option<PathBuf>,
    store: Option<PathBuf>,
    backend: Option<Backend>,
}

/// A store verb's resolved configuration: the trusted host document, read and checked.
struct Store {
    host: CliHostConfigurationV1,
    store: PathBuf,
    backend: Backend,
}

impl Configured {
    fn resolve(self, verb: &str) -> Result<Store, Failure> {
        let host = required(self.host, "--host", "EKR_HOST", verb)?;
        let store = required(self.store, "--store", "EKR_STORE", verb)?;
        let backend = required(self.backend, "--backend", "EKR_BACKEND", verb)?;
        let host = std::fs::read(&host)
            .map_err(|error| Failure::fault(format!("reading host {}: {error}", host.display())))
            .and_then(|bytes| {
                CliHostConfigurationV1::from_json(&bytes)
                    .map_err(|error| Failure::fault(format!("host configuration: {error}")))
            })?;
        Ok(Store {
            host,
            store,
            backend,
        })
    }
}

impl Store {
    fn open(&self) -> Result<Runtime, Failure> {
        let CliHostConfigurationV1 {
            tenant,
            context,
            authority,
            ..
        } = self.host.clone();
        match self.backend {
            Backend::File => Runtime::file(&self.store, &tenant, context, authority),
            Backend::Sqlite => Runtime::sqlite(&self.store, &tenant, context, authority),
        }
        .map_err(|error| Failure::fault(format!("opening the provider: {error}")))
    }
}

fn render(result: &impl Serialize) -> Result<String, Failure> {
    let mut text = serde_json::to_string_pretty(result).map_err(Failure::fault)?;
    text.push('\n');
    Ok(text)
}
